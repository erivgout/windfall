//! Mixer inserts to mixer tracks: faders, routing, sends and effects.
//!
//! FL Studio always has its whole mixer in a file, 127 inserts from
//! version 12.9 on, used or not. Only the inserts that are in use are
//! brought over: the master, and any insert that has a name, an effect, a
//! fader, pan, mute or solo that was touched, routing other than "to the
//! master", or something playing into it. The last insert of a full mixer
//! is the "current" one, which follows whatever is selected in FL Studio
//! and is never brought over.
//!
//! Routing: an insert feeds any number of others, each at its own level.
//! A Windfall track has one output without a level, and sends with one.
//! So an insert that feeds the master at full level gets the master as
//! its output and its other routes as sends; an insert with a single
//! route at full level gets that as its output; and anything else gets no
//! output and a send for every route, which is the same signal flow.

use std::collections::BTreeSet;

use windfall_project::{
    Command, EffectSlotPatch, MAX_EFFECT_SLOTS, MAX_MIXER_TRACKS, MixerTrackPatch, TrackId,
};

use super::{
    Builder, ImportedEffect, PluginPlace, PluginPlaceholder, chosen_color, effects, name_or,
};
use crate::model::{Insert, Route, Slot};
use crate::report::{Outcome, ReportSection, Worst};
use crate::units::{fader_gain, fader_is_exact, insert_pan};

/// The level of a route, a fader or a mix that was left alone.
const FULL: i32 = 12_800;

const FITTED_FADER: &str = "A fader or send level that is not at 0% or 100% went through a fitted curve, so it can be off by a little.";

impl Builder<'_> {
    pub(super) fn mixer(&mut self) {
        let flp = self.flp;
        let inserts = &flp.mixer.inserts;
        let current = current_insert(inserts.len());
        let used = self.inserts_in_use(current);

        let mut left_out = 0_u32;
        for &index in &used {
            let insert = &inserts[usize::from(index)];
            if index == 0 {
                self.fader(insert, index);
                continue;
            }
            if self.tracks.len() >= MAX_MIXER_TRACKS {
                left_out += 1;
                continue;
            }
            let name = name_or(insert.name.as_deref(), || format!("Insert {index}"));
            let command = Command::AddMixerTrack { name: Some(name) };
            if let Some(id) = self.create(ReportSection::Mixer, "A mixer track", command) {
                self.tracks.insert(index, TrackId(id));
                self.fader(insert, index);
            }
        }
        if left_out > 0 {
            self.report
                .count(ReportSection::Mixer, Outcome::Dropped, left_out);
            self.report.say(
                ReportSection::Mixer,
                Outcome::Dropped,
                format!(
                    "Windfall's mixer holds {MAX_MIXER_TRACKS} tracks. {left_out} more were left out, and what played into them plays into the master."
                ),
            );
        }
        if let Some(current) = current
            && inserts
                .get(usize::from(current))
                .is_some_and(|insert| !insert.slots.is_empty())
        {
            self.report.say(
                ReportSection::Mixer,
                Outcome::Dropped,
                "The effects on FL Studio's \"current\" mixer track, which follows the selection, were left out.",
            );
        }

        for &index in &used {
            if index != 0 && self.tracks.contains_key(&index) {
                self.routing(&inserts[usize::from(index)], index, current);
            }
        }
        for &index in &used {
            if self.tracks.contains_key(&index) {
                self.effects(&inserts[usize::from(index)], index);
            }
        }
    }

    /// The inserts that are brought over, in order.
    fn inserts_in_use(&self, current: Option<u16>) -> Vec<u16> {
        let inserts = &self.flp.mixer.inserts;
        let mut used: BTreeSet<u16> = BTreeSet::from([0]);
        for (index, insert) in inserts.iter().enumerate() {
            if touched(insert) {
                used.insert(index as u16);
            }
        }
        for channel in &self.flp.channels {
            if let Some(insert) = channel.insert.and_then(|insert| u16::try_from(insert).ok()) {
                used.insert(insert);
            }
        }
        // An insert that something in use feeds is in use too.
        let mut frontier: Vec<u16> = used.iter().copied().collect();
        while let Some(index) = frontier.pop() {
            let Some(insert) = inserts.get(usize::from(index)) else {
                continue;
            };
            for route in &insert.routes {
                if used.insert(route.target) {
                    frontier.push(route.target);
                }
            }
        }
        used.into_iter()
            .filter(|&index| usize::from(index) < inserts.len() && Some(index) != current)
            .collect()
    }

    /// Sets the name, colour, fader, pan, mute and solo of the track an
    /// insert became, and says what else the insert had.
    fn fader(&mut self, insert: &Insert, index: u16) {
        let section = ReportSection::Mixer;
        let Some(&track) = self.tracks.get(&index) else {
            return;
        };
        let mut worst = Worst::new();
        let name = name_or(insert.name.as_deref(), || {
            self.project()
                .mixer
                .track(track)
                .map_or_else(String::new, |track| track.name.clone())
        });
        let raw = insert.volume.unwrap_or(FULL);
        let mut volume = fader_gain(raw);
        if !fader_is_exact(raw) {
            worst.note(Outcome::Approximated);
            self.report
                .say(section, Outcome::Approximated, FITTED_FADER);
        }
        if index == 0 {
            // FL Studio's main volume comes after the master insert. The
            // master's fader takes both.
            let main = self.main_volume();
            if main != FULL {
                volume *= fader_gain(main);
                worst.note(Outcome::Approximated);
                self.report.say(
                    section,
                    Outcome::Approximated,
                    "The project's main volume was folded into the master track's fader.",
                );
            }
        }
        let patch = MixerTrackPatch {
            name: (index != 0 || !name.eq_ignore_ascii_case("master")).then(|| name.clone()),
            color: chosen_color(insert.color),
            volume: Some(volume),
            pan: Some(insert_pan(insert.pan.unwrap_or(0))),
            muted: Some(!insert.enabled()),
            solo: Some(insert.solo()),
        };
        let command = Command::UpdateMixerTrack { id: track, patch };
        if self
            .apply(
                section,
                &format!("The settings of the mixer track \"{name}\""),
                command,
            )
            .is_none()
        {
            worst.note(Outcome::Dropped);
        }

        let mut lost = Vec::new();
        if insert.polarity_reversed() {
            lost.push("reversed polarity");
        }
        if insert.channels_swapped() {
            lost.push("swapped left and right");
        }
        if insert.stereo_separation.is_some_and(|amount| amount != 0) {
            lost.push("stereo separation");
        }
        if insert
            .eq
            .iter()
            .any(|band| band.gain.is_some_and(|gain| gain != 0))
        {
            lost.push("the track's own three-band equaliser");
        }
        if !lost.is_empty() {
            worst.note(Outcome::Approximated);
            self.report.say(
                section,
                Outcome::Approximated,
                format!(
                    "Mixer track \"{name}\": Windfall's tracks have no {}, so that was left out.",
                    lost.join(", no ")
                ),
            );
        }
        self.report.count(section, worst.0, 1);
    }

    /// The main volume as FL Studio's number, where 12800 is 100%.
    fn main_volume(&self) -> i32 {
        let flp = self.flp;
        flp.mixer.main_volume().unwrap_or_else(|| {
            // Older files have it as a byte, where 128 is full.
            flp.settings
                .main_volume
                .map_or(FULL, |volume| i32::from(volume) * 100)
        })
    }

    fn routing(&mut self, insert: &Insert, index: u16, current: Option<u16>) {
        let section = ReportSection::Mixer;
        let Some(&track) = self.tracks.get(&index) else {
            return;
        };
        let name = self.track_name(track);
        let routes: Vec<Route> = insert
            .routes
            .iter()
            .copied()
            .filter(|route| {
                route.target != index
                    && Some(route.target) != current
                    && self.tracks.contains_key(&route.target)
            })
            .collect();
        let full = |route: &Route| route.level.unwrap_or(FULL) == FULL;
        let output = routes
            .iter()
            .find(|route| route.target == 0 && full(route))
            .or_else(|| {
                routes
                    .first()
                    .filter(|route| routes.len() == 1 && full(route))
            })
            .map(|route| route.target);

        let mut output_set = false;
        if let Some(target) = output {
            let command = Command::SetTrackOutput {
                id: track,
                output: Some(self.tracks[&target]),
            };
            match self.run(command) {
                Ok(_) => output_set = true,
                Err(_) => self.loop_refused(&name, target),
            }
        }
        if !output_set {
            // A new track goes to the master, and this one does not.
            let command = Command::SetTrackOutput {
                id: track,
                output: None,
            };
            self.apply(
                section,
                &format!("The output of the mixer track \"{name}\""),
                command,
            );
        }
        for route in routes {
            if output_set && Some(route.target) == output {
                continue;
            }
            let level = route.level.unwrap_or(FULL);
            let command = Command::SetSend {
                from: track,
                to: self.tracks[&route.target],
                gain: Some(fader_gain(level)),
            };
            match self.run(command) {
                Ok(_) => {
                    self.sends.insert((index, route.target));
                    if !fader_is_exact(level) {
                        self.report
                            .say(section, Outcome::Approximated, FITTED_FADER);
                    }
                }
                Err(_) => self.loop_refused(&name, route.target),
            }
        }
    }

    fn loop_refused(&mut self, from: &str, target: u16) {
        let to = self
            .tracks
            .get(&target)
            .map_or_else(String::new, |&track| self.track_name(track));
        self.report.count(ReportSection::Mixer, Outcome::Dropped, 1);
        self.report.say(
            ReportSection::Mixer,
            Outcome::Dropped,
            format!(
                "The route from \"{from}\" to \"{to}\" would feed a mixer track back into itself, which Windfall does not allow. It was left out."
            ),
        );
    }

    pub(super) fn track_name(&self, track: TrackId) -> String {
        self.project()
            .mixer
            .track(track)
            .map_or_else(String::new, |track| track.name.clone())
    }

    fn effects(&mut self, insert: &Insert, index: u16) {
        let Some(&track) = self.tracks.get(&index) else {
            return;
        };
        let track_name = self.track_name(track);
        let mut slots: Vec<&Slot> = insert.slots.iter().collect();
        slots.sort_by_key(|slot| slot.index);
        for slot in slots {
            self.effect(insert, index, track, &track_name, slot);
        }
    }

    fn effect(
        &mut self,
        insert: &Insert,
        index: u16,
        track: TrackId,
        track_name: &str,
        slot: &Slot,
    ) {
        let section = ReportSection::Effects;
        let plugin = &slot.plugin;
        let hosted = plugin.hosted();
        let shown = name_or(slot.name.as_deref(), || {
            hosted
                .as_ref()
                .and_then(|hosted| hosted.name.clone())
                .unwrap_or_else(|| plugin.internal_name.clone())
        });

        let Some(translated) =
            effects::translate(&plugin.internal_name, &plugin.state, self.tempo_bpm)
        else {
            self.report.count(section, Outcome::Placeholder, 1);
            let what = if plugin.is_hosted() {
                let format = hosted
                    .as_ref()
                    .map_or(PluginPlaceholderFormat::UNKNOWN, |hosted| {
                        hosted.format.label()
                    });
                let vendor = hosted
                    .as_ref()
                    .and_then(|hosted| hosted.vendor.as_deref())
                    .map_or_else(String::new, |vendor| format!(" by {vendor}"));
                format!(
                    "Mixer track \"{track_name}\": the plugin \"{shown}\"{vendor} ({format}) is not one Windfall can load yet. Its place and its settings were kept for when it can."
                )
            } else {
                format!(
                    "Mixer track \"{track_name}\": the effect \"{shown}\" has no Windfall equivalent. It was left out of the chain, and its settings were kept."
                )
            };
            self.report.say(section, Outcome::Placeholder, what);
            self.plugins.push(PluginPlaceholder {
                place: PluginPlace::Effect {
                    track,
                    slot: slot.index,
                },
                internal_name: plugin.internal_name.clone(),
                name: hosted.as_ref().and_then(|hosted| hosted.name.clone()),
                vendor: hosted.as_ref().and_then(|hosted| hosted.vendor.clone()),
                format: hosted.as_ref().map(|hosted| hosted.format),
                path: hosted.as_ref().and_then(|hosted| hosted.path.clone()),
                state: plugin.state.clone(),
            });
            return;
        };

        let chain = self
            .project()
            .mixer
            .track(track)
            .map_or(0, |track| track.effects.len());
        if chain >= MAX_EFFECT_SLOTS {
            self.report.count(section, Outcome::Dropped, 1);
            self.report.say(
                section,
                Outcome::Dropped,
                format!(
                    "Mixer track \"{track_name}\": a track holds {MAX_EFFECT_SLOTS} effects, so \"{shown}\" was left out."
                ),
            );
            return;
        }
        let kind = translated.params.kind();
        let what = format!("The effect \"{shown}\" on the mixer track \"{track_name}\"");
        let command = Command::AddEffect {
            track,
            kind,
            index: None,
        };
        let Some(created) = self.apply(section, &what, command) else {
            self.report.count(section, Outcome::Dropped, 1);
            return;
        };
        let id = windfall_project::EffectId(created[0]);
        let command = Command::SetEffectParams {
            track,
            effect: id,
            params: translated.params,
        };
        self.apply(
            section,
            &format!("The settings of {}", lower_first(&what)),
            command,
        );
        let mix = slot.mix.unwrap_or(FULL);
        let patch = EffectSlotPatch {
            // Switching all of an insert's slots off is one button in FL
            // Studio; here every effect of the track is switched off.
            enabled: Some(slot.enabled.unwrap_or(true) && insert.effects_enabled()),
            mix: Some((mix as f32 / FULL as f32).clamp(0.0, 1.0)),
        };
        let command = Command::UpdateEffect {
            track,
            effect: id,
            patch,
        };
        self.apply(
            section,
            &format!("The mix of {}", lower_first(&what)),
            command,
        );

        let outcome = if translated.notes.is_empty() {
            Outcome::Exact
        } else {
            Outcome::Approximated
        };
        self.report.count(section, outcome, 1);
        if !translated.notes.is_empty() {
            self.report.say(
                section,
                Outcome::Approximated,
                format!(
                    "Mixer track \"{track_name}\": \"{shown}\" became a Windfall {}: {}.",
                    kind.name().to_lowercase(),
                    translated.notes.join("; ")
                ),
            );
        }
        self.effects.insert(
            (index, u16::from(slot.index)),
            ImportedEffect {
                track,
                id,
                kind,
                links: translated.links,
            },
        );
    }
}

/// Words for a plugin format the wrapper did not give.
struct PluginPlaceholderFormat;

impl PluginPlaceholderFormat {
    const UNKNOWN: &'static str = "an unknown format";
}

/// True for an insert somebody did something with.
fn touched(insert: &Insert) -> bool {
    let named = insert
        .name
        .as_deref()
        .is_some_and(|name| !name.trim().is_empty());
    let moved = insert.volume.is_some_and(|volume| volume != FULL)
        || insert.pan.is_some_and(|pan| pan != 0);
    let to_master_only = insert.routes.len() == 1
        && insert.routes[0].target == 0
        && insert.routes[0].level.unwrap_or(FULL) == FULL;
    let rerouted = !insert.routes.is_empty() && !to_master_only;
    named || moved || rerouted || !insert.slots.is_empty() || !insert.enabled() || insert.solo()
}

/// The number of FL Studio's "current" insert, the last of a full mixer.
///
/// PyFLP lists the sizes the mixer has had: 105 inserts from version 9 and
/// 127 from 12.9, each with the master first and the current insert last.
/// A file with another number of inserts is a preset or a fixture, and has
/// none.
fn current_insert(count: usize) -> Option<u16> {
    matches!(count, 105 | 127).then(|| (count - 1) as u16)
}

fn lower_first(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().chain(characters).collect(),
        None => String::new(),
    }
}
