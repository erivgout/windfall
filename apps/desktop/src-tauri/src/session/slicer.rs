//! Off-lock analysis and compilation, followed by guarded publication.
use windfall_core::AudioBuffer;
use windfall_ipc::{SliceOptions, SliceReview};
use windfall_project::{Clip, ClipId, DispatchResult, slicer};

use super::{Session, State};

#[derive(Clone)]
pub(super) struct Prepared {
    review: SliceReview,
    original: Clip,
    source: AudioBuffer,
    generation: u64,
    edits: u64,
}
impl Prepared {
    fn check(&self, state: &State) -> Result<(), String> {
        if state.generation != self.generation
            || state.edits != self.edits
            || state.slice_ticket != self.review.token
            || self
                .original
                .content
                .sample()
                .and_then(|id| state.pool.get(id))
                .is_none_or(|now| now.samples().as_ptr() != self.source.samples().as_ptr())
        {
            return Err("The project or source changed. Analyze the clip again.".into());
        }
        Ok(())
    }
}
impl Session {
    pub fn slice_analyze(
        &self,
        clip: ClipId,
        options: SliceOptions,
    ) -> Result<SliceReview, String> {
        let (original, source, tempo, swing, generation, edits, token) = {
            let _recording = self.recording_idle()?;
            let mut state = self.state();
            let project = state.document.project();
            let original = project
                .playlist
                .clips
                .iter()
                .find(|c| c.id == clip)
                .cloned()
                .ok_or("The selected clip no longer exists.")?;
            slicer::supported(
                &original,
                project.settings.tempo_bpm,
                project.settings.swing,
            )?;
            let source = original
                .content
                .sample()
                .and_then(|sample| state.pool.get(sample))
                .cloned()
                .ok_or("The clip's source audio is not loaded. Reload samples and try again.")?;
            let (tempo, swing) = (project.settings.tempo_bpm, project.settings.swing);
            state.slice_ticket = state
                .slice_ticket
                .checked_add(1)
                .ok_or("Slice review token limit reached.")?;
            state.slice_review = None;
            (
                original,
                source,
                tempo,
                swing,
                state.generation,
                state.edits,
                state.slice_ticket,
            )
        };
        let analysis = slicer::analyze(&source, &original, tempo, swing, options)?;
        #[cfg(test)]
        self.pause("slice:analyzed");
        let prepared = Prepared {
            review: SliceReview {
                token,
                clip,
                length_ticks: original.length,
                analysis,
            },
            original,
            source,
            generation,
            edits,
        };
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        prepared.check(&state)?;
        let review = prepared.review.clone();
        state.slice_review = Some(prepared);
        Ok(review)
    }

    pub fn slice_apply(&self, token: u32, markers: Vec<u32>) -> Result<DispatchResult, String> {
        let (review, mut document, pool) = {
            let _recording = self.recording_idle()?;
            let state = self.state();
            let review = state
                .slice_review
                .clone()
                .filter(|r| r.review.token == token)
                .ok_or("This slice review expired. Analyze the clip again.")?;
            review.check(&state)?;
            (review, state.document.clone(), state.pool.clone())
        };
        if markers.iter().any(|tick| {
            !review
                .review
                .analysis
                .markers
                .iter()
                .any(|m| m.tick == *tick)
        }) {
            return Err("Apply accepts only markers from the current slice review.".into());
        }
        let settings = &document.project().settings;
        let command = slicer::split_command(
            &review.original,
            &markers,
            settings.tempo_bpm,
            settings.swing,
        )?;
        document
            .dispatch(command.clone(), None)
            .map_err(|e| e.to_string())?;
        let prepared = windfall_engine::Controller::prepare_project(document.project(), &pool);
        #[cfg(test)]
        self.pause("slice:prepared");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        review.check(&state)?;
        if state
            .slice_review
            .as_ref()
            .is_none_or(|r| r.review.token != token)
        {
            return Err("This slice review expired. Analyze the clip again.".into());
        }
        // Other source reloads can invalidate the compiled playback plan too.
        if state.pool.iter().any(|(id, now)| {
            pool.get(id)
                .is_none_or(|old| old.samples().as_ptr() != now.samples().as_ptr())
        }) || pool.iter().any(|(id, old)| {
            state
                .pool
                .get(id)
                .is_none_or(|now| old.samples().as_ptr() != now.samples().as_ptr())
        }) {
            return Err("A source changed while slices were being prepared. Analyze again.".into());
        }
        let applied = state
            .document
            .dispatch(command, None)
            .map_err(|e| e.to_string())?;
        state.slice_review = None;
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish_prepared(&mut state, &applied.touched, prepared),
        })
    }

    pub fn slice_discard(&self, token: u32) {
        let mut state = self.state();
        if state.slice_ticket == token {
            state.slice_review = None;
        }
    }
}
