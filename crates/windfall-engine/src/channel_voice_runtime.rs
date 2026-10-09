//! Prepared control-side storage for channel note routing. No callback allocation.
use windfall_dsp::NoteInstanceId;
use windfall_project::ChannelVoiceSettings;

use crate::channel_voice::*;
use crate::sequencer::Clock;
use crate::voice::{Note, Origin};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Generator {
    Arp,
    Echo,
}
#[derive(Clone, Copy)]
pub(crate) struct Owner {
    pub note: Note,
    pub instance: NoteInstanceId,
    pub generated: Option<(Generator, u64)>,
}

pub(crate) struct ChannelVoiceRuntime {
    pub settings: ChannelVoiceSettings,
    pub envelopes: PreparedChannelEnvelopes,
    pub glide: f64,
    pub arp: Arpeggiator,
    pub echo: NoteEcho,
    pub held: HeldNotes,
    pub owners: [Option<Owner>; MAX_HELD_NOTES],
    chord: [Option<Note>; MAX_CHORD_NOTES],
    echo_sources: [Option<(u64, Note)>; ECHO_EVENT_CAPACITY],
    pub arp_events: NoteEvents,
    pub echo_events: NoteEvents,
    pub rejected_echoes: u64,
}

impl ChannelVoiceRuntime {
    pub fn prepare(settings: ChannelVoiceSettings, rate: f64) -> Self {
        let settings = settings.sanitized();
        Self {
            settings,
            envelopes: PreparedChannelEnvelopes::prepare(settings.envelopes, rate),
            glide: settings.polyphony.glide_coefficient(rate),
            arp: Arpeggiator::prepare(settings.arpeggiator),
            echo: NoteEcho::prepare(settings.echo),
            held: HeldNotes::default(),
            owners: [None; MAX_HELD_NOTES],
            chord: [None; MAX_CHORD_NOTES],
            echo_sources: [None; ECHO_EVENT_CAPACITY],
            arp_events: NoteEvents::default(),
            echo_events: NoteEvents::default(),
            rejected_echoes: 0,
        }
    }

    pub fn chord_on(&mut self, note: Note) {
        let slot = self
            .chord
            .iter()
            .position(|n| n.is_some_and(|n| n.key == note.key && n.origin == note.origin))
            .or_else(|| self.chord.iter().position(Option::is_none));
        if let Some(slot) = slot {
            self.chord[slot] = Some(note);
            self.refresh_chord();
        }
    }

    fn refresh_chord(&mut self) {
        let mut used = 0;
        for index in 0..self.chord.len() {
            if let Some(note) = self.chord[index] {
                self.chord[used] = Some(note);
                used += 1;
            }
        }
        self.chord[used..].fill(None);
        let mut chord = [HeldNote::default(); MAX_CHORD_NOTES];
        let mut len = 0;
        for note in self.chord.iter().flatten() {
            chord[len] = HeldNote {
                key: note.key,
                velocity: note.velocity,
            };
            len += 1;
        }
        self.arp.set_chord(&chord[..len]);
    }

    pub fn chord_off(&mut self, key: u8, origin: Origin) {
        let mut changed = false;
        for slot in &mut self.chord {
            if slot.is_some_and(|n| n.key == key && n.origin == origin) {
                *slot = None;
                changed = true;
            }
        }
        if changed {
            self.refresh_chord();
        }
    }

    pub fn expire(&mut self, clock: Clock, now: u64) {
        let mut changed = false;
        for slot in &mut self.chord {
            if slot.is_some_and(|n| clock.frame_of(n.end) <= now) {
                *slot = None;
                changed = true;
            }
        }
        if changed {
            self.refresh_chord();
        }
        for owner in &mut self.owners {
            if owner.is_some_and(|o| o.generated.is_none() && clock.frame_of(o.note.end) <= now) {
                self.held.note_off(owner.unwrap().note.key);
                *owner = None;
            }
        }
    }

    pub fn schedule_echo(&mut self, note: Note, duration: usize, rate: f64, bpm: f64) {
        let first = self.echo.next_id();
        if !self.echo.schedule_note(
            HeldNote {
                key: note.key,
                velocity: note.velocity,
            },
            0,
            duration,
            rate,
            bpm,
        ) {
            self.rejected_echoes = self.rejected_echoes.saturating_add(1);
            return;
        }
        let count = self.echo.next_id().wrapping_sub(first);
        for offset in 0..count {
            // One metadata slot per queued pair. Released pairs clear their slot.
            if let Some(slot) = self.echo_sources.iter_mut().find(|s| s.is_none()) {
                *slot = Some((first.wrapping_add(offset), note));
            }
        }
    }

    pub fn source(&self, generator: Generator, event: NoteEvent) -> Option<Note> {
        match generator {
            Generator::Arp => self
                .chord
                .iter()
                .flatten()
                .find(|note| {
                    event.note.velocity == note.velocity
                        && event.note.key >= note.key
                        && (event.note.key - note.key).is_multiple_of(12)
                        && event.note.key - note.key <= 36
                })
                .copied(),
            Generator::Echo => self
                .echo_sources
                .iter()
                .flatten()
                .find(|(id, _)| *id == event.id)
                .map(|(_, note)| *note),
        }
    }

    pub fn forget_echo(&mut self, id: u64) {
        for slot in &mut self.echo_sources {
            if slot.is_some_and(|(known, _)| known == id) {
                *slot = None;
            }
        }
    }

    pub fn rebind(&mut self, channel: usize) {
        for note in self.chord.iter_mut().flatten() {
            note.channel = channel;
        }
        for (_, note) in self.echo_sources.iter_mut().flatten() {
            note.channel = channel;
        }
        for owner in self.owners.iter_mut().flatten() {
            owner.note.channel = channel;
        }
    }

    pub fn move_ends(&mut self, moved: &impl Fn(f64) -> f64) {
        for note in self
            .chord
            .iter_mut()
            .flatten()
            .chain(self.echo_sources.iter_mut().flatten().map(|(_, note)| note))
        {
            note.end = moved(note.end);
            if let Some(source) = &mut note.source {
                source.move_clock(moved);
            }
        }
        for owner in self.owners.iter_mut().flatten() {
            owner.note.end = moved(owner.note.end);
            if let Some(source) = &mut owner.note.source {
                source.move_clock(moved);
            }
        }
    }

    /// Flush transport-owned scheduling without retiring notes held by hand.
    pub fn end_sequenced(&mut self) {
        self.arp.reset();
        self.echo.reset();
        self.echo_sources.fill(None);
        for note in &mut self.chord {
            if note.is_some_and(|note| note.origin == Origin::Sequenced) {
                *note = None;
            }
        }
        self.refresh_chord();
        self.held.clear();
        for owner in self.owners.iter().flatten() {
            admit_note(&mut self.held, owner.note.key, self.settings.polyphony);
        }
    }

    pub fn reset(&mut self) {
        self.arp.reset();
        self.arp.set_chord(&[]);
        self.echo.reset();
        self.held.clear();
        self.chord.fill(None);
        self.owners.fill(None);
        self.echo_sources.fill(None);
        self.envelopes.reset();
    }
}
