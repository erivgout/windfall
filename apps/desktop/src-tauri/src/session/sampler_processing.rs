//! Checked sampler preparation, outside document and recording locks.
use super::Session;
use std::sync::atomic::Ordering;
use windfall_project::{Command, DispatchResult};

impl Session {
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
        let (mut document, pool, generation, edits, replacements) = {
            let state = self.state();
            (
                state.document.clone(),
                state.pool.clone(),
                state.generation,
                state.edits,
                state.replacements,
            )
        };
        document
            .dispatch(command.clone(), gesture)
            .map_err(|error| error.to_string())?;
        let current =
            || request.is_none_or(|id| self.inner.sampler_ticket.load(Ordering::Acquire) == id);
        let pool = pool
            .prepare_samplers(
                document.project(),
                &mut || current(),
                &mut |_, done, total| {
                    if current() {
                        self.inner
                            .sampler_done
                            .store(u32::from(done), Ordering::Release);
                        self.inner
                            .sampler_total
                            .store(u32::from(total), Ordering::Release);
                    }
                },
            )
            .map_err(|error| error.to_string())?;
        if !current() {
            return Err("Sampler preparation was cancelled.".into());
        }
        let prepared =
            windfall_engine::Controller::compile_prepared_project(document.project(), &pool);
        #[cfg(test)]
        self.pause("sampler:prepared");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        if !current() {
            return Err("Sampler preparation was cancelled.".into());
        }
        if state.generation != generation
            || state.edits != edits
            || state.replacements != replacements
        {
            return Err(
                "The project changed while sampler audio was being prepared. Try again.".into(),
            );
        }
        if pool.len() != state.pool.len()
            || pool.iter().any(|(id, old)| {
                state
                    .pool
                    .get(id)
                    .is_none_or(|now| !std::ptr::eq(old.samples(), now.samples()))
            })
        {
            return Err(
                "The source changed while sampler audio was being prepared. Try again.".into(),
            );
        }
        let applied = state
            .document
            .dispatch(command, gesture)
            .map_err(|error| error.to_string())?;
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish_prepared(&mut state, &applied.touched, prepared),
        })
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
            if state.edits != edits
                || pool.len() != state.pool.len()
                || pool.iter().any(|(id, old)| {
                    state
                        .pool
                        .get(id)
                        .is_none_or(|now| !std::ptr::eq(old.samples(), now.samples()))
                })
            {
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
