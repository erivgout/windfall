use crate::blocks::oscillator::Waveform;
use crate::instrument::Instrument;
use crate::param::ParamSet;

use super::*;

fn render<I: Instrument>(instrument: &mut I, samples: usize, block: usize) -> Vec<f32> {
    let mut left = vec![0.0; samples];
    let mut right = vec![0.0; samples];
    for (l, r) in left.chunks_mut(block).zip(right.chunks_mut(block)) {
        instrument.process(l, r);
        assert_eq!(l, r);
    }
    left
}

fn audible<I: Instrument + Default>() {
    let mut instrument = I::default();
    instrument.prepare(48_000.0, 64);
    instrument.note_on(48, 0.75);
    let signal = render(&mut instrument, 4096, 63);
    assert!(signal.iter().all(|s| s.is_finite()));
    assert!(
        signal.iter().any(|s| s.abs() > 0.001),
        "{}",
        I::Params::NAME
    );
    instrument.note_off(48);
    let _ = render(&mut instrument, 200_000, 127);
    assert_eq!(instrument.active_voices(), 0);
    assert!(render(&mut instrument, 256, 1).iter().all(|&s| s == 0.0));
    instrument.note_on(60, 1.0);
    let _ = render(&mut instrument, 1024, 64);
    instrument.all_notes_off();
    let _ = render(&mut instrument, 241, 7);
    assert_eq!(instrument.active_voices(), 0);
    instrument.reset();
    assert!(render(&mut instrument, 64, 64).iter().all(|&s| s == 0.0));
}

#[test]
fn all_instruments_sound_release_stop_and_reset() {
    audible::<AcidLine>();
    audible::<TripleOsc>();
    audible::<WaveLane>();
    audible::<MacroVoice>();
}

fn split<I: Instrument + Default>(params: I::Params) {
    let mut whole = I::default();
    let mut split = I::default();
    for synth in [&mut whole, &mut split] {
        synth.set_params(&params);
        synth.prepare(48_000.0, 8192);
        synth.set_tempo(137.0);
        synth.note_on(48, 0.8);
        synth.process(&mut [], &mut []);
    }
    assert_eq!(render(&mut whole, 8192, 8192), render(&mut split, 8192, 37));
    let mut changed = params;
    for (i, descriptor) in I::Params::descriptors().iter().enumerate() {
        changed.set(i, (descriptor.min + descriptor.max) * 0.5);
    }
    for synth in [&mut whole, &mut split] {
        synth.set_params(&changed);
        synth.set_tempo(91.0);
        synth.note_on(60, 0.7);
        synth.note_off(48);
    }
    assert_eq!(
        render(&mut whole, 32000, 32000),
        render(&mut split, 32000, 127)
    );
    for synth in [&mut whole, &mut split] {
        synth.all_notes_off();
    }
    assert_eq!(render(&mut whole, 1024, 1024), render(&mut split, 1024, 1));
}

#[test]
fn blocks_split_identically_across_automation_tempo_and_note_events() {
    split::<AcidLine>(AcidLineParams::default());
    let mut sequence = AcidLineParams {
        sequencer: true,
        ..AcidLineParams::default()
    };
    for (i, step) in sequence.steps.iter_mut().enumerate() {
        step.pitch_offset = i as i8 - 8;
        step.accent = i % 3 == 0;
        step.slide = i % 2 == 0;
        step.enabled = i % 5 != 0;
    }
    split::<AcidLine>(sequence);
    split::<TripleOsc>(TripleOscParams::default());
    let mut noise = TripleOscParams::default();
    noise.oscillators[0].waveform = Waveform::WhiteNoise;
    noise.oscillators[1].waveform = Waveform::PinkNoise;
    split::<TripleOsc>(noise);
    split::<WaveLane>(WaveLaneParams::default());
    split::<MacroVoice>(MacroVoiceParams {
        engine_mix: 0.7,
        ..MacroVoiceParams::default()
    });
}

fn params<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned + ts_rs::TS>(length: usize) {
    let defaults = P::default();
    assert_eq!(P::descriptors().len(), length);
    let json = serde_json::to_value(defaults).unwrap();
    assert_eq!(serde_json::from_value::<P>(json.clone()).unwrap(), defaults);
    assert!(!P::decl(&ts_rs::Config::default()).is_empty());
    for (i, info) in P::descriptors().iter().enumerate() {
        assert_eq!(defaults.get(i), Some(info.default));
        assert_eq!(P::index_of(info.id), Some(i));
        let mut path = &json;
        for part in info.id.split('.') {
            path = if let Ok(index) = part.parse::<usize>() {
                &path[index]
            } else {
                &path[part]
            };
        }
        assert!(!path.is_null(), "{}", info.id);
        let mut p = defaults;
        for value in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            -f32::MAX,
        ] {
            assert!(p.set(i, value));
            let value = p.get(i).unwrap();
            assert!(value.is_finite() && value >= info.min && value <= info.max);
        }
        for boundary in [info.min, info.max] {
            p.set(i, boundary);
            assert_eq!(p.get(i), Some(boundary));
        }
    }
    assert_eq!(defaults.get(length), None);
    assert!(!P::default().set(length, 1.0));
}

#[test]
fn parameter_indices_defaults_serialization_and_sanitization() {
    params::<AnalogEnvelopeParams>(4);
    params::<AnalogOscParams>(4);
    params::<AcidLineParams>(75);
    params::<TripleOscParams>(20);
    params::<WaveLaneParams>(9);
    params::<MacroVoiceParams>(4);
    assert_eq!(AcidLineParams::index_of("steps.15.slide"), Some(74));
    assert_eq!(TripleOscParams::index_of("gain"), Some(19));
    assert_eq!(WaveLaneParams::index_of("gain"), Some(8));
}

fn extremes<I: Instrument + Default>() {
    for sample_rate in [f32::NAN, f32::INFINITY, -1.0, 8000.0, 192000.0] {
        let mut instrument = I::default();
        instrument.prepare(sample_rate, 64);
        let mut p = I::Params::default();
        for iteration in 0..8 {
            for (i, info) in I::Params::descriptors().iter().enumerate() {
                p.set(
                    i,
                    if iteration % 2 == 0 {
                        info.max
                    } else {
                        info.min
                    },
                );
            }
            instrument.set_params(&p);
            instrument.set_tempo([f32::NAN, f32::INFINITY, f32::MAX, -f32::MAX][iteration % 4]);
            for key in 240..=255 {
                instrument.note_on(key, f32::MAX);
            }
            let signal = render(&mut instrument, 513, 17);
            assert!(signal.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
            assert!(instrument.active_voices() <= 8);
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0] {
                instrument.note_on(255, value);
            }
            instrument.all_notes_off();
            instrument.reset();
        }
    }
}

#[test]
fn extreme_controls_notes_and_rates_remain_finite() {
    extremes::<AcidLine>();
    extremes::<TripleOsc>();
    extremes::<WaveLane>();
    extremes::<MacroVoice>();
}

#[test]
fn direct_nonfinite_fields_are_cleaned_before_audio() {
    let mut acid = AcidLine::default();
    acid.prepare(48000.0, 64);
    acid.set_params(&AcidLineParams {
        cutoff_hz: f32::NAN,
        resonance: f32::INFINITY,
        slide_ms: -f32::MAX,
        ..AcidLineParams::default()
    });
    acid.note_on(36, 1.0);
    assert!(render(&mut acid, 1024, 64).iter().all(|s| s.is_finite()));
    let mut triple = TripleOsc::default();
    triple.prepare(48000.0, 64);
    let mut p = TripleOscParams {
        gain: f32::NAN,
        cutoff_hz: f32::INFINITY,
        ..TripleOscParams::default()
    };
    p.oscillators[0].detune_cents = f32::NAN;
    p.envelope.release_ms = f32::INFINITY;
    triple.set_params(&p);
    triple.note_on(60, 1.0);
    assert!(render(&mut triple, 1024, 64).iter().all(|s| s.is_finite()));
    let mut wave = WaveLane::default();
    wave.prepare(48000.0, 64);
    wave.set_params(&WaveLaneParams {
        scan: f32::NAN,
        gain: f32::INFINITY,
        ..WaveLaneParams::default()
    });
    wave.note_on(60, 1.0);
    assert!(render(&mut wave, 1024, 64).iter().all(|s| s.is_finite()));
    let mut macros = MacroVoice::default();
    macros.prepare(48000.0, 64);
    macros.set_params(&MacroVoiceParams {
        engine_mix: f32::NAN,
        tone: f32::INFINITY,
        motion: f32::NEG_INFINITY,
        shape: f32::MAX,
    });
    macros.note_on(60, 1.0);
    assert!(render(&mut macros, 1024, 64).iter().all(|s| s.is_finite()));
}

fn steal(bank: &mut common::Bank) {
    for key in 60..68 {
        bank.note_on(key, 1.0, 48000.0);
    }
    assert_eq!(bank.active(), 8);
    bank.note_on(80, 1.0, 48000.0);
    assert!(!bank.voices.iter().any(|v| v.key == 60));
    assert!(bank.voices.iter().any(|v| v.key == 80));
    bank.note_off(65);
    bank.note_on(81, 1.0, 48000.0);
    assert!(!bank.voices.iter().any(|v| v.key == 65));
    assert!(bank.voices.iter().any(|v| v.key == 61));
    // Long sessions retain bounded, unique age ranks.
    for i in 0..1000 {
        bank.note_on((i % 128) as u8, 1.0, 48000.0);
    }
    let mut ages = bank.voices.map(|v| v.age);
    ages.sort();
    assert_eq!(ages, [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn each_poly_instrument_steals_oldest_release_then_oldest_held() {
    steal(&mut TripleOsc::default().bank);
    steal(&mut WaveLane::default().bank);
    steal(&mut MacroVoice::default().bank);
}

#[test]
fn expired_voice_slots_do_not_age_a_long_held_voice_past_capacity() {
    let mut bank = common::Bank::default();
    bank.note_on(40, 1.0, 48000.0);
    for _ in 0..1000 {
        bank.note_on(60, 1.0, 48000.0);
        for voice in &mut bank.voices {
            if voice.key == 60 {
                *voice = common::Voice::default();
            }
        }
    }
    assert_eq!(bank.active(), 1);
    assert!(bank.voices.iter().find(|v| v.key == 40).unwrap().age < 8);
}

#[test]
fn mono_last_note_priority_and_slide_are_defined() {
    let mut acid = AcidLine::default();
    acid.prepare(48000.0, 64);
    acid.set_params(&AcidLineParams {
        manual_slide: true,
        slide_ms: 100.0,
        ..AcidLineParams::default()
    });
    acid.note_on(36, 0.7);
    let _ = render(&mut acid, 1024, 64);
    acid.note_on(48, 0.7);
    acid.note_off(36);
    assert_eq!(acid.active_voices(), 1);
    assert_eq!(acid.pitch.value(), 36.0);
    let _ = render(&mut acid, 2400, 64);
    assert!((acid.pitch.value() - 42.0).abs() < 0.02);
    let _ = render(&mut acid, 2400, 64);
    assert_eq!(acid.pitch.value(), 48.0);
    acid.note_off(48);
    let _ = render(&mut acid, 3000, 64);
    assert_eq!(acid.active_voices(), 0);
}

#[test]
fn sequencer_runs_sixteenths_wraps_transposes_and_slides() {
    let mut acid = AcidLine::default();
    acid.prepare(48000.0, 64);
    let mut p = AcidLineParams {
        sequencer: true,
        slide_ms: 100.0,
        ..AcidLineParams::default()
    };
    p.steps[0].slide = true;
    p.steps[1].pitch_offset = 12;
    p.steps[1].accent = true;
    acid.set_params(&p);
    acid.set_tempo(120.0);
    acid.note_on(36, 0.7);
    let _ = render(&mut acid, 6000, 64);
    assert_eq!(acid.step, 0);
    let _ = render(&mut acid, 1, 1);
    assert_eq!(acid.step, 1);
    assert!(acid.pitch.value() > 36.0 && acid.pitch.value() < 36.1);
    assert_eq!(acid.pitch.target(), 48.0);
    let _ = render(&mut acid, 90000, 127);
    assert_eq!(acid.step, 0);
    acid.note_off(36);
    let _ = render(&mut acid, 3000, 64);
    assert_eq!(acid.active_voices(), 0);
}

#[test]
fn tempo_changes_preserve_fractional_step_progress() {
    let mut acid = AcidLine::default();
    acid.prepare(48000.0, 64);
    acid.set_params(&AcidLineParams {
        sequencer: true,
        ..AcidLineParams::default()
    });
    acid.note_on(36, 0.7);
    let _ = render(&mut acid, 3000, 64);
    acid.set_tempo(240.0);
    let _ = render(&mut acid, 1500, 64);
    assert_eq!(acid.step, 0);
    let _ = render(&mut acid, 2, 1);
    assert_eq!(acid.step, 1);
}

#[test]
fn disabled_steps_are_silent_and_remain_counted_while_clock_is_armed() {
    let mut acid = AcidLine::default();
    acid.prepare(48000.0, 64);
    let p = AcidLineParams {
        sequencer: true,
        steps: [AcidStep {
            enabled: false,
            ..AcidStep::default()
        }; 16],
        ..AcidLineParams::default()
    };
    acid.set_params(&p);
    acid.note_on(36, 1.0);
    assert!(render(&mut acid, 12001, 64).iter().all(|&s| s == 0.0));
    assert_eq!(acid.step, 2);
    assert_eq!(acid.active_voices(), 1);
    acid.all_notes_off();
    assert_eq!(acid.active_voices(), 0);
}

#[test]
fn accent_and_each_oscillator_have_audible_effect() {
    let acid_signal = |accent| {
        let mut synth = AcidLine::default();
        synth.prepare(48000.0, 64);
        let mut p = AcidLineParams {
            sequencer: true,
            ..AcidLineParams::default()
        };
        p.steps[0].accent = accent;
        synth.set_params(&p);
        synth.note_on(36, 0.7);
        render(&mut synth, 4096, 64)
    };
    assert_ne!(acid_signal(false), acid_signal(true));
    let mut signals = Vec::new();
    for oscillator in 0..3 {
        let mut synth = TripleOsc::default();
        synth.prepare(48000.0, 64);
        let mut p = TripleOscParams::default();
        for (i, osc) in p.oscillators.iter_mut().enumerate() {
            osc.level = if i == oscillator { 1.0 } else { 0.0 };
            osc.semitones = i as f32 * 7.0;
        }
        synth.set_params(&p);
        synth.note_on(60, 1.0);
        signals.push(render(&mut synth, 4096, 64));
    }
    assert_ne!(signals[0], signals[1]);
    assert_ne!(signals[1], signals[2]);
}

#[test]
fn tables_are_copy_finite_periodic_and_scan_changes_timbre() {
    let mut tables = common::Tables::default();
    tables.generate();
    let copy = tables;
    for scan in [0.0, 0.5, 1.0] {
        assert!((tables.sample(0.0, 0.001, scan) - tables.sample(1.0, 0.001, scan)).abs() < 1e-6);
        for i in 0..256 {
            let value = copy.sample(i as f32 / 256.0, 0.001, scan);
            assert!(value.is_finite() && value.abs() <= 1.0);
        }
    }
    assert_ne!(
        tables.sample(0.1, 0.001, 0.0),
        tables.sample(0.1, 0.001, 1.0)
    );
    let signal = |scan| {
        let mut synth = WaveLane::default();
        synth.prepare(48000.0, 64);
        synth.set_params(&WaveLaneParams {
            scan,
            ..WaveLaneParams::default()
        });
        synth.note_on(48, 1.0);
        render(&mut synth, 4096, 64)
    };
    assert_ne!(signal(0.0), signal(1.0));
}

#[test]
fn macro_engines_and_all_four_controls_change_audio() {
    let signal = |p| {
        let mut synth = MacroVoice::default();
        synth.prepare(48000.0, 64);
        synth.set_params(&p);
        synth.note_on(48, 1.0);
        render(&mut synth, 4096, 64)
    };
    let mut engines = Vec::new();
    for engine_mix in [0.0, 0.5, 1.0] {
        let p = MacroVoiceParams {
            engine_mix,
            ..MacroVoiceParams::default()
        };
        let original = signal(p);
        assert!(original.iter().any(|s| s.abs() > 0.001));
        for i in 1..4 {
            let mut changed = p;
            changed.set(i, 0.95);
            assert_ne!(original, signal(changed), "engine {engine_mix}, macro {i}");
        }
        engines.push(original);
    }
    assert_ne!(engines[0], engines[1]);
    assert_ne!(engines[1], engines[2]);
}

#[test]
fn reset_repeats_original_audio_with_current_parameters() {
    fn repeat<I: Instrument + Default>() {
        let mut synth = I::default();
        synth.prepare(48000.0, 64);
        let mut p = I::Params::default();
        p.set(0, I::Params::descriptors()[0].max);
        synth.set_params(&p);
        synth.note_on(50, 0.8);
        let original = render(&mut synth, 4096, 64);
        synth.reset();
        synth.note_on(50, 0.8);
        assert_eq!(original, render(&mut synth, 4096, 37));
    }
    repeat::<AcidLine>();
    repeat::<TripleOsc>();
    repeat::<WaveLane>();
    repeat::<MacroVoice>();
}
