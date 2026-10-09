use super::*;
use crate::param::ParamSet;

fn note(key: u8) -> MappedNote {
    MappedNote {
        key,
        velocity: 0.5,
        channel: 2,
        color: 3,
    }
}

#[test]
fn color_table_maps_all_entries_and_clamps() {
    let mut map = ColorMap::default();
    let mut params = ColorMapParams::default();
    for index in 0..16 {
        params.table[index] = (15 - index) as u8;
    }
    map.set_params(&params);
    for color in 0..16 {
        let input = MappedNote { color, ..note(60) };
        assert_eq!(
            map.map(input),
            Some(MappedNote {
                color: 15 - color,
                ..input
            })
        );
    }
    assert_eq!(
        map.map(MappedNote {
            color: 255,
            ..note(60)
        })
        .unwrap()
        .color,
        0
    );
    params.table[15] = 255;
    map.set_params(&params);
    assert_eq!(
        map.map(MappedNote {
            color: 255,
            ..note(60)
        })
        .unwrap()
        .color,
        15
    );
}

#[test]
fn velocity_percent_and_bounds() {
    let mut scale = LevelScale::default();
    assert_eq!(scale.map(note(60)), Some(note(60)));
    scale.set_params(&LevelScaleParams {
        percent: 50.0,
        floor: 0.1,
        ceiling: 0.8,
    });
    assert_eq!(scale.map(note(60)).unwrap().velocity, 0.25);
    assert_eq!(
        scale
            .map(MappedNote {
                velocity: -1.0,
                ..note(60)
            })
            .unwrap()
            .velocity,
        0.1
    );
    scale.set_params(&LevelScaleParams {
        percent: 200.0,
        floor: 0.1,
        ceiling: 0.8,
    });
    assert_eq!(scale.map(note(60)).unwrap().velocity, 0.8);
    scale.set_params(&LevelScaleParams {
        percent: 100.0,
        floor: 0.9,
        ceiling: 0.2,
    });
    assert_eq!(scale.map(note(60)).unwrap().velocity, 0.9);
}

#[test]
fn key_remap_identity_collisions_and_clamping() {
    let mut map = KeyMap::default();
    for key in 0..128 {
        assert_eq!(map.map(note(key)), Some(note(key)));
    }
    let mut params = KeyMapParams::default();
    params.table[60] = 36;
    params.table[61] = 36;
    params.table[127] = 255;
    map.set_params(&params);
    let mut notes = [note(60), note(61), note(255)];
    assert_eq!(map.transform(&mut notes, 3), 3);
    assert_eq!(notes.map(|n| n.key), [36, 36, 127]);
}

#[test]
fn split_boundary_drops_and_compacts_in_order() {
    let mut split = KeySplit::default();
    split.set_params(&KeySplitParams {
        low_min: 48,
        high_max: 72,
        ..KeySplitParams::default()
    });
    let mut notes = [note(47), note(48), note(59), note(60), note(72), note(73)];
    assert_eq!(split.transform(&mut notes, usize::MAX), 4);
    assert_eq!(
        notes[..4]
            .iter()
            .map(|n| (n.key, n.channel))
            .collect::<Vec<_>>(),
        [(48, 0), (48, 0), (72, 1), (72, 1)]
    );
    assert!(notes[..4].iter().all(|n| n.color == 3));
    split.set_params(&KeySplitParams {
        low_min: 70,
        high_max: 50,
        ..KeySplitParams::default()
    });
    assert_eq!(split.map(note(59)), None);
    assert_eq!(split.map(note(60)), None);
}

#[test]
fn array_prefix_empty_and_unused_tail() {
    let map = KeyMap::default();
    let mut notes = [note(60), note(255)];
    assert_eq!(map.transform(&mut notes, 1), 1);
    assert_eq!(notes[1].key, 255);
    assert_eq!(map.transform(&mut notes, 0), 0);
    assert_eq!(map.transform(&mut [], 1), 0);
}

#[test]
fn step_grid_known_tempo_gate_pitch_velocity_and_wrap() {
    let mut grid = StepGrid::default();
    let mut params = StepGridParams::default();
    params.steps[1].gate = false;
    params.steps[2].pitch_offset = 12;
    params.steps[2].velocity_scale = 0.5;
    params.steps[3].pitch_offset = -127;
    params.steps[4].pitch_offset = 127;
    grid.set_params(&params);
    grid.set_tempo(120.0);
    assert_eq!(grid.current_step(), 0);
    assert_eq!(grid.map(note(60)), Some(note(60)));
    // 120 BPM, four steps per beat: 6000 samples per step at 48 kHz.
    grid.advance_samples(5999, 48_000.0);
    assert_eq!(grid.current_step(), 0);
    grid.advance_samples(1, 48_000.0);
    assert_eq!(grid.current_step(), 1);
    assert_eq!(grid.map(note(60)), None);
    grid.advance_samples(6000, 48_000.0);
    assert_eq!(
        grid.map(note(60)),
        Some(MappedNote {
            key: 72,
            velocity: 0.25,
            ..note(60)
        })
    );
    grid.advance_samples(6000, 48_000.0);
    assert_eq!(grid.map(note(60)).unwrap().key, 0);
    grid.advance_samples(6000, 48_000.0);
    assert_eq!(grid.map(note(60)).unwrap().key, 127);
    grid.advance_samples(12 * 6000, 48_000.0);
    assert_eq!(grid.current_step(), 0);
    grid.set_tempo(60.0);
    grid.advance_samples(6000, 48_000.0);
    assert_eq!(grid.current_step(), 0);
    grid.advance_samples(6000, 48_000.0);
    assert_eq!(grid.current_step(), 1);
    grid.reset();
    assert_eq!(grid.current_step(), 0);
}

#[test]
fn clock_invalid_values_and_partitioning() {
    let mut whole = StepGrid::default();
    let mut split = whole;
    whole.advance_samples(18_000, 48_000.0);
    for _ in 0..3 {
        split.advance_samples(6000, 48_000.0);
    }
    assert_eq!(whole.current_step(), split.current_step());
    split.advance_samples(u64::MAX, f32::NAN);
    split.advance_samples(u64::MAX, 0.0);
    assert_eq!(split.current_step(), 3);
    split.reset();
    split.set_tempo(f32::NAN);
    split.advance_samples(6000, 48_000.0);
    assert_eq!(split.current_step(), 1);
    split.advance_samples(u64::MAX, f32::MIN_POSITIVE);
    assert!(split.current_step() < 16);
}

#[test]
fn nonfinite_velocity_is_sanitized_by_every_transform() {
    fn check<T: NoteTransform>() {
        for velocity in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let result = T::default().map(MappedNote {
                velocity,
                key: 255,
                channel: 255,
                color: 255,
            });
            let result = result.unwrap();
            assert_eq!(result.velocity, 0.0);
            assert!(result.key <= 127 && result.channel <= 15 && result.color <= 15);
        }
    }
    check::<ColorMap>();
    check::<LevelScale>();
    check::<KeyMap>();
    check::<KeySplit>();
    check::<StepGrid>();
}

#[test]
fn params_cover_fixed_tables_and_match_json_defaults() {
    fn check<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned + ts_rs::TS>() {
        let mut params = P::default();
        let json = serde_json::to_value(params).unwrap();
        assert_eq!(serde_json::from_value::<P>(json.clone()).unwrap(), params);
        assert!(!P::decl(&ts_rs::Config::default()).is_empty());
        for (index, info) in P::descriptors().iter().enumerate() {
            assert_eq!(params.get(index), Some(info.default));
            let mut field = &json;
            for part in info.id.split('.') {
                field = if let Ok(index) = part.parse::<usize>() {
                    &field[index]
                } else {
                    &field[part]
                };
            }
            let value = field
                .as_f64()
                .map(|v| v as f32)
                .or_else(|| field.as_bool().map(|v| if v { 1.0 } else { 0.0 }))
                .unwrap();
            assert_eq!(value, info.default, "{}", info.id);
            assert!(params.set(index, f32::NAN));
            assert!(params.get(index).unwrap().is_finite());
            assert!(params.set(index, f32::MAX));
            assert_eq!(params.get(index), Some(info.max));
            assert!(params.set(index, -f32::MAX));
            assert_eq!(params.get(index), Some(info.min));
            assert!(params.set(index, info.default));
        }
        assert_eq!(params.sanitized(), P::default());
        assert_eq!(params.get(P::descriptors().len()), None);
        assert!(!params.set(P::descriptors().len(), 1.0));
        assert!(!params.approach(&P::default(), 1.0));
    }
    check::<ColorMapParams>();
    check::<LevelScaleParams>();
    check::<KeyMapParams>();
    check::<KeySplitParams>();
    check::<StepGridParams>();
    assert_eq!(ColorMapParams::descriptors().len(), 16);
    assert_eq!(KeyMapParams::descriptors().len(), 128);
    assert_eq!(StepGridParams::descriptors().len(), 49);
}

#[test]
fn key_table_rejects_incomplete_or_oversized_data() {
    assert!(serde_json::from_value::<KeyMapParams>(serde_json::json!({"table": [60]})).is_err());
    assert!(
        serde_json::from_value::<KeyMapParams>(serde_json::json!({"table": vec![60; 129]}))
            .is_err()
    );
    assert_eq!(
        serde_json::from_str::<KeyMapParams>("{}").unwrap(),
        KeyMapParams::default()
    );
}
