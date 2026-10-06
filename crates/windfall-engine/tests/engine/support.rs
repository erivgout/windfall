//! Shared helpers: building small projects and running a processor.

use windfall_core::{AudioBuffer, TICKS_PER_STEP};
use windfall_engine::{Controller, Processor, SamplePool};
use windfall_project::{
    Channel, ChannelId, ChannelSource, Clip, ClipContent, ClipId, DEFAULT_KEY, Lane, MixerTrack,
    Note, NoteId, Pattern, PatternId, PlaylistTrack, PlaylistTrackId, Project, SampleAsset,
    SampleId, SamplePath, SamplerSettings, TrackId,
};

/// A project under construction together with its decoded samples.
///
/// Everything starts at unity: channel volume, track volume and velocity
/// are 1 and pans are centered, so a sample comes out of the master exactly
/// as it went in unless a test changes something.
pub struct Rig {
    pub project: Project,
    pub pool: SamplePool,
}

impl Rig {
    pub fn new() -> Self {
        Self {
            project: Project::new("test"),
            pool: SamplePool::new(),
        }
    }

    fn id(&mut self) -> u32 {
        self.project.next_id += 1;
        self.project.next_id - 1
    }

    pub fn first_pattern(&self) -> PatternId {
        self.project.patterns[0].id
    }

    pub fn sample(&mut self, audio: AudioBuffer) -> SampleId {
        let id = SampleId(self.id());
        self.pool.insert(id, audio);
        self.project.samples.push(SampleAsset {
            id,
            name: format!("sample {}", id.0),
            path: SamplePath::Project(format!("{}.wav", id.0)),
        });
        id
    }

    /// Adds a mixer track that plays into the master.
    pub fn track(&mut self) -> TrackId {
        let id = TrackId(self.id());
        self.project.mixer.tracks.push(MixerTrack {
            id,
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            output: Some(TrackId::MASTER),
            sends: Vec::new(),
        });
        id
    }

    /// Adds a channel that plays `audio` into `track`.
    pub fn channel_on(&mut self, audio: AudioBuffer, track: TrackId) -> ChannelId {
        let sample = self.sample(audio);
        let id = ChannelId(self.id());
        self.project.channels.push(Channel {
            id,
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: track,
            source: ChannelSource::Sampler(SamplerSettings {
                sample: Some(sample),
                ..SamplerSettings::default()
            }),
        });
        id
    }

    /// Adds a channel that plays `audio` straight into the master.
    pub fn channel(&mut self, audio: AudioBuffer) -> ChannelId {
        self.channel_on(audio, TrackId::MASTER)
    }

    pub fn channel_mut(&mut self, id: ChannelId) -> &mut Channel {
        self.project
            .channels
            .iter_mut()
            .find(|channel| channel.id == id)
            .expect("the channel exists")
    }

    pub fn sampler_mut(&mut self, id: ChannelId) -> &mut SamplerSettings {
        let ChannelSource::Sampler(settings) = &mut self.channel_mut(id).source;
        settings
    }

    pub fn track_mut(&mut self, id: TrackId) -> &mut MixerTrack {
        self.project
            .mixer
            .tracks
            .iter_mut()
            .find(|track| track.id == id)
            .expect("the track exists")
    }

    pub fn pattern_mut(&mut self, id: PatternId) -> &mut Pattern {
        self.project
            .patterns
            .iter_mut()
            .find(|pattern| pattern.id == id)
            .expect("the pattern exists")
    }

    pub fn pattern(&mut self, length_steps: u32) -> PatternId {
        let id = PatternId(self.id());
        self.project.patterns.push(Pattern {
            id,
            name: String::new(),
            color: 0,
            length_steps,
            lanes: Vec::new(),
        });
        id
    }

    /// Adds a note to a pattern and returns it for further changes. The
    /// note has the default key, full velocity and no pan.
    pub fn note_in(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        start: u32,
        length: u32,
    ) -> &mut Note {
        let id = NoteId(self.id());
        let pattern = self.pattern_mut(pattern);
        let lane = match pattern
            .lanes
            .iter()
            .position(|lane| lane.channel == channel)
        {
            Some(index) => index,
            None => {
                pattern.lanes.push(Lane {
                    channel,
                    notes: Vec::new(),
                });
                pattern.lanes.sort_by_key(|lane| lane.channel);
                pattern
                    .lanes
                    .iter()
                    .position(|lane| lane.channel == channel)
                    .expect("the lane was just added")
            }
        };
        let notes = &mut pattern.lanes[lane].notes;
        notes.push(Note {
            id,
            start,
            length,
            key: DEFAULT_KEY,
            velocity: 1.0,
            pan: 0.0,
        });
        notes.sort_by_key(Note::sort_key);
        notes
            .iter_mut()
            .find(|note| note.id == id)
            .expect("the note was just added")
    }

    /// Adds a note to the first pattern.
    pub fn note(&mut self, channel: ChannelId, start: u32, length: u32) -> &mut Note {
        self.note_in(self.first_pattern(), channel, start, length)
    }

    /// Lights steps of the first pattern's step sequencer.
    pub fn steps(&mut self, channel: ChannelId, steps: &[u32]) {
        for step in steps {
            self.note(channel, step * TICKS_PER_STEP, TICKS_PER_STEP);
        }
    }

    pub fn playlist_track(&mut self) -> PlaylistTrackId {
        let id = PlaylistTrackId(self.id());
        self.project.playlist.tracks.push(PlaylistTrack {
            id,
            name: String::new(),
            muted: false,
        });
        id
    }

    pub fn clip(
        &mut self,
        track: PlaylistTrackId,
        pattern: PatternId,
        start: u32,
        length: u32,
    ) -> &mut Clip {
        let id = ClipId(self.id());
        let clips = &mut self.project.playlist.clips;
        clips.push(Clip {
            id,
            track,
            start,
            length,
            offset: 0,
            muted: false,
            content: ClipContent::Pattern { pattern },
        });
        clips.sort_by_key(Clip::sort_key);
        clips
            .iter_mut()
            .find(|clip| clip.id == id)
            .expect("the clip was just added")
    }

    /// A processor with this project loaded, not yet playing.
    pub fn processor(&self, sample_rate: u32) -> (Processor, Controller) {
        let (processor, controller) = Processor::new(sample_rate);
        controller.set_project(&self.project, &self.pool);
        (processor, controller)
    }

    /// Plays the project from the top for `frames` frames, `block` frames
    /// per call, and returns the interleaved stereo output.
    pub fn play(&self, sample_rate: u32, frames: usize, block: usize) -> Vec<f32> {
        let (mut processor, controller) = self.processor(sample_rate);
        controller.play();
        run(&mut processor, frames, block)
    }
}

/// Processes `frames` frames, `block` frames per call.
pub fn run(processor: &mut Processor, frames: usize, block: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    for chunk in out.chunks_mut(block * 2) {
        processor.process(chunk);
    }
    out
}

/// A single full-scale mono sample. Played at its own rate it marks exactly
/// one output frame.
pub fn impulse(sample_rate: u32) -> AudioBuffer {
    AudioBuffer::from_interleaved(sample_rate, 1, vec![1.0])
}

/// A mono buffer holding one constant value.
pub fn level(sample_rate: u32, value: f32, seconds: f64) -> AudioBuffer {
    let frames = (seconds * f64::from(sample_rate)).round() as usize;
    AudioBuffer::from_interleaved(sample_rate, 1, vec![value; frames])
}

/// A mono sine wave at half scale.
pub fn sine(sample_rate: u32, frequency: f64, seconds: f64) -> AudioBuffer {
    let frames = (seconds * f64::from(sample_rate)).round() as usize;
    let data = (0..frames)
        .map(|frame| {
            let phase = frame as f64 * frequency / f64::from(sample_rate);
            ((phase * std::f64::consts::TAU).sin() * 0.5) as f32
        })
        .collect();
    AudioBuffer::from_interleaved(sample_rate, 1, data)
}

/// The frames of an interleaved stereo buffer where either side is not zero.
pub fn sounding_frames(audio: &[f32]) -> Vec<usize> {
    frames(audio)
        .iter()
        .enumerate()
        .filter(|(_, frame)| frame[0] != 0.0 || frame[1] != 0.0)
        .map(|(index, _)| index)
        .collect()
}

/// The left side of an interleaved stereo buffer.
pub fn left(audio: &[f32]) -> Vec<f32> {
    frames(audio).iter().map(|frame| frame[0]).collect()
}

/// The right side of an interleaved stereo buffer.
pub fn right(audio: &[f32]) -> Vec<f32> {
    frames(audio).iter().map(|frame| frame[1]).collect()
}

/// An interleaved stereo buffer as left and right pairs.
fn frames(audio: &[f32]) -> &[[f32; 2]] {
    audio.as_chunks().0
}

/// The largest jump between neighbouring samples.
pub fn largest_step(signal: &[f32]) -> f32 {
    signal
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

pub fn peak(signal: &[f32]) -> f32 {
    signal
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

/// The frame a note at `tick` must land on: the first frame at or after the
/// tick's time, worked out in whole numbers. The tempo is `bpm_times_100`
/// hundredths of a beat per minute.
pub fn frame_of_tick(tick: u64, bpm_times_100: u64, sample_rate: u32) -> usize {
    // frames = tick * sample_rate * 60 / (bpm * 960)
    let numerator = u128::from(tick) * u128::from(sample_rate) * 60 * 100;
    let denominator = u128::from(bpm_times_100) * 960;
    numerator.div_ceil(denominator) as usize
}

/// Frequency of a signal in hertz, from the average spacing of its upward
/// zero crossings.
pub fn frequency(signal: &[f32], sample_rate: u32) -> f64 {
    let crossings: Vec<f64> = signal
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, pair)| {
            // Where the line between the two samples crosses zero.
            index as f64 + f64::from(-pair[0]) / f64::from(pair[1] - pair[0])
        })
        .collect();
    assert!(crossings.len() > 2, "the signal does not oscillate");
    let cycles = (crossings.len() - 1) as f64;
    let span = crossings[crossings.len() - 1] - crossings[0];
    cycles * f64::from(sample_rate) / span
}
