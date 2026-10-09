//! Channel polyphony from event 221. FL's slide scale has no documented
//! millisecond conversion here, so portamento stays off and is reported.
use super::Builder;
use crate::{
    model::Channel,
    report::{Outcome, ReportSection},
};
use windfall_project::{ChannelId, ChannelVoiceSettings, Command};

impl Builder<'_> {
    /// Runs after channel creation. The command replaces the whole voice
    /// block, so tools not described by Polyphony retain their defaults.
    pub(super) fn channel_voice(&mut self, id: ChannelId, channel: &Channel) -> Outcome {
        let Some(polyphony) = channel.polyphony else {
            return Outcome::Exact;
        };
        let section = ReportSection::Channels;
        let mut settings = ChannelVoiceSettings::default();
        let mut outcome = Outcome::Exact;
        settings.polyphony.max_voices = match polyphony.max {
            0 => {
                outcome = Outcome::Approximated;
                self.report.say(
                    section,
                    outcome,
                    "Unlimited FL channel polyphony was brought to Windfall's maximum of 32 voices.",
                );
                32
            }
            1..=32 => polyphony.max as u8,
            max => {
                outcome = Outcome::Approximated;
                self.report.say(
                    section,
                    outcome,
                    format!("FL channel polyphony of {max} voices was clamped to Windfall's maximum of 32."),
                );
                32
            }
        };
        settings.polyphony.mono_legato = polyphony.flags & 1 != 0;
        if polyphony.flags & 2 != 0 {
            outcome = Outcome::Approximated;
            self.report.say(
                section,
                outcome,
                "FL channel portamento was enabled, but its glide time was not decoded: no conversion from Polyphony.slide to milliseconds is documented. Portamento remains at 0 ms.",
            );
        }
        if self
            .apply(
                section,
                "Channel voice settings",
                Command::SetChannelVoiceSettings { id, settings },
            )
            .is_none()
        {
            return Outcome::Dropped;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        convert::{Conversion, ConvertOptions, convert},
        model::{Channel, ChannelKind, FlpProject, Polyphony},
        report::{Outcome, ReportSection},
    };
    use windfall_project::ChannelVoiceSettings;

    fn imported(polyphony: Option<Polyphony>) -> Conversion {
        let source = FlpProject {
            channels: vec![Channel {
                kind: ChannelKind::Sampler,
                polyphony,
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = convert(&source, &ConvertOptions::default());
        assert_eq!(result.project.channels.len(), 1);
        result.project.check().expect("valid imported project");
        result
    }

    fn expected(max_voices: u8, mono_legato: bool) -> ChannelVoiceSettings {
        let mut settings = ChannelVoiceSettings::default();
        settings.polyphony.max_voices = max_voices;
        settings.polyphony.mono_legato = mono_legato;
        settings
    }

    #[test]
    fn mono_with_max_four_sets_channel_voice_settings() {
        let result = imported(Some(Polyphony {
            max: 4,
            flags: 1,
            ..Default::default()
        }));
        assert_eq!(result.project.channels[0].voice, expected(4, true));
        let report = result.report.category(ReportSection::Channels);
        assert_eq!(report.exact, 1);
        assert!(report.lines.is_empty());
    }

    #[test]
    fn unlimited_max_becomes_thirty_two() {
        let result = imported(Some(Polyphony::default()));
        assert_eq!(result.project.channels[0].voice, expected(32, false));
        let report = result.report.category(ReportSection::Channels);
        assert_eq!(report.approximated, 1);
        assert_eq!(report.total(), 1);
        assert!(report.lines.iter().any(|line| {
            line.outcome == Outcome::Approximated && line.text.contains("Unlimited")
        }));
    }

    #[test]
    fn missing_polyphony_keeps_defaults_without_a_report_line() {
        let result = imported(None);
        assert_eq!(
            result.project.channels[0].voice,
            ChannelVoiceSettings::default()
        );
        let report = result.report.category(ReportSection::Channels);
        assert_eq!(report.exact, 1);
        assert!(report.lines.is_empty());
    }

    #[test]
    fn portamento_with_undocumented_slide_reports_missing_glide_time() {
        for flags in [2, 3] {
            let result = imported(Some(Polyphony {
                max: 4,
                slide: 12_345,
                flags,
            }));
            assert_eq!(
                result.project.channels[0].voice,
                expected(4, flags & 1 != 0)
            );
            let report = result.report.category(ReportSection::Channels);
            assert_eq!(report.approximated, 1);
            assert_eq!(report.total(), 1);
            assert_eq!(report.lines.len(), 1);
            let line = &report.lines[0];
            assert_eq!(line.outcome, Outcome::Approximated);
            assert!(line.text.contains("glide time was not decoded"));
            assert!(line.text.contains("Polyphony.slide"));
            assert!(line.text.contains("0 ms"));
        }
    }

    #[test]
    fn bounded_max_copies_through_and_larger_values_clamp() {
        for max in (1..=33).chain([u32::MAX]) {
            let result = imported(Some(Polyphony {
                max,
                ..Default::default()
            }));
            assert_eq!(
                result.project.channels[0].voice,
                expected(max.min(32) as u8, false)
            );
            let report = result.report.category(ReportSection::Channels);
            assert_eq!(report.total(), 1);
            if max <= 32 {
                assert_eq!(report.exact, 1);
                assert!(report.lines.is_empty());
            } else {
                assert_eq!(report.approximated, 1);
                assert!(
                    report
                        .lines
                        .iter()
                        .any(|line| line.text.contains("clamped"))
                );
            }
        }
    }

    #[test]
    fn slide_without_portamento_does_not_enable_glide() {
        let result = imported(Some(Polyphony {
            max: 8,
            slide: u32::MAX,
            flags: 0,
        }));
        assert_eq!(result.project.channels[0].voice, expected(8, false));
        assert!(
            result
                .report
                .category(ReportSection::Channels)
                .lines
                .is_empty()
        );
    }
}
