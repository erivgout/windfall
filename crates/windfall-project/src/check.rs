//! Checks that a project obeys every rule the model promises.
//!
//! Commands keep these rules by construction. The check exists for projects
//! that come from outside, such as a file on disk, and for tests.

use std::collections::HashSet;
use std::hash::Hash;

use crate::model::{AutomationId, MAX_AUTOMATION_POINTS};
use crate::model::{
    Channel, ChannelId, ChannelSource, ClipContent, EffectId, Envelope, FORMAT_VERSION,
    MAX_EFFECT_SLOTS, MAX_ENVELOPE_MS, MAX_GAIN, MAX_KEY, MAX_MIXER_TRACKS, MAX_PATTERN_STEPS,
    MAX_SONG_TICKS, MAX_TEMPO_BPM, MAX_TUNE_SEMITONES, MIN_TEMPO_BPM, Mixer, MixerTrack, Note,
    NoteId, Pattern, Project, SamplerSettings, TimeSignature, TrackId,
};

impl Project {
    /// Verifies every rule of the project model and names the first one that
    /// is broken.
    ///
    /// The rules are the ones the doc comments in [`model`](crate::model)
    /// state, plus these, which the commands also keep:
    ///
    /// - Only the master mixer track has id 0, and every other id is below
    ///   `next_id`.
    /// - Lanes are sorted by channel id.
    /// - The master track has no sends, so rerouting a track to the master
    ///   can never close a loop.
    /// - A track has at most one send to any other track.
    /// - No two samples share a path.
    /// - The sample and the mixer track of an audio clip exist.
    /// - What an automation moves exists: its channel, track, send or
    ///   effect is there, the effect is on the track the target names, and
    ///   the setting is one its processor has.
    /// - A note ends at or before the largest tick a `u32` holds. Where it
    ///   starts is not checked: the commands keep new and moved notes
    ///   inside the longest pattern, and a note that an older file has
    ///   further out is harmless, because it never plays.
    /// - The settings of an effect or instrument are stored with every
    ///   value already inside its range.
    pub fn check(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "format version is {}, expected {FORMAT_VERSION}",
                self.format_version
            ));
        }
        check_settings(self)?;
        check_samples(self)?;
        check_mixer(self)?;
        check_channels(self)?;
        let mut targets = HashSet::new();
        for plugin in &self.plugins {
            plugin.validate().map_err(str::to_owned)?;
            let exists = match plugin.target {
                crate::PluginTarget::Instrument { channel } => {
                    self.channel(channel).is_some_and(|channel| {
                        matches!(channel.source, ChannelSource::Instrument { .. })
                    })
                }
                crate::PluginTarget::Effect { effect } => self
                    .mixer
                    .tracks
                    .iter()
                    .any(|track| track.effect(effect).is_some()),
            };
            if !exists || !targets.insert(plugin.target) {
                return Err("a plugin binding has a missing or repeated owner".to_owned());
            }
        }
        check_patterns(self)?;
        check_automations(self)?;
        check_playlist(self)
    }
}

/// True when `value` is a number from `min` to `max`. False for NaN.
fn within(value: f32, min: f32, max: f32) -> bool {
    value >= min && value <= max
}

fn check_color(owner: &str, color: u32) -> Result<(), String> {
    if color > 0xFF_FFFF {
        return Err(format!(
            "{owner} has color {color:#x}, which is not 0xRRGGBB"
        ));
    }
    Ok(())
}

/// Checks that an id is below `next_id`, is not the reserved 0, and has not
/// been seen before in `seen`.
fn check_id<T: Copy + Eq + Hash + Into<u32>>(
    project: &Project,
    kind: &str,
    id: T,
    seen: &mut HashSet<T>,
) -> Result<(), String> {
    let raw: u32 = id.into();
    if raw == 0 {
        return Err(format!(
            "a {kind} has id 0, which is reserved for the master"
        ));
    }
    if raw >= project.next_id {
        return Err(format!(
            "{kind} {raw} is not below the next id to hand out, {}",
            project.next_id
        ));
    }
    if !seen.insert(id) {
        return Err(format!("more than one {kind} has id {raw}"));
    }
    Ok(())
}

pub(crate) fn time_signature_problem(signature: TimeSignature) -> Option<String> {
    if !(1..=16).contains(&signature.numerator) {
        return Some(format!(
            "a time signature needs 1 to 16 beats per bar, not {}",
            signature.numerator
        ));
    }
    if ![2, 4, 8, 16].contains(&signature.denominator) {
        return Some(format!(
            "a time signature's beat unit must be 2, 4, 8 or 16, not {}",
            signature.denominator
        ));
    }
    None
}

fn check_settings(project: &Project) -> Result<(), String> {
    let settings = &project.settings;
    if !(settings.tempo_bpm >= MIN_TEMPO_BPM && settings.tempo_bpm <= MAX_TEMPO_BPM) {
        return Err(format!(
            "tempo {} is outside {MIN_TEMPO_BPM} to {MAX_TEMPO_BPM} bpm",
            settings.tempo_bpm
        ));
    }
    if let Some(problem) = time_signature_problem(settings.time_signature) {
        return Err(problem);
    }
    if !within(settings.swing, 0.0, 1.0) {
        return Err(format!("swing {} is outside 0 to 1", settings.swing));
    }
    Ok(())
}

fn check_samples(project: &Project) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    for sample in &project.samples {
        check_id(project, "sample", sample.id, &mut ids)?;
        if let Some(problem) = sample.path.problem() {
            return Err(format!("sample {}: {problem}", sample.id.0));
        }
        if !paths.insert(&sample.path) {
            return Err(format!(
                "sample {} repeats the path of another sample",
                sample.id.0
            ));
        }
    }
    Ok(())
}

fn check_mixer(project: &Project) -> Result<(), String> {
    let tracks = &project.mixer.tracks;
    let Some(master) = tracks.first() else {
        return Err("the mixer has no master track".to_owned());
    };
    if master.id != TrackId::MASTER {
        return Err("the first mixer track is not the master".to_owned());
    }
    if master.output.is_some() {
        return Err("the master track has an output".to_owned());
    }
    if !master.sends.is_empty() {
        return Err("the master track has sends".to_owned());
    }
    if tracks.len() > MAX_MIXER_TRACKS {
        return Err(format!(
            "the mixer has {} tracks, more than the limit of {MAX_MIXER_TRACKS}",
            tracks.len()
        ));
    }

    let mut ids = HashSet::new();
    for track in &tracks[1..] {
        check_id(project, "mixer track", track.id, &mut ids)?;
    }
    let mut effects = HashSet::new();
    for track in tracks {
        let owner = format!("mixer track {}", track.id.0);
        check_effects(project, &owner, track, &mut effects)?;
        check_color(&owner, track.color)?;
        if !within(track.volume, 0.0, MAX_GAIN) {
            return Err(format!(
                "{owner} has volume {}, outside 0 to {MAX_GAIN}",
                track.volume
            ));
        }
        if !within(track.pan, -1.0, 1.0) {
            return Err(format!("{owner} has pan {}, outside -1 to 1", track.pan));
        }
        if let Some(output) = track.output
            && project.mixer.track(output).is_none()
        {
            return Err(format!(
                "{owner} outputs to mixer track {}, which does not exist",
                output.0
            ));
        }
        let mut targets = HashSet::new();
        for send in &track.sends {
            if project.mixer.track(send.target).is_none() {
                return Err(format!(
                    "{owner} sends to mixer track {}, which does not exist",
                    send.target.0
                ));
            }
            if !targets.insert(send.target) {
                return Err(format!(
                    "{owner} has more than one send to mixer track {}",
                    send.target.0
                ));
            }
            if !within(send.gain, 0.0, MAX_GAIN) {
                return Err(format!(
                    "{owner} has a send with gain {}, outside 0 to {MAX_GAIN}",
                    send.gain
                ));
            }
        }
    }
    if let Some(track) = routing_cycle(&project.mixer) {
        return Err(format!(
            "mixer routing loops back on itself at track {}",
            track.0
        ));
    }
    Ok(())
}

fn check_effects(
    project: &Project,
    owner: &str,
    track: &MixerTrack,
    seen: &mut HashSet<EffectId>,
) -> Result<(), String> {
    if track.effects.len() > MAX_EFFECT_SLOTS {
        return Err(format!(
            "{owner} has {} effects, more than the limit of {MAX_EFFECT_SLOTS}",
            track.effects.len()
        ));
    }
    for effect in &track.effects {
        check_id(project, "effect", effect.id, seen)?;
        let name = format!("{owner}, effect {}", effect.id.0);
        if !within(effect.mix, 0.0, 1.0) {
            return Err(format!("{name} has mix {}, outside 0 to 1", effect.mix));
        }
        if effect.params.sanitized() != effect.params {
            return Err(format!("{name} has a setting outside its range"));
        }
    }
    Ok(())
}

/// Finds a track that can reach itself through outputs and sends.
fn routing_cycle(mixer: &Mixer) -> Option<TrackId> {
    mixer
        .tracks
        .iter()
        .find(|track| {
            routing_targets(mixer, track.id).any(|target| reaches(mixer, target, track.id))
        })
        .map(|track| track.id)
}

/// The tracks `from` feeds directly: its output and its sends.
fn routing_targets(mixer: &Mixer, from: TrackId) -> impl Iterator<Item = TrackId> + '_ {
    mixer.track(from).into_iter().flat_map(|track| {
        track
            .output
            .into_iter()
            .chain(track.sends.iter().map(|send| send.target))
    })
}

/// True when signal leaving `from` can arrive at `to`, or they are the same
/// track.
pub(crate) fn reaches(mixer: &Mixer, from: TrackId, to: TrackId) -> bool {
    let mut seen = HashSet::new();
    let mut pending = vec![from];
    while let Some(track) = pending.pop() {
        if track == to {
            return true;
        }
        if seen.insert(track) {
            pending.extend(routing_targets(mixer, track));
        }
    }
    false
}

fn check_channels(project: &Project) -> Result<(), String> {
    let mut ids = HashSet::new();
    for channel in &project.channels {
        check_id(project, "channel", channel.id, &mut ids)?;
        check_channel(project, channel)?;
    }
    Ok(())
}

fn check_channel(project: &Project, channel: &Channel) -> Result<(), String> {
    let owner = format!("channel {}", channel.id.0);
    check_color(&owner, channel.color)?;
    if !within(channel.volume, 0.0, MAX_GAIN) {
        return Err(format!(
            "{owner} has volume {}, outside 0 to {MAX_GAIN}",
            channel.volume
        ));
    }
    if !within(channel.pan, -1.0, 1.0) {
        return Err(format!("{owner} has pan {}, outside -1 to 1", channel.pan));
    }
    if project.mixer.track(channel.mixer_track).is_none() {
        return Err(format!(
            "{owner} plays into mixer track {}, which does not exist",
            channel.mixer_track.0
        ));
    }
    match &channel.source {
        ChannelSource::Sampler(sampler) => check_sampler(project, &owner, sampler),
        ChannelSource::Instrument { params } => {
            if params.sanitized() != *params {
                return Err(format!(
                    "{owner} has an instrument setting outside its range"
                ));
            }
            Ok(())
        }
    }
}

fn check_sampler(project: &Project, owner: &str, sampler: &SamplerSettings) -> Result<(), String> {
    sampler
        .stretch
        .validate()
        .map_err(|error| format!("{owner}: {error}"))?;
    if let Some(sample) = sampler.sample
        && project.sample(sample).is_none()
    {
        return Err(format!(
            "{owner} plays sample {}, which does not exist",
            sample.0
        ));
    }
    if sampler.root_key > MAX_KEY {
        return Err(format!(
            "{owner} has root key {}, above {MAX_KEY}",
            sampler.root_key
        ));
    }
    if !within(sampler.tune, -MAX_TUNE_SEMITONES, MAX_TUNE_SEMITONES) {
        return Err(format!(
            "{owner} is tuned by {} semitones, outside -{MAX_TUNE_SEMITONES} to {MAX_TUNE_SEMITONES}",
            sampler.tune
        ));
    }
    if !within(sampler.gain, 0.0, MAX_GAIN) {
        return Err(format!(
            "{owner} has sample gain {}, outside 0 to {MAX_GAIN}",
            sampler.gain
        ));
    }
    if !within(sampler.start, 0.0, 1.0) || !within(sampler.end, 0.0, 1.0) {
        return Err(format!(
            "{owner} plays its sample from {} to {}, outside 0 to 1",
            sampler.start, sampler.end
        ));
    }
    if sampler.start >= sampler.end {
        return Err(format!(
            "{owner} has its sample start at {}, not before its end at {}",
            sampler.start, sampler.end
        ));
    }
    if !within(sampler.loop_start, 0.0, 1.0) || !within(sampler.loop_end, 0.0, 1.0) {
        return Err(format!("{owner} has loop points outside 0 to 1"));
    }
    if sampler.loop_start >= sampler.loop_end {
        return Err(format!("{owner} has a loop start not before its end"));
    }
    match &sampler.envelope {
        Some(envelope) => check_envelope(owner, envelope),
        None => Ok(()),
    }
}

fn check_envelope(owner: &str, envelope: &Envelope) -> Result<(), String> {
    let times = [
        ("attack", envelope.attack_ms),
        ("decay", envelope.decay_ms),
        ("release", envelope.release_ms),
    ];
    for (name, value) in times {
        if !within(value, 0.0, MAX_ENVELOPE_MS) {
            return Err(format!(
                "{owner} has an envelope {name} of {value} ms, outside 0 to {MAX_ENVELOPE_MS}"
            ));
        }
    }
    if !within(envelope.sustain, 0.0, 1.0) {
        return Err(format!(
            "{owner} has an envelope sustain of {}, outside 0 to 1",
            envelope.sustain
        ));
    }
    Ok(())
}

fn check_patterns(project: &Project) -> Result<(), String> {
    if project.patterns.is_empty() {
        return Err("the project has no patterns".to_owned());
    }
    let mut ids = HashSet::new();
    let mut notes = HashSet::new();
    for pattern in &project.patterns {
        check_id(project, "pattern", pattern.id, &mut ids)?;
        check_pattern(project, pattern, &mut notes)?;
    }
    Ok(())
}

fn check_pattern(
    project: &Project,
    pattern: &Pattern,
    notes: &mut HashSet<NoteId>,
) -> Result<(), String> {
    let owner = format!("pattern {}", pattern.id.0);
    check_color(&owner, pattern.color)?;
    if !(1..=MAX_PATTERN_STEPS).contains(&pattern.length_steps) {
        return Err(format!(
            "{owner} is {} steps long, outside 1 to {MAX_PATTERN_STEPS}",
            pattern.length_steps
        ));
    }
    let mut previous: Option<ChannelId> = None;
    for lane in &pattern.lanes {
        let lane_owner = format!("{owner}, channel {}", lane.channel.0);
        if previous.is_some_and(|previous| previous >= lane.channel) {
            return Err(format!("{owner} does not keep its lanes sorted by channel"));
        }
        previous = Some(lane.channel);
        if project.channel(lane.channel).is_none() {
            return Err(format!("{lane_owner}: the channel does not exist"));
        }
        if lane.notes.is_empty() {
            return Err(format!("{lane_owner}: the lane is empty"));
        }
        if !lane.notes.is_sorted_by(|a, b| a.sort_key() < b.sort_key()) {
            return Err(format!("{lane_owner}: the notes are not sorted"));
        }
        for note in &lane.notes {
            check_id(project, "note", note.id, notes)?;
            check_note(&lane_owner, note)?;
        }
    }
    Ok(())
}

fn check_note(owner: &str, note: &Note) -> Result<(), String> {
    let name = format!("{owner}, note {}", note.id.0);
    if note.length == 0 {
        return Err(format!("{name} has no length"));
    }
    if note.start.checked_add(note.length).is_none() {
        return Err(format!("{name} ends past the last tick"));
    }
    if note.key > MAX_KEY {
        return Err(format!("{name} has key {}, above {MAX_KEY}", note.key));
    }
    if !within(note.velocity, 0.0, 1.0) {
        return Err(format!(
            "{name} has velocity {}, outside 0 to 1",
            note.velocity
        ));
    }
    if !within(note.pan, -1.0, 1.0) {
        return Err(format!("{name} has pan {}, outside -1 to 1", note.pan));
    }
    Ok(())
}

fn check_automations(project: &Project) -> Result<(), String> {
    let mut ids: HashSet<AutomationId> = HashSet::new();
    for automation in &project.automations {
        check_id(project, "automation", automation.id, &mut ids)?;
        let owner = format!("automation {}", automation.id.0);
        check_color(&owner, automation.color)?;
        if project.automation_range(&automation.target).is_none() {
            return Err(format!(
                "{owner} moves something the project does not have: {:?}",
                automation.target
            ));
        }
        let points = &automation.points;
        if points.is_empty() {
            return Err(format!("{owner} has no points"));
        }
        if points.len() > MAX_AUTOMATION_POINTS {
            return Err(format!(
                "{owner} has {} points, more than the limit of {MAX_AUTOMATION_POINTS}",
                points.len()
            ));
        }
        if !points.is_sorted_by_key(|point| point.tick) {
            return Err(format!("{owner} does not keep its points in order"));
        }
        for point in points {
            if point.tick > MAX_SONG_TICKS {
                return Err(format!(
                    "{owner} has a point past the end of the longest song, tick {MAX_SONG_TICKS}"
                ));
            }
            if !within(point.value, 0.0, 1.0) {
                return Err(format!(
                    "{owner} has a point with value {}, outside 0 to 1",
                    point.value
                ));
            }
            if !within(point.curve, -1.0, 1.0) {
                return Err(format!(
                    "{owner} has a point with curve {}, outside -1 to 1",
                    point.curve
                ));
            }
        }
    }
    Ok(())
}

fn check_playlist(project: &Project) -> Result<(), String> {
    let playlist = &project.playlist;
    let mut tracks = HashSet::new();
    for track in &playlist.tracks {
        check_id(project, "playlist track", track.id, &mut tracks)?;
    }
    if !playlist
        .clips
        .is_sorted_by(|a, b| a.sort_key() < b.sort_key())
    {
        return Err("the playlist clips are not sorted".to_owned());
    }
    let mut clips = HashSet::new();
    for clip in &playlist.clips {
        check_id(project, "clip", clip.id, &mut clips)?;
        let owner = format!("clip {}", clip.id.0);
        if !tracks.contains(&clip.track) {
            return Err(format!(
                "{owner} is on playlist track {}, which does not exist",
                clip.track.0
            ));
        }
        if clip.length == 0 {
            return Err(format!("{owner} has no length"));
        }
        if u64::from(clip.start) + u64::from(clip.length) > u64::from(MAX_SONG_TICKS) {
            return Err(format!(
                "{owner} ends past the end of the longest song, tick {MAX_SONG_TICKS}"
            ));
        }
        if clip.offset > MAX_SONG_TICKS {
            return Err(format!(
                "{owner} has an offset past the end of the longest song, tick {MAX_SONG_TICKS}"
            ));
        }
        match &clip.content {
            ClipContent::Pattern { pattern } => {
                if project.pattern(*pattern).is_none() {
                    return Err(format!(
                        "{owner} plays pattern {}, which does not exist",
                        pattern.0
                    ));
                }
            }
            ClipContent::Audio {
                sample,
                mixer_track,
                gain,
                pan,
                fade_in,
                fade_out,
                reverse: _,
                pitch,
                stretch,
            } => {
                if project.sample(*sample).is_none() {
                    return Err(format!(
                        "{owner} plays sample {}, which does not exist",
                        sample.0
                    ));
                }
                if project.mixer.track(*mixer_track).is_none() {
                    return Err(format!(
                        "{owner} plays into mixer track {}, which does not exist",
                        mixer_track.0
                    ));
                }
                if !within(*gain, 0.0, MAX_GAIN) {
                    return Err(format!("{owner} has gain {gain}, outside 0 to {MAX_GAIN}"));
                }
                if !within(*pan, -1.0, 1.0) {
                    return Err(format!("{owner} has pan {pan}, outside -1 to 1"));
                }
                if !within(*pitch, -MAX_TUNE_SEMITONES, MAX_TUNE_SEMITONES) {
                    return Err(format!(
                        "{owner} is pitched by {pitch} semitones, outside -{MAX_TUNE_SEMITONES} to {MAX_TUNE_SEMITONES}"
                    ));
                }
                if let crate::ClipStretch::Spectral { ratio, .. } = stretch
                    && (!ratio.is_finite()
                        || !(0.25..=4.0).contains(ratio)
                        || !within(*pitch, -24.0, 24.0))
                {
                    return Err(format!("{owner} has invalid spectral stretch settings"));
                }
                if *fade_in > MAX_SONG_TICKS || *fade_out > MAX_SONG_TICKS {
                    return Err(format!(
                        "{owner} has a fade longer than the longest song, {MAX_SONG_TICKS} ticks"
                    ));
                }
            }
            ClipContent::Automation { automation } => {
                if project.automation(*automation).is_none() {
                    return Err(format!(
                        "{owner} shows automation {}, which does not exist",
                        automation.0
                    ));
                }
            }
        }
    }
    Ok(())
}
