//! Exporting a project built with real commands: what a pattern and a
//! playlist become as a song.

use windfall_midi::*;
use windfall_project::{
    AutomationId, AutomationPoint, AutomationRange, AutomationTarget, ChannelId, ChannelPatch,
    ClipContent, ClipId, ClipInit, ClipPatch, ClipUpdate, Command, Document, InstrumentKind,
    MAX_SONG_TICKS, NoteInit, PatternId, PatternPatch, PlaylistTrackId, PlaylistTrackPatch,
    Project, SettingsPatch, TimeSignature,
};

const FIRST_PATTERN: PatternId = PatternId(1);

/// A document and shorthand for the commands the tests build projects
/// with.
struct Session {
    document: Document,
}

impl Session {
    fn new() -> Self {
        Self {
            document: Document::new(Project::new("Test")),
        }
    }

    fn project(&self) -> &Project {
        self.document.project()
    }

    /// Dispatches a command that must succeed and returns the first id it
    /// made, or 0.
    #[track_caller]
    fn run(&mut self, command: Command) -> u32 {
        let applied = match self.document.dispatch(command.clone(), None) {
            Ok(applied) => applied,
            Err(error) => panic!("{command:?} failed: {error}"),
        };
        self.document
            .project()
            .check()
            .expect("the project is valid");
        applied.created.first().copied().unwrap_or(0)
    }

    fn settings(&mut self, patch: SettingsPatch) {
        self.run(Command::UpdateSettings { patch });
    }

    fn synth(&mut self, name: &str) -> ChannelId {
        ChannelId(self.run(Command::AddChannel {
            name: Some(name.to_owned()),
            sample: None,
            instrument: Some(InstrumentKind::SubtractiveSynth),
            index: None,
            mixer_track: None,
        }))
    }

    fn pattern(&mut self, name: &str, length_steps: u32) -> PatternId {
        let id = PatternId(self.run(Command::AddPattern {
            name: Some(name.to_owned()),
        }));
        self.run(Command::UpdatePattern {
            id,
            patch: PatternPatch {
                length_steps: Some(length_steps),
                ..PatternPatch::default()
            },
        });
        id
    }

    /// Adds notes given as (start, length, key), at full velocity.
    fn notes(&mut self, pattern: PatternId, channel: ChannelId, notes: &[(u32, u32, u8)]) {
        let notes = notes
            .iter()
            .map(|&(start, length, key)| NoteInit {
                start,
                length,
                key,
                velocity: Some(1.0),
                pan: None,
            })
            .collect();
        self.run(Command::AddNotes {
            pattern,
            channel,
            notes,
        });
    }

    fn track(&mut self) -> PlaylistTrackId {
        PlaylistTrackId(self.run(Command::AddPlaylistTrack {
            name: None,
            index: None,
        }))
    }

    fn clip(
        &mut self,
        track: PlaylistTrackId,
        content: ClipContent,
        start: u32,
        length: u32,
        offset: u32,
    ) -> ClipId {
        ClipId(self.run(Command::AddClips {
            clips: vec![ClipInit {
                track,
                start,
                length: Some(length),
                offset: Some(offset),
                muted: None,
                content,
            }],
        }))
    }

    fn pattern_clip(
        &mut self,
        track: PlaylistTrackId,
        pattern: PatternId,
        start: u32,
        length: u32,
        offset: u32,
    ) -> ClipId {
        self.clip(
            track,
            ClipContent::Pattern { pattern },
            start,
            length,
            offset,
        )
    }

    /// Adds a tempo automation with points given as (tick, bpm, hold).
    fn tempo_curve(&mut self, points: &[(u32, f32, bool)]) -> AutomationId {
        self.automation(AutomationTarget::Tempo, points)
    }

    fn automation(
        &mut self,
        target: AutomationTarget,
        points: &[(u32, f32, bool)],
    ) -> AutomationId {
        let range = self.project().automation_range(&target).expect("a target");
        let points = points
            .iter()
            .map(|&(tick, value, hold)| AutomationPoint {
                tick,
                value: range.normalized(value),
                curve: 0.0,
                hold,
            })
            .collect();
        AutomationId(self.run(Command::AddAutomation {
            name: None,
            target,
            points: Some(points),
        }))
    }

    fn mute_clip(&mut self, id: ClipId) {
        self.run(Command::UpdateClips {
            updates: vec![ClipUpdate {
                id,
                patch: ClipPatch {
                    muted: Some(true),
                    ..ClipPatch::default()
                },
            }],
        });
    }

    fn mute_track(&mut self, id: PlaylistTrackId) {
        self.run(Command::UpdatePlaylistTrack {
            id,
            patch: PlaylistTrackPatch {
                muted: Some(true),
                ..PlaylistTrackPatch::default()
            },
        });
    }

    #[track_caller]
    fn song(&self) -> MidiSong {
        self.song_with(&ExportOptions::default())
    }

    #[track_caller]
    fn song_with(&self, options: &ExportOptions) -> MidiSong {
        let song = export_song(self.project(), options).expect("the song exports");
        assert_eq!(song.clone().normalized(), song);
        song
    }

    /// The tempo events of the exported song as (tick, microseconds per
    /// quarter note).
    #[track_caller]
    fn tempos(&self) -> Vec<(u32, u32)> {
        let song = self.song();
        let tempos = song.tempos.iter();
        tempos
            .map(|tempo| (tempo.tick, tempo.micros_per_quarter))
            .collect()
    }
}

/// The notes of a track as (start, length, key).
fn notes_of(track: &MidiTrack) -> Vec<(u32, u32, u8)> {
    let notes = track.notes.iter();
    notes
        .map(|note| (note.start, note.length, note.key))
        .collect()
}

fn micros(bpm: f64) -> u32 {
    micros_per_quarter(bpm)
}

#[test]
fn a_clip_at_song_start_can_skip_into_and_loop_a_pattern() {
    let mut session = Session::new();
    let channel = session.synth("Lead");
    session.notes(FIRST_PATTERN, channel, &[(0, 120, 60), (480, 120, 62)]);
    let track = session.track();
    session.pattern_clip(track, FIRST_PATTERN, 0, 4_000, 240);

    let song = export_song(session.project(), &ExportOptions::default()).expect("it exports");
    let notes: Vec<_> = song.tracks[0]
        .notes
        .iter()
        .map(|note| (note.start, note.length, note.key))
        .collect();
    assert_eq!(notes, [(240, 120, 62), (3_600, 120, 60)]);
}

#[test]
fn a_pattern_becomes_a_song_of_its_own_length() {
    let mut session = Session::new();
    session.settings(SettingsPatch {
        tempo_bpm: Some(96.0),
        time_signature: Some(TimeSignature {
            numerator: 3,
            denominator: 8,
        }),
        ..SettingsPatch::default()
    });
    let lead = session.synth("Lead");
    // A channel with no notes in the pattern gets no track.
    session.synth("Silent");
    let kick = ChannelId(session.run(Command::AddChannel {
        name: Some("Kick".to_owned()),
        sample: None,
        instrument: None,
        index: None,
        mixer_track: None,
    }));
    session.run(Command::AddNotes {
        pattern: FIRST_PATTERN,
        channel: lead,
        notes: vec![
            NoteInit {
                start: 0,
                length: 480,
                key: 60,
                velocity: Some(0.5),
                pan: Some(-1.0),
            },
            // Lasts past the end of the pattern, where it is cut.
            NoteInit {
                start: 3_600,
                length: 480,
                key: 62,
                velocity: Some(1.0),
                pan: None,
            },
            // Starts at the end of the pattern, so it never plays.
            NoteInit {
                start: 3_840,
                length: 100,
                key: 64,
                velocity: None,
                pan: None,
            },
        ],
    });
    session.run(Command::ToggleStep {
        pattern: FIRST_PATTERN,
        channel: kick,
        step: 1,
    });

    let options = ExportOptions::default();
    let song = export_pattern(session.project(), FIRST_PATTERN, &options).expect("it exports");
    assert_eq!(song.name.as_deref(), Some("Pattern 1"));
    assert_eq!(song.length, 3_840);
    assert_eq!(
        song.tempos,
        [TempoChange {
            tick: 0,
            micros_per_quarter: 625_000
        }]
    );
    assert_eq!(
        song.time_signatures,
        [TimeSignatureChange {
            tick: 0,
            numerator: 3,
            denominator: 8
        }]
    );
    let names: Vec<_> = song.tracks.iter().map(|t| t.name.as_deref()).collect();
    assert_eq!(names, [Some("Lead"), Some("Kick")]);
    let note = |start, length, key, velocity, channel| MidiNote {
        start,
        length,
        key,
        velocity,
        release: DEFAULT_RELEASE,
        channel,
    };
    assert_eq!(
        song.tracks[0].notes,
        [note(0, 480, 60, 64, 0), note(3_600, 240, 62, 127, 0)]
    );
    assert_eq!(song.tracks[1].notes, [note(240, 240, 60, 102, 1)]);
    assert!(song.tracks.iter().all(|track| track.controls.is_empty()));
    assert_eq!(song.clone().normalized(), song);

    assert_eq!(
        export_pattern(session.project(), PatternId(999), &options),
        Err(ExportError::PatternNotFound(PatternId(999)))
    );
    let error = ExportError::PatternNotFound(PatternId(999)).to_string();
    assert_eq!(error, "pattern 999 does not exist");
}

#[test]
fn swing_moves_the_notes_as_it_does_when_the_project_plays() {
    let mut session = Session::new();
    let lead = session.synth("Lead");
    // Steps 0, 1, 2 and 3 of a five-step pattern, whose last step has no
    // partner to swing against.
    let pattern = session.pattern("Swung", 5);
    session.notes(
        pattern,
        lead,
        &[
            (0, 240, 60),
            (240, 240, 61),
            (480, 240, 62),
            (720, 120, 63),
            (960, 240, 64),
        ],
    );
    let export = |session: &Session, swing: bool| {
        let options = ExportOptions {
            swing,
            ..ExportOptions::default()
        };
        let song = export_pattern(session.project(), pattern, &options).expect("it exports");
        notes_of(&song.tracks[0])
    };
    let straight = [
        (0, 240, 60),
        (240, 240, 61),
        (480, 240, 62),
        (720, 120, 63),
        (960, 240, 64),
    ];
    assert_eq!(export(&session, true), straight);

    session.settings(SettingsPatch {
        swing: Some(1.0),
        ..SettingsPatch::default()
    });
    // Full swing puts the second step of a pair two thirds of the way
    // through the pair.
    assert_eq!(
        export(&session, true),
        [
            (0, 320, 60),
            (320, 160, 61),
            (480, 320, 62),
            (800, 80, 63),
            (960, 240, 64),
        ]
    );
    assert_eq!(export(&session, false), straight);

    session.settings(SettingsPatch {
        swing: Some(0.5),
        ..SettingsPatch::default()
    });
    assert_eq!(export(&session, true)[1], (280, 200, 61));
}

#[test]
fn a_clip_is_a_window_onto_a_pattern_that_goes_around() {
    let mut session = Session::new();
    let lead = session.synth("Lead");
    // The second note lasts past the end of the pattern.
    session.notes(FIRST_PATTERN, lead, &[(0, 480, 60), (1_920, 2_400, 62)]);
    let upper = session.track();
    let lower = session.track();

    // One pass: the long note is cut where the clip ends.
    session.pattern_clip(upper, FIRST_PATTERN, 0, 3_840, 0);
    // Two passes that begin half way into the pattern.
    session.pattern_clip(upper, FIRST_PATTERN, 7_680, 7_680, 1_920);
    // An offset past the end of the pattern goes around it.
    session.pattern_clip(upper, FIRST_PATTERN, 30_000, 960, 3_840 * 5 + 1_900);
    let muted = session.pattern_clip(upper, FIRST_PATTERN, 40_000, 3_840, 0);
    session.mute_clip(muted);
    session.pattern_clip(lower, FIRST_PATTERN, 50_000, 3_840, 0);
    session.mute_track(lower);

    let song = session.song();
    assert_eq!(song.name.as_deref(), Some("Test"));
    assert_eq!(song.tracks.len(), 1);
    assert_eq!(
        notes_of(&song.tracks[0]),
        [
            (0, 480, 60),
            (1_920, 1_920, 62),
            (7_680, 2_400, 62),
            (9_600, 480, 60),
            (11_520, 2_400, 62),
            (13_440, 480, 60),
            // 20 ticks into the clip the long note begins, and the clip
            // ends 940 ticks later.
            (30_020, 940, 62),
        ]
    );
    // Muted clips still say how long the song is.
    assert_eq!(song.length, 53_840);
}

#[test]
fn every_channel_that_plays_is_a_track_in_rack_order() {
    let mut session = Session::new();
    let track = session.track();
    let channels: Vec<ChannelId> = (0..17)
        .map(|index| session.synth(&format!("Part {index}")))
        .collect();
    for (index, &channel) in channels.iter().enumerate() {
        // The sixth channel plays nothing.
        if index != 5 {
            session.notes(FIRST_PATTERN, channel, &[(0, 240, 40 + index as u8)]);
        }
    }
    // The last channel is moved to the top of the rack.
    session.run(Command::MoveChannel {
        id: channels[16],
        index: 0,
    });
    session.pattern_clip(track, FIRST_PATTERN, 0, 3_840, 0);

    let song = session.song();
    assert_eq!(song.tracks.len(), 16);
    assert_eq!(song.tracks[0].name.as_deref(), Some("Part 16"));
    assert_eq!(song.tracks[1].name.as_deref(), Some("Part 0"));
    assert_eq!(song.tracks[6].name.as_deref(), Some("Part 6"));
    let midi_channels: Vec<u8> = song.tracks.iter().map(|t| t.notes[0].channel).collect();
    // Channel 10, which is 9 counted from 0, is left for drums.
    assert_eq!(
        midi_channels,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 0]
    );
}

#[test]
fn a_volume_or_pan_that_is_not_the_default_is_written_as_a_controller() {
    let mut session = Session::new();
    let track = session.track();
    let loud = session.synth("Loud");
    let plain = session.synth("Plain");
    for channel in [loud, plain] {
        session.notes(FIRST_PATTERN, channel, &[(0, 240, 60)]);
    }
    session.run(Command::UpdateChannel {
        id: loud,
        patch: ChannelPatch {
            volume: Some(1.0),
            pan: Some(-0.5),
            ..ChannelPatch::default()
        },
    });
    session.pattern_clip(track, FIRST_PATTERN, 0, 3_840, 0);

    let song = session.song();
    let value = |track: &MidiTrack, controller| {
        let curve = track.control(track.notes[0].channel, ControlKind::Controller(controller));
        curve.map(|curve| (curve.points[0].tick, curve.points[0].value))
    };
    // 100 * sqrt(1 / 0.8) is 111.8.
    assert_eq!(value(&song.tracks[0], CC_VOLUME), Some((0, 112)));
    assert_eq!(value(&song.tracks[0], CC_PAN), Some((0, 32)));
    assert!(song.tracks[1].controls.is_empty());

    let bare = ExportOptions {
        channel_mix: false,
        ..ExportOptions::default()
    };
    let song = session.song_with(&bare);
    assert!(song.tracks.iter().all(|track| track.controls.is_empty()));
}

#[test]
fn a_project_with_no_tempo_automation_has_one_tempo() {
    let mut session = Session::new();
    assert_eq!(session.tempos(), [(0, 500_000)]);
    session.settings(SettingsPatch {
        tempo_bpm: Some(87.5),
        ..SettingsPatch::default()
    });
    assert_eq!(session.tempos(), [(0, micros(87.5))]);
    let song = session.song();
    assert!(song.tracks.is_empty());
    assert_eq!(song.length, 0);
}

#[test]
fn tempo_steps_are_written_on_their_ticks() {
    let mut session = Session::new();
    let track = session.track();
    let curve = session.tempo_curve(&[(0, 100.0, true), (1_920, 150.0, true), (3_840, 60.0, true)]);
    let automation = ClipContent::Automation { automation: curve };
    session.clip(track, automation.clone(), 0, 7_680, 0);
    assert_eq!(
        session.tempos(),
        [(0, 600_000), (1_920, 400_000), (3_840, 1_000_000)]
    );

    // A second clip shows the curve from its second point on, later in
    // the song. Between the clips the tempo holds what the first ended on.
    session.clip(track, automation, 9_600, 3_840, 1_920);
    assert_eq!(
        session.tempos(),
        [
            (0, 600_000),
            (1_920, 400_000),
            (3_840, 1_000_000),
            (9_600, 400_000),
            (11_520, 1_000_000),
        ]
    );
}

#[test]
fn before_its_first_clip_the_tempo_is_the_projects() {
    let mut session = Session::new();
    let track = session.track();
    let curve = session.tempo_curve(&[(0, 150.0, true), (960, 75.0, true)]);
    let automation = ClipContent::Automation { automation: curve };
    // The clip is cut before the curve's second point.
    session.clip(track, automation.clone(), 3_840, 480, 0);
    session.clip(track, automation, 7_680, 1_920, 0);
    assert_eq!(
        session.tempos(),
        [(0, 500_000), (3_840, 400_000), (8_640, 800_000),]
    );
}

#[test]
fn a_tempo_that_glides_is_written_as_steps_at_the_tempo_of_their_middle() {
    let mut session = Session::new();
    let track = session.track();
    let curve = session.tempo_curve(&[
        (0, 100.0, false),
        (1_920, 200.0, false),
        (2_880, 200.0, false),
    ]);
    session.clip(
        track,
        ClipContent::Automation { automation: curve },
        0,
        7_680,
        0,
    );
    let options = ExportOptions {
        tempo_step: 480,
        ..ExportOptions::default()
    };
    let song = session.song_with(&options);
    let tempos: Vec<_> = song
        .tempos
        .iter()
        .map(|tempo| (tempo.tick, tempo.micros_per_quarter))
        .collect();
    assert_eq!(
        tempos,
        [
            (0, micros(112.5)),
            (480, micros(137.5)),
            (960, micros(162.5)),
            (1_440, micros(187.5)),
            (1_920, micros(200.0)),
        ]
    );

    // The steps of the default are a sixteenth note long, and a step that
    // does not fit before the next point is shorter.
    let song = session.song();
    assert_eq!(song.tempos.len(), 9);
    assert_eq!(song.tempos[1].tick, 240);
    let odd = ExportOptions {
        tempo_step: 1_000,
        ..ExportOptions::default()
    };
    let song = session.song_with(&odd);
    let ticks: Vec<_> = song.tempos.iter().map(|tempo| tempo.tick).collect();
    assert_eq!(ticks, [0, 1_000, 1_920]);
    assert_eq!(
        song.tempos[1].micros_per_quarter,
        micros(100.0 + 100.0 * 1_460.0 / 1_920.0)
    );
    // A step of nothing counts as one tick.
    let finest = ExportOptions {
        tempo_step: 0,
        ..ExportOptions::default()
    };
    let song = session.song_with(&finest);
    assert!(song.tempos.len() > 1_000);
    assert!(
        song.tempos
            .windows(2)
            .all(|pair| pair[0].tick < pair[1].tick)
    );
}

#[test]
fn a_jump_of_the_tempo_curve_is_one_tempo_event() {
    let mut session = Session::new();
    let track = session.track();
    let curve = session.tempo_curve(&[
        (0, 100.0, false),
        (960, 100.0, false),
        (960, 150.0, false),
        (1_920, 150.0, false),
    ]);
    session.clip(
        track,
        ClipContent::Automation { automation: curve },
        0,
        3_840,
        0,
    );
    assert_eq!(session.tempos(), [(0, 600_000), (960, 400_000)]);
}

#[test]
fn where_tempo_clips_overlap_the_one_the_engine_plays_is_written() {
    let mut session = Session::new();
    let upper = session.track();
    let lower = session.track();
    let steady = |session: &mut Session, bpm| {
        let automation = session.tempo_curve(&[(0, bpm, true)]);
        ClipContent::Automation { automation }
    };
    let slow = steady(&mut session, 60.0);
    let fast = steady(&mut session, 150.0);
    let faster = steady(&mut session, 200.0);
    let fastest = steady(&mut session, 250.0);

    // The lower track's clip gives way to the upper track's for as long as
    // that one lasts.
    session.clip(lower, slow, 0, 7_680, 0);
    session.clip(upper, fast, 1_920, 1_920, 0);
    // Of two clips on one track the one that starts later has its way.
    session.clip(upper, faster.clone(), 9_600, 3_840, 0);
    session.clip(upper, fastest, 11_520, 960, 0);
    assert_eq!(
        session.tempos(),
        [
            (0, micros(60.0)),
            (1_920, micros(150.0)),
            (3_840, micros(60.0)),
            (9_600, micros(200.0)),
            (11_520, micros(250.0)),
            (12_480, micros(200.0)),
        ]
    );

    // A muted clip and a clip on a muted track count for nothing.
    let muted = session.clip(upper, faster.clone(), 20_000, 960, 0);
    session.mute_clip(muted);
    let silent = session.track();
    session.clip(silent, faster, 21_000, 960, 0);
    session.mute_track(silent);
    assert_eq!(session.tempos().len(), 6);
    assert_eq!(session.song().length, 21_960);
}

#[test]
fn automation_of_anything_but_the_tempo_is_not_in_the_file() {
    let mut session = Session::new();
    let track = session.track();
    let lead = session.synth("Lead");
    let volume = AutomationTarget::ChannelVolume { channel: lead };
    let automation = session.automation(volume, &[(0, 0.0, false), (3_840, 2.0, false)]);
    session.clip(track, ClipContent::Automation { automation }, 0, 3_840, 0);
    let song = session.song();
    assert_eq!(song.tempos.len(), 1);
    assert!(song.tracks.is_empty());
    assert_eq!(song.length, 3_840);
    assert_eq!(AutomationRange::GAIN.value(1.0), 2.0);
}

#[test]
fn a_song_that_plays_more_notes_than_an_export_holds_is_refused() {
    let mut session = Session::new();
    let track = session.track();
    let pattern = session.pattern("Tick", 1);
    let first = session.synth("One");
    let second = session.synth("Two");
    session.notes(pattern, first, &[(0, 240, 60)]);
    session.notes(pattern, second, &[(0, 240, 60)]);
    // A one-step pattern, around four million times.
    session.pattern_clip(track, pattern, 0, MAX_SONG_TICKS, 0);
    let refused = export_song(session.project(), &ExportOptions::default());
    assert_eq!(refused, Err(ExportError::TooManyNotes));
    assert!(
        ExportError::TooManyNotes
            .to_string()
            .contains("4000000 notes")
    );
}

#[test]
fn an_exported_song_survives_being_written_and_read() {
    let mut session = Session::new();
    let track = session.track();
    let lead = session.synth("Lead");
    session.notes(
        FIRST_PATTERN,
        lead,
        &[(0, 480, 60), (480, 480, 60), (1_000, 77, 72)],
    );
    session.pattern_clip(track, FIRST_PATTERN, 0, 7_680, 0);
    let song = session.song();
    for format in [SmfFormat::Multi, SmfFormat::Single] {
        let options = WriteOptions {
            format,
            ..WriteOptions::default()
        };
        let back = read(&write(&song, &options)).expect("the file is valid");
        let expected = match format {
            SmfFormat::Multi => song.clone(),
            SmfFormat::Single => song.flattened().normalized(),
        };
        assert_eq!(back, expected);
    }
}
