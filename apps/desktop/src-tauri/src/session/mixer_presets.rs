use super::Session;
use windfall_project::{MixerTrackPreset, TrackId};
use std::io::Read;

impl Session {
    pub fn mixer_preset_capture(&self, id: TrackId, generation: u64, revision: u64) -> Result<MixerTrackPreset, String> {
        let _recording = self.recording_idle()?;
        let (project, plugin_revision) = {
            let state = self.state();
            if state.generation != generation || state.document.revision() != revision {
                return Err("The project changed before preset capture; try again".into());
            }
            (state.document.project().clone(), self.plugin_revision())
        };
        let project = self.capture_plugins_idle(project, plugin_revision)?;
        let state = self.state();
        if state.generation != generation || state.document.revision() != revision {
            return Err("The project changed during preset capture; try again".into());
        }
        MixerTrackPreset::capture(&project, id)
    }

    pub fn mixer_preset_write(&self, path: String, preset: MixerTrackPreset) -> Result<String, String> {
        preset.validate()?;
        let path = crate::paths::absolute(path)?;
        let bytes = serde_json::to_vec_pretty(&preset).map_err(|error| error.to_string())?;
        if bytes.len() > 256 * 1024 * 1024 { return Err("Preset file exceeds 256 MiB".into()); }
        windfall_project::file::write_atomic(&path, &bytes).map_err(|error| format!("Could not save the mixer preset: {error}"))?;
        Ok(crate::paths::display(&path))
    }

    pub fn mixer_preset_read(&self, path: String) -> Result<MixerTrackPreset, String> {
        let file = std::fs::File::open(crate::paths::absolute(path)?).map_err(|error| format!("Could not open the mixer preset: {error}"))?;
        let mut bytes = Vec::new();
        file.take(256 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|error| format!("Could not read the mixer preset: {error}"))?;
        if bytes.len() > 256 * 1024 * 1024 { return Err("Preset file exceeds 256 MiB".into()); }
        let preset: MixerTrackPreset = serde_json::from_slice(&bytes).map_err(|error| format!("Invalid mixer preset: {error}"))?;
        preset.validate()?;
        Ok(preset)
    }
}
