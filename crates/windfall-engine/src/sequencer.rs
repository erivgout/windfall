//! The transport: where the playhead is and which notes fall on which
//! output frame.
//!
//! Musical time is tied to output frames by a [`Clock`], a straight line
//! from an anchor frame. A note plays on the first frame whose time is at
//! or after the note's tick, and that frame is computed from the anchor
//! alone, never by adding up block lengths. However the output is cut into
//! buffers, a note therefore lands on the same frame, exactly once.

use windfall_core::samples_per_tick;
use windfall_ipc::PlayMode;
use windfall_project::PatternId;

use crate::plan::{FALLBACK_LOOP_TICKS, Plan, PlanPattern};

/// How close to a frame boundary, in frames, a note may fall past it and
/// still be played on it. It absorbs floating-point rounding, so a note
/// that sits exactly on a frame in exact arithmetic is never a frame late.
const FRAME_TOLERANCE: f64 = 1e-6;

/// Note-ons the sequencer can hand over for one block. More notes than this
/// inside a few milliseconds are dropped.
pub(crate) const MAX_TRIGGERS: usize = 1024;

/// A straight line from ticks to frames.
#[derive(Debug, Clone, Copy)]
struct Clock {
    anchor_frame: u64,
    /// Tick at the anchor frame. It goes negative when a loop wraps, which
    /// keeps the line unbroken across the wrap.
    anchor_tick: f64,
    frames_per_tick: f64,
}

impl Clock {
    /// First frame at or after `tick`, never before the anchor.
    fn frame_of(&self, tick: f64) -> u64 {
        let frames = ((tick - self.anchor_tick) * self.frames_per_tick - FRAME_TOLERANCE).ceil();
        self.anchor_frame + frames.max(0.0) as u64
    }

    /// Tick at `frame`, which must not be before the anchor.
    fn tick_at(&self, frame: u64) -> f64 {
        self.anchor_tick + (frame - self.anchor_frame) as f64 / self.frames_per_tick
    }
}

/// What separates notes already behind the playhead from notes still to
/// come, at the moment the clock was last replaced.
#[derive(Debug, Clone, Copy)]
enum Floor {
    /// Playback jumped to this tick. Notes before it are skipped.
    Tick(f64),
    /// The tempo changed at `frame`. Notes the old clock placed before that
    /// frame have played, and notes it placed at or after it have not.
    /// Asking the old clock keeps a note sitting right on the change from
    /// being played twice or not at all.
    Clock { clock: Clock, frame: u64 },
}

/// A note to start on an exact frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Trigger {
    pub frame: u64,
    /// Position in the order the sequencer found the notes of one block.
    /// Sorting by frame and then by this gives one fixed order for notes
    /// that share a frame.
    order: u32,
    /// Index into [`Plan::channels`].
    pub channel: usize,
    pub key: u8,
    pub velocity: f32,
    pub pan: f32,
    /// Frame on which the note ends. Only samplers with an envelope care.
    pub release_at: u64,
}

/// The note-ons of one block, in a buffer allocated once.
pub(crate) struct Triggers {
    items: Vec<Trigger>,
}

impl Triggers {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(MAX_TRIGGERS),
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, index: usize) -> Trigger {
        self.items[index]
    }

    fn push(&mut self, mut trigger: Trigger) {
        if self.items.len() < self.items.capacity() {
            trigger.order = self.items.len() as u32;
            self.items.push(trigger);
        }
    }

    fn sort(&mut self) {
        self.items
            .sort_unstable_by_key(|trigger| (trigger.frame, trigger.order));
    }
}

pub(crate) struct Sequencer {
    sample_rate: f64,
    playing: bool,
    mode: PlayMode,
    pattern: PatternId,
    loop_song: bool,
    clock: Clock,
    floor: Floor,
    /// Where playback last started or jumped to. It is the playhead while
    /// stopped, so stopping returns here, and the next `play` starts here.
    position: f64,
    /// Passes left before playback ends by itself. `None` follows the
    /// transport's own looping. The offline renderer sets it.
    passes_left: Option<u32>,
}

impl Sequencer {
    pub fn new(sample_rate: u32, plan: &Plan) -> Self {
        let sample_rate = f64::from(sample_rate);
        Self {
            sample_rate,
            playing: false,
            mode: PlayMode::Pattern,
            pattern: PatternId(0),
            loop_song: false,
            clock: Clock {
                anchor_frame: 0,
                anchor_tick: 0.0,
                frames_per_tick: samples_per_tick(plan.tempo_bpm, sample_rate),
            },
            floor: Floor::Tick(0.0),
            position: 0.0,
            passes_left: None,
        }
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    /// The playhead in ticks as of frame `now`.
    pub fn tick(&self, plan: &Plan, now: u64) -> f64 {
        if !self.playing {
            return self.position;
        }
        let tick = self.clock.tick_at(now.max(self.clock.anchor_frame));
        let length = self.length(plan);
        // The wrap itself happens while the next block is processed.
        let wrapped = if tick >= length { tick - length } else { tick };
        wrapped.max(0.0)
    }

    /// Follows a new plan from frame `now`: picks up a tempo change without
    /// moving the playhead, and keeps the playhead inside whatever is
    /// playing if that got shorter.
    pub fn set_plan(&mut self, plan: &Plan, now: u64) {
        let frames_per_tick = samples_per_tick(plan.tempo_bpm, self.sample_rate);
        if frames_per_tick != self.clock.frames_per_tick {
            if self.playing && now > self.clock.anchor_frame {
                let old = self.clock;
                self.clock = Clock {
                    anchor_frame: now,
                    anchor_tick: old.tick_at(now),
                    frames_per_tick,
                };
                self.floor = Floor::Clock {
                    clock: old,
                    frame: now,
                };
            } else {
                self.clock.frames_per_tick = frames_per_tick;
            }
        }
        self.keep_in_range(plan, now);
    }

    /// Starts playback from the stopped position. `passes` limits how many
    /// times the pattern or song plays before stopping by itself.
    pub fn play(&mut self, plan: &Plan, now: u64, passes: Option<u32>) {
        if self.playing {
            return;
        }
        self.playing = true;
        self.passes_left = passes;
        self.jump(self.position, now);
        self.keep_in_range(plan, now);
    }

    /// Stops and returns the playhead to where playback last started.
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Moves the playhead. Returns true when that interrupted playback.
    pub fn seek(&mut self, tick: f64, plan: &Plan, now: u64) -> bool {
        let tick = if tick.is_finite() { tick.max(0.0) } else { 0.0 };
        self.position = tick;
        if !self.playing {
            return false;
        }
        self.jump(tick, now);
        self.keep_in_range(plan, now);
        true
    }

    /// Applies transport settings. Returns true when playback restarted from
    /// the top because the mode changed while playing.
    pub fn set_transport(
        &mut self,
        mode: PlayMode,
        pattern: PatternId,
        loop_song: bool,
        plan: &Plan,
        now: u64,
    ) -> bool {
        let mode_changed = mode != self.mode;
        self.mode = mode;
        self.pattern = pattern;
        self.loop_song = loop_song;
        if mode_changed {
            // A pattern position means nothing on the playlist and the
            // other way round.
            self.position = 0.0;
            if self.playing {
                self.jump(0.0, now);
            }
        }
        self.keep_in_range(plan, now);
        mode_changed && self.playing
    }

    /// Finds every note that starts in frames `from..to`, in the order they
    /// must be started, and moves the transport through any loop point or
    /// song end inside that range.
    pub fn collect(&mut self, plan: &Plan, from: u64, to: u64, triggers: &mut Triggers) {
        let mut cursor = from;
        while self.playing && cursor < to {
            let length = self.length(plan);
            // First frame at or past the end of the pattern or song.
            let end_frame = self.clock.frame_of(length).max(cursor);
            // Notes just before the end can round onto that frame, so this
            // pass still owns it.
            let pass_end = to.min(end_frame + 1);
            match self.mode {
                PlayMode::Pattern => {
                    if let Some(pattern) = self.pattern(plan) {
                        self.collect_pattern(pattern, cursor, pass_end, triggers);
                    }
                }
                PlayMode::Song => self.collect_song(plan, cursor, pass_end, triggers),
            }
            if end_frame >= to {
                break;
            }
            if self.another_pass() {
                self.clock.anchor_tick -= length;
                match &mut self.floor {
                    Floor::Tick(tick) => *tick -= length,
                    Floor::Clock { clock, .. } => clock.anchor_tick -= length,
                }
                cursor = end_frame;
            } else {
                self.finish();
            }
        }
        triggers.sort();
    }

    fn collect_pattern(&self, pattern: &PlanPattern, from: u64, to: u64, triggers: &mut Triggers) {
        let (low, high) = self.window(from, to);
        let first = pattern.events.partition_point(|event| event.tick < low);
        for event in &pattern.events[first..] {
            if event.tick > high {
                break;
            }
            let Some(frame) = self.due(event.tick, from, to) else {
                continue;
            };
            triggers.push(Trigger {
                frame,
                order: 0,
                channel: event.channel,
                key: event.key,
                velocity: event.velocity,
                pan: event.pan,
                release_at: self.clock.frame_of(event.tick + event.length),
            });
        }
    }

    fn collect_song(&self, plan: &Plan, from: u64, to: u64, triggers: &mut Triggers) {
        let (low, high) = self.window(from, to);
        let first = plan
            .clips
            .partition_point(|clip| f64::from(clip.reach) < low);
        for clip in &plan.clips[first..] {
            let clip_start = f64::from(clip.start);
            let clip_end = f64::from(clip.end);
            if clip_start > high {
                break;
            }
            if clip_end < low {
                continue;
            }
            let pattern = &plan.patterns[clip.pattern];
            let pattern_length = f64::from(pattern.length);
            // Timeline tick at which the clip's first, possibly cut off,
            // repeat of the pattern begins.
            let origin = clip_start - f64::from(clip.offset);
            let first_repeat = ((low - origin) / pattern_length).floor().max(0.0) as u64;
            let last_repeat = ((high.min(clip_end) - origin) / pattern_length)
                .floor()
                .max(0.0) as u64;
            for repeat in first_repeat..=last_repeat {
                let shift = origin + repeat as f64 * pattern_length;
                let start = pattern
                    .events
                    .partition_point(|event| shift + event.tick < low);
                for event in &pattern.events[start..] {
                    let tick = shift + event.tick;
                    if tick > high {
                        break;
                    }
                    if tick < clip_start || tick >= clip_end {
                        continue;
                    }
                    let Some(frame) = self.due(tick, from, to) else {
                        continue;
                    };
                    triggers.push(Trigger {
                        frame,
                        order: 0,
                        channel: event.channel,
                        key: event.key,
                        velocity: event.velocity,
                        pan: event.pan,
                        // The clip is a window onto the pattern: a held note
                        // ends where the clip does.
                        release_at: self.clock.frame_of((tick + event.length).min(clip_end)),
                    });
                }
            }
        }
    }

    /// A range of ticks sure to contain every note due in frames
    /// `from..to`. It is wider than needed; [`Self::due`] decides exactly.
    fn window(&self, from: u64, to: u64) -> (f64, f64) {
        let tick_per_frame = 1.0 / self.clock.frames_per_tick;
        // After a tempo change, notes between the last frame of the old
        // tempo and the first frame of the new one are still due.
        let behind = match &self.floor {
            Floor::Tick(_) => 0.0,
            Floor::Clock { clock, .. } => 1.0 / clock.frames_per_tick,
        };
        let from = from.max(self.clock.anchor_frame);
        let to = to.max(from);
        (
            self.clock.tick_at(from) - 2.0 * tick_per_frame - 2.0 * behind,
            self.clock.tick_at(to) + tick_per_frame,
        )
    }

    /// The frame a note at `tick` plays on, if that is inside `from..to`
    /// and the note is not already behind the playhead.
    fn due(&self, tick: f64, from: u64, to: u64) -> Option<u64> {
        let ahead = match &self.floor {
            Floor::Tick(floor) => tick >= *floor,
            Floor::Clock { clock, frame } => clock.frame_of(tick) >= *frame,
        };
        let frame = self.clock.frame_of(tick);
        (ahead && frame >= from && frame < to).then_some(frame)
    }

    /// Length in ticks of what is playing: the pattern, or the whole song.
    fn length(&self, plan: &Plan) -> f64 {
        match self.mode {
            PlayMode::Pattern => f64::from(
                self.pattern(plan)
                    .map_or(FALLBACK_LOOP_TICKS, |pattern| pattern.length),
            ),
            PlayMode::Song => f64::from(plan.song_end),
        }
    }

    /// The transport's pattern, or the first one when that id is gone.
    fn pattern<'a>(&self, plan: &'a Plan) -> Option<&'a PlanPattern> {
        plan.pattern_ids
            .get(self.pattern.0)
            .map(|index| &plan.patterns[index])
            .or(plan.patterns.first())
    }

    fn loops(&self) -> bool {
        match self.passes_left {
            Some(passes) => passes > 1,
            None => self.mode == PlayMode::Pattern || self.loop_song,
        }
    }

    /// Decides at the end of a pass whether to go around again.
    fn another_pass(&mut self) -> bool {
        let again = self.loops();
        if let Some(passes) = &mut self.passes_left {
            *passes = passes.saturating_sub(1);
        }
        again
    }

    /// Restarts the clock at `tick` on frame `now`.
    fn jump(&mut self, tick: f64, now: u64) {
        self.clock.anchor_frame = now;
        self.clock.anchor_tick = tick;
        self.floor = Floor::Tick(tick);
    }

    /// Playback ran out by itself. Voices are left to ring.
    fn finish(&mut self) {
        self.playing = false;
        self.passes_left = None;
        self.position = 0.0;
    }

    /// Brings the playhead back inside the pattern or song when a jump or a
    /// change to the plan left it past the end.
    fn keep_in_range(&mut self, plan: &Plan, now: u64) {
        if !self.playing {
            return;
        }
        let length = self.length(plan);
        if length <= 0.0 {
            self.finish();
            return;
        }
        // Reaching the end exactly on frame `now` is an ordinary wrap, and
        // `collect` handles it. Only a playhead that is already past the end
        // needs moving.
        let jumped_past = matches!(self.floor, Floor::Tick(tick) if tick >= length);
        if jumped_past || self.clock.frame_of(length) < now {
            if self.loops() {
                let tick = self.clock.tick_at(now.max(self.clock.anchor_frame)) % length;
                self.jump(tick, now);
            } else {
                self.finish();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use windfall_project::Project;

    use super::*;
    use crate::plan::compile;
    use crate::pool::SamplePool;

    #[test]
    fn a_note_exactly_on_a_frame_plays_on_that_frame() {
        // At 69 bpm and 44.1 kHz, step 23 falls exactly on frame 220500, but
        // the floating-point product comes out a hair above it.
        let clock = Clock {
            anchor_frame: 0,
            anchor_tick: 0.0,
            frames_per_tick: samples_per_tick(69.0, 44_100.0),
        };
        assert!(23.0 * 240.0 * clock.frames_per_tick > 220_500.0);
        assert_eq!(clock.frame_of(23.0 * 240.0), 220_500);
        assert_eq!(clock.frame_of(0.0), 0);
        assert_eq!(clock.frame_of(-5.0), 0);
    }

    #[test]
    fn stop_returns_to_where_playback_started() {
        let plan = compile(&Project::new("t"), &SamplePool::new());
        let mut sequencer = Sequencer::new(48_000, &plan);
        sequencer.seek(480.0, &plan, 0);
        sequencer.play(&plan, 0, None);
        let mut triggers = Triggers::new();
        sequencer.collect(&plan, 0, 10_000, &mut triggers);
        assert!(sequencer.tick(&plan, 10_000) > 480.0);
        sequencer.stop();
        assert_eq!(sequencer.tick(&plan, 10_000), 480.0);
    }

    #[test]
    fn playing_an_empty_song_stops_at_once() {
        let plan = compile(&Project::new("t"), &SamplePool::new());
        let mut sequencer = Sequencer::new(48_000, &plan);
        sequencer.set_transport(PlayMode::Song, PatternId(1), true, &plan, 0);
        sequencer.play(&plan, 0, None);
        assert!(!sequencer.playing());
    }
}
