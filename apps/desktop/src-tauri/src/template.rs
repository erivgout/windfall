//! The project a new session starts with.

use windfall_factory::DEFAULT_KIT;
use windfall_project::{Command, Document, Project, SamplePath};

/// Name of a project that has not been saved yet.
pub const DEFAULT_PROJECT_NAME: &str = "Untitled";

/// A new project: empty, plus one channel for each sound of the factory's
/// default kit, each with its own mixer track.
///
/// The channels are added with the same commands the UI sends, and only the
/// resulting project is returned. Wrapping it in a fresh
/// [`Document`] therefore gives a project that starts clean, with the kit
/// outside the undo history.
pub fn default_project() -> Project {
    let mut scratch = Document::new(Project::new(DEFAULT_PROJECT_NAME));
    for (name, path) in DEFAULT_KIT {
        let added = scratch.dispatch(
            Command::AddSample {
                name: name.to_owned(),
                path: SamplePath::Factory(path.to_owned()),
            },
            None,
        );
        let sample = added
            .ok()
            .and_then(|applied| applied.created.first().copied());
        let channel = scratch.dispatch(
            Command::AddChannel {
                name: Some(name.to_owned()),
                sample: sample.map(Into::into),
                index: None,
                mixer_track: None,
            },
            None,
        );
        // Neither command can fail on a new project. If one ever does, a
        // project with fewer channels is still a project.
        if let Err(error) = channel {
            log::error!("could not add \"{name}\" to the new project: {error}");
        }
    }
    scratch.project().clone()
}

#[cfg(test)]
mod tests {
    use windfall_project::{ChannelSource, TrackId};

    use super::*;

    #[test]
    fn the_template_has_one_routed_channel_per_kit_sound() {
        let project = default_project();
        assert_eq!(project.check(), Ok(()));
        assert_eq!(project.settings.name, DEFAULT_PROJECT_NAME);
        assert_eq!(project.channels.len(), DEFAULT_KIT.len());
        assert_eq!(project.samples.len(), DEFAULT_KIT.len());
        // The master and one track per channel.
        assert_eq!(project.mixer.tracks.len(), DEFAULT_KIT.len() + 1);

        for (channel, (name, path)) in project.channels.iter().zip(DEFAULT_KIT) {
            assert_eq!(channel.name, name);
            assert_ne!(channel.mixer_track, TrackId::MASTER);
            assert!(project.mixer.track(channel.mixer_track).is_some());
            let ChannelSource::Sampler(sampler) = &channel.source;
            let sample = sampler.sample.and_then(|id| project.sample(id)).unwrap();
            assert_eq!(sample.path, SamplePath::Factory(path.to_owned()));
        }
        let tracks: Vec<TrackId> = project.channels.iter().map(|c| c.mixer_track).collect();
        let mut distinct = tracks.clone();
        distinct.dedup();
        assert_eq!(tracks, distinct);
    }

    #[test]
    fn the_template_is_not_in_the_undo_history() {
        let mut document = Document::new(default_project());
        assert!(!document.is_dirty());
        assert!(document.history().entries.is_empty());
        assert_eq!(document.undo(), None);
    }
}
