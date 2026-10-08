//! Checked sampler preparation, outside document and recording locks.
use super::{Session, State};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use windfall_core::AudioBuffer;
use windfall_engine::{PreparedProject, SamplePool};
use windfall_project::{Applied, Command, DispatchResult, Document, SampleId};

/// Candidate import/edit snapshot. The caller owns recording/library exclusion
/// when taking this ticket and committing its result; preparation takes no locks.
pub(super) struct SampleEditTicket {
    session: Session,
    command: Command,
    applied: Applied,
    gesture: Option<u64>,
    document: Document,
    original_pool: SamplePool,
    candidate_pool: SamplePool,
    loading: HashSet<SampleId>,
    generation: u64,
    edits: u64,
    replacements: u64,
    request: Option<u64>,
}

pub(super) struct PreparedSampleEdit {
    ticket: SampleEditTicket,
    prepared: PreparedProject,
}

impl SampleEditTicket {
    fn current(&self) -> bool {
        self.request
            .is_none_or(|id| self.session.inner.sampler_ticket.load(Ordering::Acquire) == id)
    }

    /// Slow DSP/compilation; call only after dropping State/library guards.
    /// Does not acquire recording exclusion, including for recording-finish imports.
    pub(super) fn prepare(self) -> Result<PreparedSampleEdit, String> {
        let pool = self
            .candidate_pool
            .prepare_samplers(
                self.document.project(),
                &mut || self.current(),
                &mut |_, done, total| {
                    if self.request.is_some() && self.current() {
                        self.session
                            .inner
                            .sampler_done
                            .store(u32::from(done), Ordering::Release);
                        self.session
                            .inner
                            .sampler_total
                            .store(u32::from(total), Ordering::Release);
                    }
                },
            )
            .map_err(|error| error.to_string())?;
        if !self.current() {
            return Err("Sampler preparation was cancelled.".into());
        }
        let prepared =
            windfall_engine::Controller::compile_prepared_project(self.document.project(), &pool);
        Ok(PreparedSampleEdit {
            ticket: self,
            prepared,
        })
    }
}

impl PreparedSampleEdit {
    /// Final caller-held recording -> library -> State exclusion is required.
    /// Caller must additionally recheck file/root/version/project-directory guards.
    pub(super) fn commit(self, state: &mut State) -> Result<DispatchResult, String> {
        let ticket = self.ticket;
        if !ticket.current() {
            return Err("Sampler preparation was cancelled.".into());
        }
        if state.generation != ticket.generation
            || state.edits != ticket.edits
            || state.replacements != ticket.replacements
        {
            return Err(
                "The project changed while sampler audio was being prepared. Try again.".into(),
            );
        }
        if !state.pool.same_sources(&ticket.original_pool) || state.loading != ticket.loading {
            return Err(
                "The source changed while sampler audio was being prepared. Try again.".into(),
            );
        }
        let applied = if ticket.applied.touched.is_empty() {
            // Preserve even an open gesture: this publication only repairs runtime.
            ticket.applied
        } else {
            state
                .document
                .dispatch(ticket.command, ticket.gesture)
                .map_err(|error| error.to_string())?
        };
        Ok(DispatchResult {
            created: applied.created,
            patch: ticket
                .session
                .publish_prepared(state, &applied.touched, self.prepared),
        })
    }
}

impl Session {
    /// Snapshot a candidate under the caller's existing lock order. Overlays are
    /// private until successful publication and ignored for musical no-ops.
    pub(super) fn sample_edit_ticket(
        &self,
        state: &State,
        command: Command,
        gesture: Option<u64>,
        sources: Vec<(SampleId, AudioBuffer)>,
    ) -> Result<SampleEditTicket, String> {
        let mut document = state.document.clone();
        let applied = document
            .dispatch(command.clone(), gesture)
            .map_err(|error| error.to_string())?;
        let mut candidate_pool = state.pool.clone();
        if !applied.touched.is_empty() {
            for (id, audio) in sources {
                if state.loading.contains(&id) {
                    return Err(
                        "The candidate source is already loading. Wait and try again.".into(),
                    );
                }
                if !document
                    .project()
                    .samples
                    .iter()
                    .any(|asset| asset.id == id)
                {
                    return Err("The candidate source is not registered in the project.".into());
                }
                candidate_pool.insert(id, audio);
            }
            if applied.touched.samples {
                for asset in &document.project().samples {
                    if !candidate_pool.contains(asset.id)
                        && !state.loading.contains(&asset.id)
                        && let Ok(path) = super::samples::locate(
                            asset,
                            state.sample_dir.as_deref(),
                            &self.inner.factory_dir,
                        )
                        && let Some(audio) = self.inner.cache.peek(&path)
                    {
                        candidate_pool.insert(asset.id, audio);
                    }
                }
            }
        }
        Ok(SampleEditTicket {
            session: self.clone(),
            command,
            applied,
            gesture,
            document,
            original_pool: state.pool.clone(),
            candidate_pool,
            loading: state.loading.clone(),
            generation: state.generation,
            edits: state.edits,
            replacements: state.replacements,
            request: None,
        })
    }
    pub fn sampler_preparation_begin(&self) -> Result<u64, String> {
        drop(self.recording_idle()?);
        let _state = self.state();
        let request = self.inner.sampler_ticket.fetch_add(1, Ordering::AcqRel) + 1;
        self.inner.sampler_done.store(0, Ordering::Release);
        self.inner.sampler_total.store(0, Ordering::Release);
        Ok(request)
    }

    pub fn sampler_preparation_cancel(&self, request: u64) {
        // Linearize cancellation with the final check/publication under state.
        let _state = self.state();
        let _ = self.inner.sampler_ticket.compare_exchange(
            request,
            request + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    pub fn sampler_preparation_progress(
        &self,
        request: u64,
    ) -> windfall_ipc::SamplerPreparationProgress {
        windfall_ipc::SamplerPreparationProgress {
            request,
            completed: self.inner.sampler_done.load(Ordering::Acquire),
            total: self.inner.sampler_total.load(Ordering::Acquire),
            current: self.inner.sampler_ticket.load(Ordering::Acquire) == request,
        }
    }

    pub fn prepare_sampler_command(
        &self,
        command: Command,
        request: u64,
    ) -> Result<DispatchResult, String> {
        fn checked(command: &Command) -> bool {
            match command {
                Command::UpdateSampler { .. } => true,
                Command::Batch { commands, .. } => {
                    !commands.is_empty() && commands.iter().all(checked)
                }
                _ => false,
            }
        }
        if !checked(&command) {
            return Err("Sampler preparation accepts only sampler settings edits.".into());
        }
        self.prepare_sampler_edit(command, None, Some(request))
    }

    pub(super) fn prepare_sampler_edit(
        &self,
        command: Command,
        gesture: Option<u64>,
        request: Option<u64>,
    ) -> Result<DispatchResult, String> {
        drop(self.recording_idle()?);
        let mut ticket = {
            let state = self.state();
            self.sample_edit_ticket(&state, command, gesture, Vec::new())?
        };
        ticket.request = request;
        self.finish_sample_edit(ticket)
    }

    pub(super) fn finish_sample_edit(
        &self,
        ticket: SampleEditTicket,
    ) -> Result<DispatchResult, String> {
        let prepared = ticket.prepare()?;
        #[cfg(test)]
        self.pause("sampler:prepared");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        prepared.commit(&mut state)
    }

    /// Source reload/attachment can need a bank without a musical settings edit.
    pub(super) fn queue_sampler_preparation(&self) {
        if self.inner.preparing_samplers.swap(true, Ordering::AcqRel) {
            return;
        }
        let session = self.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("windfall-sampler-preparation".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    session.refresh_sampler_plan()
                }));
                let state = session.state();
                session
                    .inner
                    .preparing_samplers
                    .store(false, Ordering::Release);
                if matches!(result, Ok(true))
                    && state
                        .pool
                        .needs_sampler_preparation(state.document.project())
                {
                    session.queue_sampler_preparation();
                }
                if result.is_err() {
                    log::error!("sampler preparation worker stopped unexpectedly");
                }
            })
        {
            self.inner
                .preparing_samplers
                .store(false, Ordering::Release);
            log::error!("could not start sampler preparation: {error}");
        }
    }

    fn refresh_sampler_plan(&self) -> bool {
        // Continuous editing cannot turn a background worker into an unbounded loop.
        for _ in 0..8 {
            let (project, pool, generation, edits, replacements) = {
                let state = self.state();
                (
                    state.document.project().clone(),
                    state.pool.clone(),
                    state.generation,
                    state.edits,
                    state.replacements,
                )
            };
            let prepared = match windfall_engine::Controller::try_prepare_project(&project, &pool) {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.emit(crate::events::Event::ProjectWarnings(vec![
                        error.to_string(),
                    ]));
                    return false;
                }
            };
            #[cfg(test)]
            self.pause("sampler:background-prepared");
            let _recording = match self.recording_idle() {
                Ok(recording) => recording,
                Err(error) => {
                    self.emit(crate::events::Event::ProjectWarnings(vec![error]));
                    return false;
                }
            };
            let mut state = self.state();
            if state.generation != generation || state.replacements != replacements {
                return false;
            }
            if state.edits != edits || !pool.same_sources(&state.pool) {
                continue;
            }
            state
                .pool
                .install_sampler_preparation(prepared.sampler_pool());
            self.controller()
                .set_prepared_project(state.document.project(), prepared);
            self.sync_transport();
            return true;
        }
        self.emit(crate::events::Event::ProjectWarnings(vec!["Sampler preparation was superseded by repeated edits. Apply the sampler settings again.".into()]));
        false
    }
}
