//! Channel conversion uses the documented sampler fields; other generators
//! retain their notes through silent samplers and keep their state separately.
use super::{
    AudioSource, Builder, ChannelRole, PluginPlace, PluginPlaceholder, chosen_color, name_or, synth,
};
use crate::{
    model::ChannelKind,
    paths::resolve,
    report::{Outcome, ReportSection},
    units,
};
use windfall_project::{
    ChannelId, ChannelPatch, Command, Envelope, SampleId, SamplePath, SamplerPatch,
};

impl Builder<'_> {
    pub(super) fn create(
        &mut self,
        section: ReportSection,
        what: &str,
        command: Command,
    ) -> Option<u32> {
        self.apply(section, what, command)?.first().copied()
    }

    fn sample(&mut self, _name: &str, path: Option<&str>) -> Option<SampleId> {
        let path = path.filter(|p| !p.trim().is_empty())?;
        let resolved = resolve(path, self.folders());
        let outcome = if resolved.resolved {
            Outcome::Exact
        } else {
            Outcome::Approximated
        };
        if !resolved.resolved {
            self.report.say(ReportSection::Samples, outcome, format!("The sample path \"{path}\" needs a folder supplied by the user; it was kept as written."));
        }
        let id = self.create(
            ReportSection::Samples,
            "A sample reference",
            Command::AddSample {
                name: crate::paths::file_stem(path).to_owned(),
                path: SamplePath::External(resolved.path),
            },
        );
        self.report.count(
            ReportSection::Samples,
            if id.is_some() {
                outcome
            } else {
                Outcome::Dropped
            },
            1,
        );
        id.map(SampleId)
    }

    pub(super) fn channels(&mut self) {
        for channel in &self.flp.channels {
            let name = name_or(channel.display_name(), || {
                format!("Channel {}", channel.iid)
            });
            let mixer_track = self.track_of(channel.insert);
            let volume = units::channel_gain(channel.volume.unwrap_or(10_240));
            let pan = units::channel_pan(channel.pan.unwrap_or(6_400));
            let pitch = channel.pitch.unwrap_or(0) as f32 / 100.0;
            match channel.kind {
                ChannelKind::Layer => {
                    self.channels.insert(
                        channel.iid,
                        ChannelRole::Layer(channel.layer_children.clone()),
                    );
                    self.report
                        .count(ReportSection::Channels, Outcome::Approximated, 1);
                    self.report.say(ReportSection::Channels, Outcome::Approximated, "Layer notes are copied to their children; layer controls have no equivalent.");
                    continue;
                }
                ChannelKind::AutomationClip => {
                    self.channels.insert(channel.iid, ChannelRole::Automation);
                    continue;
                }
                ChannelKind::AudioClip => {
                    let sample = self.sample(&name, channel.sample_path.as_deref());
                    let stretched = channel.params.is_some_and(|p| {
                        p.stretch_time.unwrap_or(0) != 0 || p.stretch_multiplier.unwrap_or(0) != 0
                    });
                    self.channels.insert(
                        channel.iid,
                        ChannelRole::Audio(AudioSource {
                            name,
                            sample,
                            mixer_track,
                            gain: volume,
                            pan,
                            reverse: channel.reversed(),
                            pitch,
                            stretched,
                        }),
                    );
                    self.report
                        .count(ReportSection::Channels, Outcome::Exact, 1);
                    continue;
                }
                _ => {}
            }
            let translated = synth::translate(channel);
            let sampler = channel.kind == ChannelKind::Sampler && channel.plugin.is_none();
            let sample = if sampler {
                self.sample(&name, channel.sample_path.as_deref())
            } else {
                None
            };
            let instrument = translated.as_ref().map(|t| t.params.kind());
            let Some(id) = self.create(
                ReportSection::Channels,
                "A channel",
                Command::AddChannel {
                    name: Some(name.clone()),
                    sample,
                    instrument,
                    index: None,
                    mixer_track: Some(mixer_track),
                },
            ) else {
                continue;
            };
            let id = ChannelId(id);
            self.channels.insert(channel.iid, ChannelRole::Plays(id));
            self.apply(
                ReportSection::Channels,
                "Channel controls",
                Command::UpdateChannel {
                    id,
                    patch: ChannelPatch {
                        color: chosen_color(channel.color),
                        volume: Some(volume),
                        pan: Some(pan),
                        muted: Some(!channel.enabled.unwrap_or(true)),
                        ..Default::default()
                    },
                },
            );
            let mut outcome = Outcome::Exact;
            if let Some(translated) = translated {
                self.apply(
                    ReportSection::Channels,
                    "Instrument settings",
                    Command::SetInstrumentParams {
                        channel: id,
                        params: translated.params,
                    },
                );
                outcome = Outcome::Approximated;
                self.report.say(
                    ReportSection::Channels,
                    outcome,
                    format!(
                        "\"{name}\" became Windfall's subtractive synth: {}.",
                        translated.notes.join("; ")
                    ),
                );
            } else if sampler {
                let start = channel
                    .params
                    .and_then(|p| p.sample_start)
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.0)
                    .clamp(0.0, 0.999999) as f32;
                let length = channel
                    .params
                    .and_then(|p| p.sample_length)
                    .filter(|v| v.is_finite())
                    .unwrap_or(1.0);
                let end =
                    (f64::from(start) + length).clamp(f64::from(start) + 0.0000001, 1.0) as f32;
                let cut = channel.cut.unwrap_or_default();
                self.apply(
                    ReportSection::Channels,
                    "Sampler settings",
                    Command::UpdateSampler {
                        id,
                        patch: SamplerPatch {
                            root_key: Some(channel.root_note.unwrap_or(60).min(127) as u8),
                            tune: Some(pitch),
                            gain: Some(f32::from(channel.preamp.unwrap_or(256)) / 256.0),
                            start: Some(start),
                            end: Some(end),
                            reverse: Some(channel.reversed()),
                            cut_self: Some(cut.cuts != 0 && cut.cuts == cut.cut_by),
                            cut_group: Some(if cut.cuts == cut.cut_by {
                                cut.cuts.min(255) as u8
                            } else {
                                0
                            }),
                        },
                    },
                );
                if cut.cuts != cut.cut_by || cut.cuts > 255 {
                    outcome = Outcome::Approximated;
                    self.report.say(
                        ReportSection::Channels,
                        outcome,
                        "Asymmetric or large cut groups cannot be represented exactly.",
                    );
                }
                if let Some(env) = channel.volume_envelope().filter(|e| e.enabled) {
                    outcome = Outcome::Approximated;
                    self.apply(
                        ReportSection::Channels,
                        "Sampler envelope",
                        Command::SetSamplerEnvelope {
                            id,
                            envelope: Some(Envelope {
                                attack_ms: units::envelope_ms(env.attack),
                                decay_ms: units::envelope_ms(env.decay),
                                sustain: env.sustain.min(128) as f32 / 128.0,
                                release_ms: units::envelope_ms(env.release),
                            }),
                        },
                    );
                    self.report.say(ReportSection::Channels, outcome, "The sampler envelope uses fitted times; delay, hold, tempo sync and LFOs have no equivalent.");
                }
                if channel
                    .volume
                    .is_some_and(|v| !units::channel_gain_is_exact(v))
                {
                    outcome = Outcome::Approximated;
                    self.report.say(
                        ReportSection::Channels,
                        outcome,
                        "Channel volume uses DawVert's fitted gain curve.",
                    );
                }
            } else {
                outcome = Outcome::Placeholder;
                self.report.say(ReportSection::Channels, outcome, format!("\"{name}\" is a silent sampler placeholder; its notes and arrangement remain."));
                if let Some(plugin) = &channel.plugin {
                    let hosted = plugin.hosted();
                    self.plugins.push(PluginPlaceholder {
                        place: PluginPlace::Channel { channel: id },
                        internal_name: plugin.internal_name.clone(),
                        name: hosted.as_ref().and_then(|p| p.name.clone()),
                        vendor: hosted.as_ref().and_then(|p| p.vendor.clone()),
                        format: hosted.as_ref().map(|p| p.format),
                        path: hosted.as_ref().and_then(|p| p.path.clone()),
                        state: plugin.state.clone(),
                    });
                }
            }
            self.report.count(ReportSection::Channels, outcome, 1);
        }
    }
}
