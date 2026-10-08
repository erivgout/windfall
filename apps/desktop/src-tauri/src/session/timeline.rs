//! Checked session-only playback regions. Musical timeline edits use Document.
use super::Session;
use windfall_ipc::TimelinePlaybackState;
use windfall_project::TickRange;

impl Session {
    pub fn timeline_state(&self) -> TimelinePlaybackState {
        let state = self.state();
        TimelinePlaybackState {
            generation: state.generation,
            revision: state.document.revision(),
            region: self.controller().timeline_region(),
            navigation_overflows: self.controller().navigation_overflows(),
        }
    }

    pub fn timeline_region(
        &self,
        region: Option<TickRange>,
        generation: u64,
        revision: u64,
    ) -> Result<TimelinePlaybackState, String> {
        if let Some(range) = region {
            range.check()?;
        }
        let _recording = self.recording_idle()?;
        let state = self.state();
        if state.generation != generation || state.document.revision() != revision {
            return Err("The project changed before the timeline region could be applied. Select the region again.".to_owned());
        }
        self.controller().set_timeline_region(region)?;
        Ok(TimelinePlaybackState {
            generation,
            revision,
            region,
            navigation_overflows: self.controller().navigation_overflows(),
        })
    }
}
