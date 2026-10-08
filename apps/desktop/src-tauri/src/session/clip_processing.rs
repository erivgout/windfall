//! Worker preparation of immutable playlist audio; never used by the callback.
use super::Session;
use windfall_ipc::ClipTempoCandidate;
use windfall_project::{Command, DispatchResult, SampleId};
impl Session {
    pub fn prepare_clip_command(&self, command: Command) -> Result<DispatchResult, String> {
        drop(self.recording_idle()?);
        let mut targets = Vec::new();
        clip_targets(&command, &mut targets)?;
        let (mut document, pool, generation, edits, replacements, loading, preparation) = {
            let state = self.state();
            (
                state.document.clone(),
                state.pool.clone(),
                state.generation,
                state.edits,
                state.replacements,
                state.loading.clone(),
                self.project_preparation(&state),
            )
        };
        document
            .dispatch(command.clone(), None)
            .map_err(|e| e.to_string())?;
        for clip in document
            .project()
            .playlist
            .clips
            .iter()
            .filter(|clip| targets.contains(&clip.id))
        {
            if let windfall_project::ClipContent::Audio {
                sample,
                stretch,
                pitch,
                ..
            } = clip.content
            {
                pool.clip_audio(sample, stretch, pitch).ok_or_else(|| {
                    "The clip's audio is not loaded. Reload samples and try again.".to_owned()
                })?;
            }
        }
        let mut prepared = preparation
            .prepare(
                document.project(),
                &pool,
                windfall_engine::ProjectPublicationIntent::Edit,
            )
            .map_err(|error| error.to_string())?;
        let retirement;
        #[cfg(test)]
        self.pause("clip:prepared");
        let result = {
            let _recording = self.recording_idle()?;
            let mut state = self.state();
            if state.generation != generation
                || state.edits != edits
                || state.replacements != replacements
            {
                return Err(
                    "The project changed while audio was being prepared. Try again.".to_owned(),
                );
            }
            // A reload can replace the source without editing the document.
            if !pool.same_sources(&state.pool) || state.loading != loading {
                return Err(
                    "The sample changed while audio was being prepared. Try again.".to_owned(),
                );
            }
            let pool = prepared.pool().clone();
            let lease = prepared.publication(self, &state)?;
            let applied = state
                .document
                .dispatch(command, None)
                .map_err(|e| e.to_string())?;
            self.commit_prepared_parameters(&state, &applied.touched);
            retirement = lease.install();
            Ok(DispatchResult {
                created: applied.created,
                patch: self.publish_prepared(&mut state, &applied.touched, &pool),
            })
        };
        let mut retirement = retirement;
        self.retire_project(&mut retirement);
        result
    }
    pub fn detect_clip_tempo(&self, sample: SampleId) -> Result<Vec<ClipTempoCandidate>, String> {
        let audio = self
            .state()
            .pool
            .get(sample)
            .cloned()
            .ok_or_else(|| "The clip's audio is not loaded.".to_owned())?;
        Ok(windfall_stretch::estimate_tempo(&audio)
            .unwrap_or_default()
            .into_iter()
            .map(|c| ClipTempoCandidate {
                bpm: c.bpm,
                confidence: c.confidence,
            })
            .collect())
    }
}

fn clip_targets(command: &Command, ids: &mut Vec<windfall_project::ClipId>) -> Result<(), String> {
    match command {
        Command::UpdateAudioClips { updates } => ids.extend(updates.iter().map(|u| u.id)),
        Command::UpdateClips { updates } => ids.extend(updates.iter().map(|u| u.id)),
        Command::Batch { commands, .. } => {
            for command in commands {
                clip_targets(command, ids)?;
            }
        }
        _ => return Err("Audio preparation accepts only playlist clip edits.".to_owned()),
    }
    Ok(())
}

impl Session {
    /// Called with state held: coalesce cache-miss publications onto one worker.
    pub(super) fn queue_clip_preparation(&self) {
        use std::sync::atomic::Ordering;
        if self.inner.preparing_clips.swap(true, Ordering::AcqRel) {
            return;
        }
        let session = self.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("windfall-clip-preparation".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session.refresh_clip_plan()
                }));
                if result.is_err() {
                    let _state = session.state();
                    session
                        .inner
                        .preparing_clips
                        .store(false, Ordering::Release);
                    log::error!("clip preparation worker stopped unexpectedly");
                }
            })
        {
            self.inner.preparing_clips.store(false, Ordering::Release);
            log::error!("could not start clip preparation: {error}");
        }
    }
    fn refresh_clip_plan(&self) {
        for _ in 0..8 {
            let (project, pool, generation, edits, replacements, preparation) = {
                let state = self.state();
                (
                    state.document.project().clone(),
                    state.pool.clone(),
                    state.generation,
                    state.edits,
                    state.replacements,
                    self.project_preparation(&state),
                )
            };
            if pool.needs_sampler_preparation(&project) {
                // A source reload belongs to the sampler's recording/request
                // guarded worker, even when a clip worker was already running.
                let state = self.state();
                self.inner
                    .preparing_clips
                    .store(false, std::sync::atomic::Ordering::Release);
                self.push_project(&state);
                return;
            }
            let mut prepared = match preparation.prepare(
                &project,
                &pool,
                windfall_engine::ProjectPublicationIntent::Edit,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.emit(crate::events::Event::ProjectWarnings(vec![
                        error.to_string(),
                    ]));
                    self.inner
                        .preparing_clips
                        .store(false, std::sync::atomic::Ordering::Release);
                    return;
                }
            };
            #[cfg(test)]
            self.pause("clip:background-prepared");
            let retirement;
            {
                let _recording = match self.recording_idle() {
                    Ok(guard) => guard,
                    Err(error) => {
                        self.emit(crate::events::Event::ProjectWarnings(vec![
                            error.to_string(),
                        ]));
                        break;
                    }
                };
                let mut state = self.state();
                if state.generation != generation
                    || state.edits != edits
                    || state.replacements != replacements
                    || !state.pool.same_sources(&pool)
                {
                    continue;
                }
                let pool = prepared.pool().clone();
                let lease = match prepared.publication(self, &state) {
                    Ok(lease) => lease,
                    Err(error) => {
                        self.emit(crate::events::Event::ProjectWarnings(vec![
                            error.to_string(),
                        ]));
                        break;
                    }
                };
                retirement = lease.install();
                state.pool.install_sampler_preparation(&pool);
                self.sync_transport();
                // Clear under state, so an edit after installation can queue the next job.
                self.inner
                    .preparing_clips
                    .store(false, std::sync::atomic::Ordering::Release);
            }
            let mut retirement = retirement;
            self.retire_project(&mut retirement);
            return;
        }
        self.inner
            .preparing_clips
            .store(false, std::sync::atomic::Ordering::Release);
        self.emit(crate::events::Event::ProjectWarnings(vec![
            "Project audio preparation was refused or superseded. Reload samples to retry.".into(),
        ]));
    }
}
