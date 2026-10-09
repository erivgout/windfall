use super::*;

const RATE: f32 = 8_000.0;

fn ramp() -> SampleTable {
    let data: [f32; MAX_SAMPLE_FRAMES] = std::array::from_fn(|i| i as f32 / 4095.0);
    SampleTable::from_slice(&data, RATE).unwrap()
}
fn render<I: Instrument>(instrument: &mut I, count: usize, block: usize) -> Vec<f32> {
    let mut left = vec![0.0; count];
    let mut right = vec![0.0; count];
    for (left, right) in left.chunks_mut(block).zip(right.chunks_mut(block)) {
        instrument.process(left, right);
    }
    assert!(left.iter().chain(&right).all(|x| x.is_finite()));
    left
}

#[test]
fn slice_keys_address_distinct_ramp_regions_without_bleeding() {
    let params = SliceMapParams {
        table: ramp(),
        level: 1.0,
        ..Default::default()
    };
    let mut instrument = SliceMap::default();
    instrument.prepare(RATE, 512);
    instrument.set_params(&params);
    for slice in 0..SLICE_COUNT {
        instrument.reset();
        instrument.note_on(60 + slice as u8, 1.0);
        let audio = render(&mut instrument, 280, 37);
        let scale = std::f32::consts::FRAC_1_SQRT_2 * 0.25;
        for (frame, &sample) in audio.iter().enumerate().take(240).skip(8) {
            let expected = params.table.data[slice * 256 + frame] * scale;
            assert!((sample - expected).abs() < 1e-6);
        }
        assert!(audio[256..].iter().all(|x| *x == 0.0));
        assert_eq!(instrument.active_voices(), 0);
    }
}

#[test]
fn deck_reverse_flips_the_ramp_and_pitch_gain_apply_per_slice() {
    let mut params = SliceDeckParams {
        table: ramp(),
        level: 1.0,
        ..Default::default()
    };
    params.slices[0] = SliceSettings {
        pitch: 12.0,
        gain: 0.5,
        reverse: true,
    };
    let mut instrument = SliceDeck::default();
    instrument.prepare(RATE, 512);
    instrument.set_params(&params);
    instrument.note_on(60, 1.0);
    let reverse = render(&mut instrument, 150, 23);
    let scale = std::f32::consts::FRAC_1_SQRT_2 * 0.25 * 0.5;
    for (frame, &sample) in reverse.iter().enumerate().take(110).skip(8) {
        assert!((sample - params.table.data[255 - 2 * frame] * scale).abs() < 1e-6);
    }
    assert!(reverse[8] > reverse[80]);
    assert!(reverse[128..].iter().all(|x| *x == 0.0));
    instrument.reset();
    params.slices[0].reverse = false;
    instrument.set_params(&params);
    instrument.note_on(60, 1.0);
    let forward = render(&mut instrument, 150, 29);
    assert!(forward[8] < forward[80]);
}

#[test]
fn grain_density_changes_actual_event_count() {
    let mut cloud = GrainCloud::default();
    cloud.prepare(RATE, 512);
    let mut params = GrainCloudParams {
        table: ramp(),
        density: 8.0,
        grain_size_ms: 20.0,
        ..Default::default()
    };
    cloud.set_params(&params);
    cloud.note_on(60, 1.0);
    let low = render(&mut cloud, 8000, 127);
    let low_events = cloud.grains_spawned();
    cloud.reset();
    params.density = 64.0;
    cloud.set_params(&params);
    cloud.note_on(60, 1.0);
    let high = render(&mut cloud, 8000, 113);
    assert!((7..=9).contains(&low_events));
    assert!((63..=65).contains(&cloud.grains_spawned()));
    assert!(cloud.grains_spawned() > low_events * 6);
    assert_ne!(low, high);
}

#[test]
fn identical_seeds_repeat_across_reset_and_different_seeds_change_audio() {
    let mut cloud = GrainCloud::default();
    let mut params = GrainCloudParams {
        table: ramp(),
        seed: 12345,
        ..Default::default()
    };
    cloud.prepare(RATE, 512);
    cloud.set_params(&params);
    cloud.note_on(60, 1.0);
    let original = render(&mut cloud, 8000, 512);
    cloud.reset();
    cloud.note_on(60, 1.0);
    assert_eq!(original, render(&mut cloud, 8000, 1));
    cloud.reset();
    params.seed += 1;
    cloud.set_params(&params);
    cloud.note_on(60, 1.0);
    assert_ne!(original, render(&mut cloud, 8000, 47));
}

#[test]
fn wave_position_envelope_scans_and_can_reverse_direction() {
    let mut ride = WaveRide::default();
    let mut params = WaveRideParams {
        table: ramp(),
        duration_ms: 128.0,
        level: 1.0,
        ..Default::default()
    };
    ride.prepare(RATE, 512);
    ride.set_params(&params);
    ride.note_on(60, 1.0);
    let forward = render(&mut ride, 1100, 103);
    assert!(forward[100] < forward[800]);
    assert!((forward[512] - 0.5 * 0.25 * std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert!(forward[1025..].iter().all(|x| *x == 0.0));
    ride.reset();
    params.positions = [1.0, 0.8, 0.4, 0.1, 0.6, 0.9, 0.3, 0.0];
    ride.set_params(&params);
    ride.note_on(60, 1.0);
    let scratch = render(&mut ride, 1100, 71);
    assert!(scratch[20] > scratch[400]);
    assert!(scratch[400] < scratch[700]);
    assert!(scratch[700] > scratch[980]);
}

fn release_and_bounds<I: Instrument>(mut instrument: I, params: I::Params) {
    instrument.prepare(RATE, 512);
    instrument.set_params(&params);
    for _ in 0..100 {
        instrument.note_on(60, 1.0);
    }
    assert_eq!(instrument.active_voices(), MAX_POLYPHONY);
    let audio = render(&mut instrument, 32, 7);
    assert!(audio.iter().any(|x| *x != 0.0));
    instrument.note_off(60);
    render(&mut instrument, 8001, 31);
    assert_eq!(instrument.active_voices(), 0);
    assert!(render(&mut instrument, 64, 13).iter().all(|x| *x == 0.0));
    instrument.reset();
    instrument.note_on(60, 1.0);
    render(&mut instrument, 11, 3);
    instrument.all_notes_off();
    render(&mut instrument, 33, 1);
    assert_eq!(instrument.active_voices(), 0);
    assert!(render(&mut instrument, 64, 13).iter().all(|x| *x == 0.0));
}
#[test]
fn all_instruments_have_bounded_polyphony_and_exact_release_silence() {
    release_and_bounds(
        SliceMap::default(),
        SliceMapParams {
            table: ramp(),
            ..Default::default()
        },
    );
    release_and_bounds(
        SliceDeck::default(),
        SliceDeckParams {
            table: ramp(),
            ..Default::default()
        },
    );
    release_and_bounds(
        GrainCloud::default(),
        GrainCloudParams {
            table: ramp(),
            ..Default::default()
        },
    );
    release_and_bounds(
        WaveRide::default(),
        WaveRideParams {
            table: ramp(),
            ..Default::default()
        },
    );
}

fn partition_run<I: Instrument>(
    mut instrument: I,
    mut params: I::Params,
    block: usize,
) -> Vec<f32> {
    instrument.prepare(RATE, 512);
    instrument.set_params(&params);
    instrument.note_on_instance(NoteInstanceId(1), 60, 0.8, -0.3, NoteExpression::default());
    let mut audio = render(&mut instrument, 113, block);
    for index in 0..I::Params::descriptors().len() {
        let info = &I::Params::descriptors()[index];
        if info.kind == crate::ParamKind::Float {
            params.set(index, info.max);
        }
    }
    instrument.set_params(&params);
    audio.extend(render(&mut instrument, 239, block));
    instrument.set_note_pitch(NoteInstanceId(1), 62.5);
    instrument.note_on_instance(NoteInstanceId(2), 62, 0.4, 0.5, NoteExpression::default());
    audio.extend(render(&mut instrument, 101, block));
    instrument.note_off_instance(NoteInstanceId(1), 60);
    audio.extend(render(&mut instrument, 4096, block));
    instrument.all_notes_off();
    audio.extend(render(&mut instrument, 64, block));
    audio
}
#[test]
fn notes_automation_and_release_are_exactly_block_invariant() {
    macro_rules! check {
        ($instrument:ident, $params:ident) => {{
            let params = $params {
                table: ramp(),
                ..Default::default()
            };
            let reference = partition_run($instrument::default(), params, 512);
            for block in [1, 7, 127] {
                assert_eq!(
                    reference,
                    partition_run($instrument::default(), params, block)
                );
            }
        }};
    }
    check!(SliceMap, SliceMapParams);
    check!(SliceDeck, SliceDeckParams);
    check!(GrainCloud, GrainCloudParams);
    check!(WaveRide, WaveRideParams);
}

#[test]
fn instance_release_does_not_release_another_occurrence_of_the_same_key() {
    let mut cloud = GrainCloud::default();
    cloud.prepare(RATE, 512);
    cloud.set_params(&GrainCloudParams {
        table: ramp(),
        release_ms: 1.0,
        ..Default::default()
    });
    for id in [1, 2] {
        cloud.note_on_instance(NoteInstanceId(id), 60, 1.0, 0.0, NoteExpression::default());
    }
    cloud.note_off_instance(NoteInstanceId(1), 60);
    render(&mut cloud, 32, 7);
    assert_eq!(cloud.active_voices(), 1);
    cloud.note_off(60);
    render(&mut cloud, 32, 7);
    assert_eq!(cloud.active_voices(), 0);
}

fn parameter_contract<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned + ts_rs::TS>() {
    let default = P::default();
    assert_eq!(
        serde_json::from_str::<P>(&serde_json::to_string(&default).unwrap()).unwrap(),
        default
    );
    assert_eq!(serde_json::from_str::<P>("{}").unwrap(), default);
    assert!(!P::decl(&ts_rs::Config::default()).is_empty());
    let mut params = default;
    for (i, info) in P::descriptors().iter().enumerate() {
        assert_eq!(default.get(i), Some(info.default));
        assert_eq!(P::index_of(info.id), Some(i));
        for value in [
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MAX,
            -f32::MAX,
        ] {
            assert!(params.set(i, value));
            let current = params.get(i).unwrap();
            assert!(current.is_finite() && current >= info.min && current <= info.max);
        }
    }
    assert_eq!(params.get(P::descriptors().len()), None);
    assert!(!params.set(P::descriptors().len(), 0.0));
}
#[test]
fn copy_params_defaults_serialization_and_descriptors_agree() {
    parameter_contract::<SliceMapParams>();
    parameter_contract::<SliceDeckParams>();
    parameter_contract::<GrainCloudParams>();
    parameter_contract::<WaveRideParams>();
}

#[test]
fn hostile_tables_controls_and_source_replacement_remain_finite() {
    let mut table = ramp();
    table.data[0] = f32::NAN;
    table.data[1] = f32::INFINITY;
    table.data[2] = f32::MAX;
    table.len = u16::MAX;
    table.sample_rate = f32::NAN;
    macro_rules! check {
        ($instrument:ident, $params:ident) => {{
            let mut params = $params {
                table,
                ..Default::default()
            };
            for index in 0..$params::descriptors().len() {
                params.set(index, $params::descriptors()[index].max);
            }
            let mut instrument = $instrument::default();
            instrument.prepare(f32::NAN, 512);
            instrument.set_params(&params);
            for key in 0..128 {
                instrument.note_on(key, 1.0);
            }
            render(&mut instrument, 1000, 19);
            params.table = SampleTable::from_slice(&[0.25], 1000.0).unwrap();
            instrument.set_params(&params);
            render(&mut instrument, 1000, 11);
            params.table = SampleTable::default();
            instrument.set_params(&params);
            assert!(render(&mut instrument, 64, 11).iter().all(|x| *x == 0.0));
            assert_eq!(instrument.active_voices(), 0);
        }};
    }
    check!(SliceMap, SliceMapParams);
    check!(SliceDeck, SliceDeckParams);
    check!(GrainCloud, GrainCloudParams);
    check!(WaveRide, WaveRideParams);
}

#[test]
fn source_table_enforces_capacity_and_short_json_is_zero_padded() {
    assert!(SampleTable::from_slice(&[0.0; 4097], RATE).is_err());
    let table: SampleTable =
        serde_json::from_str(r#"{"data":[0.2,0.5],"len":2,"sampleRate":8000}"#).unwrap();
    assert_eq!(table.data[0..3], [0.2, 0.5, 0.0]);
    let mut value = serde_json::to_value(table).unwrap();
    value["data"] = serde_json::json!(vec![0.0; 4097]);
    assert!(serde_json::from_value::<SampleTable>(value).is_err());
}

#[test]
fn markers_sanitize_in_order_and_empty_slices_are_silent() {
    let mut params = SliceMapParams {
        table: ramp(),
        ..Default::default()
    };
    params.slice_starts = [1.0; SLICE_COUNT];
    params.slice_starts[0] = f32::NAN;
    params.slice_starts[1] = -1.0;
    let clean = params.sanitized();
    assert!(clean.slice_starts.windows(2).all(|p| p[0] <= p[1]));
    let mut instrument = SliceMap::default();
    instrument.prepare(RATE, 512);
    instrument.set_params(&params);
    instrument.note_on(61, 1.0);
    assert_eq!(instrument.active_voices(), 1);
    render(&mut instrument, 10, 1);
    instrument.reset();
    instrument.note_on(62, 1.0);
    assert_eq!(instrument.active_voices(), 0);
    assert!(render(&mut instrument, 64, 1).iter().all(|x| *x == 0.0));
}
