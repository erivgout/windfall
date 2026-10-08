//! Only the selected arrangement's documented named/meter markers cross over.
use super::Builder;
use crate::report::{Outcome, ReportSection};
use windfall_project::{Command, MarkerKind, TimeSignature};

impl Builder<'_> {
    pub(super) fn timeline(&mut self) {
        let section = ReportSection::Other;
        let markers = self
            .flp
            .main_arrangement()
            .map(|a| a.markers.clone())
            .unwrap_or_default();
        for marker in markers {
            let scaled = crate::units::rescale(marker.position, self.ppq);
            let Ok(tick) = u32::try_from(scaled.ticks) else {
                self.report.say(
                    section,
                    Outcome::Dropped,
                    "An arrangement marker exceeds the song tick range.",
                );
                continue;
            };
            let command = match marker.kind {
                8 => {
                    let (Some(numerator), Some(denominator)) =
                        (marker.numerator, marker.denominator)
                    else {
                        self.report.say(
                            section,
                            Outcome::Dropped,
                            "An arrangement meter marker has no complete signature.",
                        );
                        continue;
                    };
                    Command::AddMeterChange {
                        tick,
                        signature: TimeSignature {
                            numerator,
                            denominator,
                        },
                    }
                }
                0 => Command::AddTimelineMarker {
                    tick,
                    name: marker
                        .name
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| "Marker".into()),
                    kind: MarkerKind::Named,
                },
                kind => {
                    self.report.say(section, Outcome::Dropped, format!("Arrangement marker kind {kind} is not mapped: its navigation semantics are not supported by this importer."));
                    continue;
                }
            };
            if self
                .apply(section, "An arrangement timeline marker", command)
                .is_some()
            {
                self.report.count(
                    section,
                    if scaled.exact {
                        Outcome::Exact
                    } else {
                        Outcome::Approximated
                    },
                    1,
                );
            }
        }
    }
}
