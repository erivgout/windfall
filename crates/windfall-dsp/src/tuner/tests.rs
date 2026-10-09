use super::*;

fn feed(tuner: &mut Tuner, frequency: f64, rate: f64, frames: usize) {
    let mut start = 0;
    while start < frames {
        let n = (frames - start).min(128);
        let mut left: [f32; 128] = std::array::from_fn(|i| {
            (0.5 * (std::f64::consts::TAU * frequency * (start + i) as f64 / rate).sin()) as f32
        });
        let mut right = left;
        tuner.process(&mut left[..n], &mut right[..n]);
        start += n;
    }
}

#[test]
fn tuner_measures_octaves_and_a440_within_one_cent() {
    for (frequency, note) in [(220.0, 57), (440.0, 69), (880.0, 81)] {
        let mut tuner = Tuner::default();
        tuner.prepare(48_000.0, 128);
        feed(&mut tuner, frequency, 48_000.0, 12_288);
        let reading = tuner.readout();
        let cents = 1200.0 * (reading.frequency_hz as f64 / frequency).log2();
        assert!(reading.confidence > 0.98, "{reading:?}");
        assert!(cents.abs() < 1.0, "{frequency}: {cents} cents, {reading:?}");
        assert_eq!(reading.midi_note, Some(note));
        assert!(reading.cents_error.abs() < 1.0);
    }
}

#[test]
fn tuner_is_bit_exact_passthrough_even_for_nonfinite_audio() {
    let mut tuner = Tuner::default();
    let mut left = [0.0, -0.0, 0.37, -2.0, f32::NAN, f32::INFINITY];
    let mut right = [f32::NEG_INFINITY, -0.5, 1.0e-30, 0.0, 4.0, 0.9];
    let original_left = left.map(f32::to_bits);
    let original_right = right.map(f32::to_bits);
    tuner.process(&mut left, &mut right);
    assert_eq!(left.map(f32::to_bits), original_left);
    assert_eq!(right.map(f32::to_bits), original_right);
    assert_eq!(tuner.readout(), TunerReadout::default());
    assert_eq!(tuner.latency_samples(), 0);
    assert_eq!(tuner.tail_samples(), 0);
}

#[test]
fn tuner_silence_empty_and_near_silence_clear_old_notes() {
    let mut tuner = Tuner::default();
    for value in [0.0, 1.0e-7] {
        feed(&mut tuner, 440.0, 48_000.0, 8192);
        assert_eq!(tuner.readout().midi_note, Some(69));
        tuner.process(&mut [value; 128], &mut [value; 128]);
        assert_eq!(tuner.readout(), TunerReadout::default());
    }
    feed(&mut tuner, 440.0, 48_000.0, 8192);
    tuner.process(&mut [], &mut []);
    assert_eq!(tuner.readout(), TunerReadout::default());
}

#[test]
fn tuner_noise_and_dc_do_not_invent_a_note() {
    let mut tuner = Tuner::default();
    let mut rng = 0x57f0a93_u32;
    for block in 0..128 {
        let mut left = std::array::from_fn::<_, 128, _>(|_| {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng as f64 / u32::MAX as f64 - 0.5) as f32
        });
        let mut right = left;
        tuner.process(&mut left, &mut right);
        let reading = tuner.readout();
        assert_eq!(reading.midi_note, None, "noise block {block}: {reading:?}");
        assert_eq!(reading.frequency_hz, 0.0);
        assert!(reading.confidence < 0.3, "{reading:?}");
    }
    for _ in 0..64 {
        tuner.process(&mut [0.5; 128], &mut [0.5; 128]);
    }
    assert_eq!(tuner.readout(), TunerReadout::default());
}

#[test]
fn tuner_reference_params_and_readout_are_copy_and_described() {
    fn copy<T: Copy>(_: T) {}
    let mut params = TunerParams::default();
    copy(params);
    copy(TunerReadout::default());
    assert_eq!(TunerParams::NAME, "Tuner");
    assert_eq!(params.reference_hz, 440.0);
    assert_eq!(TunerParams::index_of("referenceHz"), Some(0));
    assert_eq!(TunerParams::descriptors()[0].default, params.reference_hz);
    assert_eq!(serde_json::to_value(params).unwrap()["referenceHz"], 440.0);
    assert_eq!(serde_json::from_str::<TunerParams>("{}").unwrap(), params);
    assert!(TunerParams::decl(&ts_rs::Config::default()).contains("referenceHz"));
    assert!(params.set(0, f32::NAN));
    assert_eq!(params.reference_hz, 440.0);
    assert!(params.set(0, 500.0));
    assert_eq!(params.reference_hz, 480.0);
    assert!(!params.set(1, 440.0));
    assert_eq!(params.get(1), None);

    let mut tuner = Tuner::default();
    feed(&mut tuner, 440.0, 48_000.0, 8192);
    let before = tuner.readout();
    tuner.set_params(&TunerParams {
        reference_hz: 442.0,
    });
    let after = tuner.readout();
    assert_eq!(after.frequency_hz, before.frequency_hz);
    assert_eq!(after.midi_note, Some(69));
    assert!((after.cents_error + 1200.0 * (442.0_f32 / 440.0).log2()).abs() < 1.0);
    tuner.reset();
    assert_eq!(tuner.readout(), TunerReadout::default());
    feed(&mut tuner, 442.0, 48_000.0, 8192);
    assert!(tuner.readout().cents_error.abs() < 1.0);
}

#[test]
fn tuner_handles_rates_and_partial_windows() {
    for rate in [8000.0, 44_100.0, 96_000.0, 384_000.0] {
        let mut tuner = Tuner::default();
        tuner.prepare(rate as f32, 128);
        feed(&mut tuner, 440.0, rate, 128);
        assert_eq!(tuner.readout(), TunerReadout::default());
        tuner.reset();
        feed(&mut tuner, 440.0, rate, (rate * 0.25) as usize);
        let reading = tuner.readout();
        assert!(reading.confidence > 0.98, "{rate}: {reading:?}");
        assert!((1200.0 * (reading.frequency_hz as f64 / 440.0).log2()).abs() < 1.0);
    }
}

#[test]
fn tuner_detects_a_harmonic_monophonic_waveform() {
    let mut tuner = Tuner::default();
    for block in 0..96 {
        let mut left: [f32; 128] = std::array::from_fn(|i| {
            let phase = std::f64::consts::TAU * 220.0 * (block * 128 + i) as f64 / 48_000.0;
            (0.35 * phase.sin() + 0.2 * (2.0 * phase).sin() + 0.1 * (3.0 * phase).sin()) as f32
        });
        let mut right = left;
        tuner.process(&mut left, &mut right);
    }
    let reading = tuner.readout();
    assert_eq!(reading.midi_note, Some(57));
    assert!(reading.confidence > 0.98);
    assert!((1200.0 * (reading.frequency_hz / 220.0).log2()).abs() < 1.0);
}
