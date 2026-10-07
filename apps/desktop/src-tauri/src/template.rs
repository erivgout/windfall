//! The project a new session starts with.

use windfall_factory::DEFAULT_KIT;
use windfall_project::{ChannelId, ChannelPatch, Command, Document, Project, SamplePath};

/// Name of a project that has not been saved yet.
pub const DEFAULT_PROJECT_NAME: &str = "Untitled";

/// The channel volume of each sound of [`DEFAULT_KIT`], in its order.
///
/// At the model's default volume the four together overshoot full scale
/// when they land on one step. With these they peak about 6 dB below it on
/// the master, which leaves the user room to build on the beat before
/// anything clips. The kick is the loudest and the hat the quietest.
const KIT_VOLUMES: [f32; DEFAULT_KIT.len()] = [0.36, 0.25, 0.18, 0.29];

/// A new project: empty, plus one channel for each sound of the factory's
/// default kit, each with its own mixer track and a volume from
/// [`KIT_VOLUMES`].
///
/// The channels are added with the same commands the UI sends, and only the
/// resulting project is returned. Wrapping it in a fresh
/// [`Document`] therefore gives a project that starts clean, with the kit
/// outside the undo history.
pub fn default_project() -> Project {
    let mut scratch = Document::new(Project::new(DEFAULT_PROJECT_NAME));
    for ((name, path), volume) in DEFAULT_KIT.into_iter().zip(KIT_VOLUMES) {
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
        let channel = scratch
            .dispatch(
                Command::AddChannel {
                    name: Some(name.to_owned()),
                    sample: sample.map(Into::into),
                    instrument: None,
                    index: None,
                    mixer_track: None,
                },
                None,
            )
            .and_then(|added| match added.created.first() {
                Some(&id) => scratch
                    .dispatch(
                        Command::UpdateChannel {
                            id: ChannelId(id),
                            patch: ChannelPatch {
                                volume: Some(volume),
                                ..ChannelPatch::default()
                            },
                        },
                        None,
                    )
                    .map(drop),
                None => Ok(()),
            });
        // None of the commands can fail on a new project. If one ever
        // does, a project with fewer channels is still a project.
        if let Err(error) = channel {
            log::error!("could not add \"{name}\" to the new project: {error}");
        }
    }
    scratch.project().clone()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use windfall_engine::{RenderOptions, SamplePool, render};
    use windfall_project::{ChannelSource, DEFAULT_CHANNEL_VOLUME, TrackId};

    use super::*;
    use crate::shell::DEV_FACTORY_DIR;

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
            let ChannelSource::Sampler(sampler) = &channel.source else {
                panic!("{name} is not a sampler");
            };
            let sample = sampler.sample.and_then(|id| project.sample(id)).unwrap();
            assert_eq!(sample.path, SamplePath::Factory(path.to_owned()));
        }
        let tracks: Vec<TrackId> = project.channels.iter().map(|c| c.mixer_track).collect();
        let mut distinct = tracks.clone();
        distinct.dedup();
        assert_eq!(tracks, distinct);
    }

    /// The loudest sample on the master when the given channels all play
    /// on the first step, rendered with the real factory sounds.
    fn master_peak(project: &Project, channels: &[usize]) -> f32 {
        let mut pool = SamplePool::new();
        for sample in &project.samples {
            let SamplePath::Factory(relative) = &sample.path else {
                panic!("{sample:?} is not a factory sound");
            };
            let file = Path::new(DEV_FACTORY_DIR).join(relative);
            pool.insert(sample.id, windfall_codec::decode_file(&file).unwrap());
        }
        let mut document = Document::new(project.clone());
        for &channel in channels {
            document
                .dispatch(
                    Command::ToggleStep {
                        pattern: project.patterns[0].id,
                        channel: project.channels[channel].id,
                        step: 0,
                    },
                    None,
                )
                .unwrap();
        }
        let audio = render(
            document.project(),
            &pool,
            &RenderOptions::default(),
            &mut |_| true,
        );
        audio
            .samples()
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    }

    #[test]
    fn the_kit_leaves_headroom_on_the_master_when_all_of_it_hits_at_once() {
        let project = default_project();
        let together = master_peak(&project, &[0, 1, 2, 3]);
        let headroom_db = -20.0 * together.log10();
        assert!(
            (5.5..=6.5).contains(&headroom_db),
            "{headroom_db} dB of headroom"
        );

        // The kick is the loudest and the hat the quietest, both on the
        // faders and in what reaches the master.
        let faders: Vec<f32> = project.channels.iter().map(|c| c.volume).collect();
        let peaks: Vec<f32> = (0..4).map(|c| master_peak(&project, &[c])).collect();
        for levels in [faders, peaks] {
            let [kick, clap, hat, snare] = levels[..] else {
                panic!("{levels:?}");
            };
            assert!(kick > clap.max(snare), "{levels:?}");
            assert!(hat < clap.min(snare), "{levels:?}");
        }

        // A channel the user adds still starts at the model's default.
        let mut document = Document::new(project);
        let added = Command::AddChannel {
            name: None,
            sample: None,
            instrument: None,
            index: None,
            mixer_track: None,
        };
        document.dispatch(added, None).unwrap();
        let channel = document.project().channels.last().unwrap();
        assert_eq!(channel.volume, DEFAULT_CHANNEL_VOLUME);
    }

    #[test]
    fn the_template_is_not_in_the_undo_history() {
        let mut document = Document::new(default_project());
        assert!(!document.is_dirty());
        assert!(document.history().entries.is_empty());
        assert_eq!(document.undo(), None);
    }
}
