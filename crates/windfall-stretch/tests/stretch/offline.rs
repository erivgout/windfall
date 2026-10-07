//! Stretching a whole buffer: its length, its edges and its channels.

use windfall_core::AudioBuffer;
use windfall_stretch::{Quality, stretch, stretched_frames};

use crate::quality::middle;
use crate::support::{
    RATE, Rng, cents, chord, clicks, db, drum_loop, frequency_near, noise, offline_mono, peak, rms,
    sine, stream_mono, tone_level,
};

#[test]
fn the_output_is_exactly_as_long_as_asked() {
    let mut rng = Rng::new(31);
    for round in 0..40 {
        let frames = match round {
            0 => 0,
            1 => 1,
            2 => 7,
            _ => rng.between(2, 60_000),
        };
        let ratio = 0.25 + f64::from(rng.unipolar()) * 3.75;
        let pitch = if round % 3 == 0 {
            0.0
        } else {
            f64::from(rng.bipolar()) * 12.0
        };
        let channels = 1 + round as u16 % 3;
        let rate = [44_100, 48_000, 22_050][round % 3];
        let data = noise(round as u32, 0.3, frames * usize::from(channels));
        let buffer = AudioBuffer::from_interleaved(rate, channels, data);
        let quality = Quality::ALL[round % 3];
        let output = stretch(&buffer, ratio, pitch, quality);
        let wanted = (frames as f64 * ratio).round() as usize;
        assert_eq!(output.frames(), wanted, "{frames} frames at {ratio}");
        assert_eq!(stretched_frames(frames, ratio), wanted);
        assert_eq!((output.sample_rate(), output.channels()), (rate, channels));
        assert!(output.samples().iter().all(|sample| sample.is_finite()));
    }
}

#[test]
fn ratios_out_of_range_are_forced_into_it() {
    let buffer = AudioBuffer::from_interleaved(RATE, 1, sine(440.0, 0.5, 20_000, RATE));
    assert_eq!(stretch(&buffer, 0.0, 0.0, Quality::Fast).frames(), 5_000);
    assert_eq!(stretch(&buffer, 100.0, 0.0, Quality::Fast).frames(), 80_000);
    assert_eq!(
        stretch(&buffer, f64::NAN, f64::NAN, Quality::Fast).frames(),
        20_000
    );
    assert_eq!(stretched_frames(1_000, -1.0), 250);
    assert_eq!(stretched_frames(1_000, f64::INFINITY), 1_000);
    // A pitch out of range is two octaves.
    let up = stretch(&buffer, 1.0, 99.0, Quality::Fast);
    let found = frequency_near(middle(up.samples()), 1_760.0, 0.02, RATE);
    assert!(cents(found, 1_760.0).abs() < 1.0, "{found} Hz");
}

#[test]
fn unity_returns_the_audio_itself() {
    let data = noise(5, 0.7, 30_000);
    let buffer = AudioBuffer::from_interleaved(RATE, 2, data.clone());
    for quality in Quality::ALL {
        assert_eq!(stretch(&buffer, 1.0, 0.0, quality).samples(), &data[..]);
    }
}

#[test]
fn the_same_buffer_gives_the_same_result_every_time() {
    let buffer = AudioBuffer::from_interleaved(RATE, 2, noise(6, 0.4, 60_000));
    let once = stretch(&buffer, 2.7, -5.0, Quality::Standard);
    let again = stretch(&buffer, 2.7, -5.0, Quality::Standard);
    assert_eq!(once.samples(), again.samples());
}

/// The frame of the loudest sample of `signal` within `around` plus or
/// minus `reach`.
fn loudest(signal: &[f32], around: usize, reach: usize) -> usize {
    let range = around.saturating_sub(reach)..(around + reach).min(signal.len());
    range
        .max_by(|a, b| signal[*a].abs().total_cmp(&signal[*b].abs()))
        .unwrap_or(around)
}

#[test]
fn every_moment_of_the_input_lands_on_its_place_in_the_output() {
    let period = 12_000;
    let frames = 8 * period;
    // The first click is on the very first frame and the last one ends
    // with the buffer.
    let mut input = clicks(0, period, frames, RATE);
    let last = frames - 96;
    let end: Vec<f32> = input[..96].to_vec();
    input[last..].copy_from_slice(&end);
    let offsets: Vec<usize> = (0..8).map(|index| index * period).chain([last]).collect();
    for quality in Quality::ALL {
        for (ratio, pitch) in [
            (0.5, 0.0),
            (0.8, 0.0),
            (1.25, 0.0),
            (1.9, 0.0),
            (1.0, 5.0),
            (1.3, -2.0),
        ] {
            let output = offline_mono(&input, ratio, pitch, quality);
            for offset in &offsets {
                let place = (*offset as f64 * ratio).round() as usize;
                let found = loudest(&output, place, period / 4) as i64;
                let off = found - place as i64;
                // A click is a millisecond of noise that is loudest near
                // its start.
                assert!(
                    (-24..72).contains(&off),
                    "{quality:?} {ratio} {pitch}: click at {offset} is {off} frames off"
                );
            }
        }
    }
}

#[test]
fn a_loop_that_starts_on_a_hit_keeps_the_hit() {
    let input = drum_loop(120.0, 8, RATE);
    let first = rms(&input[..2_400]);
    for quality in Quality::ALL {
        for ratio in [0.8, 1.0 / 1.1, 1.2, 1.5] {
            let output = offline_mono(&input, ratio, 0.0, quality);
            // The kick is there from the first frames, not faded in, and
            // nothing was lost off the front.
            let level = db(rms(&output[..(2_400.0 * ratio) as usize]) / first);
            assert!(
                level.abs() < 1.5,
                "{quality:?} at {ratio}: the first hit is {level:+.2} dB"
            );
            let whole = db(rms(&output) / rms(&input));
            assert!(whole.abs() < 1.0, "{quality:?} at {ratio}: {whole:+.2} dB");
        }
    }
}

#[test]
fn a_sound_is_at_full_level_up_to_both_edges() {
    let frames = 2 * RATE as usize;
    let input = sine(440.0, 0.5, frames, RATE);
    let steady = 0.5 / 2.0_f64.sqrt();
    for quality in Quality::ALL {
        for (ratio, pitch) in [(0.6, 0.0), (1.5, 0.0), (1.0, 7.0)] {
            let output = offline_mono(&input, ratio, pitch, quality);
            // Away from the edges by 10 ms, where a block that looks past
            // the edge of the audio sees half of it silent.
            let start = db(rms(&output[480..2_400]) / steady);
            let end = db(rms(&output[output.len() - 2_400..output.len() - 480]) / steady);
            assert!(
                start.abs() < 1.5,
                "{quality:?} {ratio} {pitch}: start at {start:+.2} dB"
            );
            assert!(
                end.abs() < 1.5,
                "{quality:?} {ratio} {pitch}: end at {end:+.2} dB"
            );
            assert!(
                peak(&output) < 0.75,
                "{quality:?} {ratio} {pitch}: peak {}",
                peak(&output)
            );
        }
    }
}

#[test]
fn audio_shorter_than_a_block_is_stretched_too() {
    // 20 ms: a sixth of a block.
    let input = sine(1_000.0, 0.5, 960, RATE);
    for quality in Quality::ALL {
        let output = offline_mono(&input, 2.0, 0.0, quality);
        assert_eq!(output.len(), 1_920);
        let level = db(rms(&output) / rms(&input));
        assert!(level.abs() < 3.0, "{quality:?}: {level:+.2} dB");
        let found = frequency_near(&output, 1_000.0, 0.05, RATE);
        assert!(
            cents(found, 1_000.0).abs() < 20.0,
            "{quality:?}: {found} Hz"
        );
    }
}

#[test]
fn channels_stay_apart_and_in_order() {
    let frames = 60_000;
    let notes = [300.0, 700.0, 1_900.0];
    let channels: Vec<Vec<f32>> = notes
        .iter()
        .map(|note| sine(*note, 0.5, frames, RATE))
        .collect();
    let data: Vec<f32> = (0..frames)
        .flat_map(|frame| channels.iter().map(move |channel| channel[frame]))
        .collect();
    let buffer = AudioBuffer::from_interleaved(RATE, 3, data);
    let output = stretch(&buffer, 1.4, 0.0, Quality::Standard);
    for (channel, note) in notes.iter().enumerate() {
        let own: Vec<f32> = output
            .samples()
            .iter()
            .skip(channel)
            .step_by(3)
            .copied()
            .collect();
        let part = middle(&own);
        assert!(
            db(tone_level(part, *note, RATE) / 0.5).abs() < 0.5,
            "channel {channel}"
        );
        for other in notes.iter().filter(|other| *other != note) {
            assert!(
                tone_level(part, *other, RATE) < 1e-3,
                "channel {channel} has {other} Hz"
            );
        }
    }
}

#[test]
fn a_buffer_and_a_stream_give_the_same_sound() {
    let input = chord(&[330.0, 495.0, 660.0], 0.6, 96_000, RATE);
    for (ratio, pitch) in [(1.25, 0.0), (0.8, 4.0)] {
        let buffer = offline_mono(&input, ratio, pitch, Quality::Standard);
        let stream = stream_mono(&input, ratio, pitch, Quality::Standard);
        assert_eq!(buffer.len(), stream.len());
        let factor = 2.0_f64.powf(pitch / 12.0);
        for note in [330.0, 495.0, 660.0] {
            let level = |signal: &[f32]| tone_level(middle(signal), note * factor, RATE);
            assert!(
                db(level(&buffer) / level(&stream)).abs() < 0.1,
                "{ratio} {pitch} {note}"
            );
        }
    }
}
