//! Worker preparation of immutable playlist audio; never used by the callback.
use super::Session;
use windfall_ipc::ClipTempoCandidate;
use windfall_project::{Command, DispatchResult, SampleId};
impl Session {
    pub fn prepare_clip_command(&self, command: Command) -> Result<DispatchResult, String> {
        drop(self.recording_idle()?);
        let mut targets = Vec::new();
        clip_targets(&command, &mut targets)?;
        let (mut document, pool, generation, edits) = {
            let state = self.state();
            (
                state.document.clone(),
                state.pool.clone(),
                state.generation,
                state.edits,
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
        let prepared = windfall_engine::Controller::prepare_project(document.project(), &pool);
        #[cfg(test)]
        self.pause("clip:prepared");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        if state.generation != generation || state.edits != edits {
            return Err(
                "The project changed while audio was being prepared. Try again.".to_owned(),
            );
        }
        // A reload can replace the source without editing the document.
        if pool.iter().any(|(id, audio)| {
            state
                .pool
                .get(id)
                .is_none_or(|now| now.samples().as_ptr() != audio.samples().as_ptr())
        }) {
            return Err("The sample changed while audio was being prepared. Try again.".to_owned());
        }
        let applied = state
            .document
            .dispatch(command, None)
            .map_err(|e| e.to_string())?;
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish_prepared(&mut state, &applied.touched, prepared),
        })
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
        loop {
            let (project, pool, generation, edits) = {
                let state = self.state();
                (
                    state.document.project().clone(),
                    state.pool.clone(),
                    state.generation,
                    state.edits,
                )
            };
            let prepared = windfall_engine::Controller::prepare_project(&project, &pool);
            #[cfg(test)]
            self.pause("clip:background-prepared");
            let state = self.state();
            if state.generation != generation
                || state.edits != edits
                || state.pool.iter().any(|(id, audio)| {
                    pool.get(id)
                        .is_none_or(|old| old.samples().as_ptr() != audio.samples().as_ptr())
                })
            {
                continue;
            }
            self.controller()
                .set_prepared_project(state.document.project(), prepared);
            self.sync_transport();
            // Clear under state, so an edit after installation can queue the next job.
            self.inner
                .preparing_clips
                .store(false, std::sync::atomic::Ordering::Release);
            return;
        }
    }
}
