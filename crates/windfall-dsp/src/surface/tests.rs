use super::*;
use crate::param::{ParamKind, ParamScale, ParamUnit};

fn params(values: [f32; 8]) -> ControlSurfaceParams {
    let [knob1, knob2, knob3, knob4, knob5, knob6, knob7, knob8] = values;
    ControlSurfaceParams {
        knob1,
        knob2,
        knob3,
        knob4,
        knob5,
        knob6,
        knob7,
        knob8,
    }
}

#[test]
fn defaults_metadata_and_json_agree() {
    let default = ControlSurfaceParams::default();
    assert_eq!(ControlSurfaceParams::NAME, "Control Surface");
    assert_eq!(ControlSurface::default().readout(), [0.5; 8]);
    let json = serde_json::to_value(default).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 8);
    assert_eq!(ControlSurfaceParams::descriptors().len(), 8);
    for (i, info) in ControlSurfaceParams::descriptors().iter().enumerate() {
        assert_eq!(info.id, format!("knob{}", i + 1));
        assert_eq!(ControlSurfaceParams::index_of(info.id), Some(i));
        assert_eq!(info.kind, ParamKind::Float);
        assert_eq!(info.unit, ParamUnit::Fraction);
        assert_eq!(info.scale, ParamScale::Linear);
        assert_eq!((info.min, info.max, info.default), (0.0, 1.0, 0.5));
        assert_eq!(default.get(i), Some(info.default));
        assert_eq!(json[info.id].as_f64(), Some(0.5));
    }
    assert_eq!(
        serde_json::from_value::<ControlSurfaceParams>(json).unwrap(),
        default
    );
    assert_eq!(
        serde_json::from_str::<ControlSurfaceParams>("{}").unwrap(),
        default
    );
    assert_eq!(default.get(8), None);
    let mut edited = default;
    assert!(!edited.set(8, 1.0));
    assert_eq!(edited, default);
    let declaration = ControlSurfaceParams::decl(&ts_rs::Config::default());
    for i in 1..=8 {
        assert!(declaration.contains(&format!("knob{i}: number")));
    }
}

#[test]
fn unity_passthrough_is_bit_exact_at_defaults_and_all_knob_positions() {
    let left = [
        0.0,
        -0.0,
        0.37,
        -2.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x7fc1_2345),
        f32::INFINITY,
    ];
    let right = [
        f32::NEG_INFINITY,
        f32::from_bits(0xffc5_4321),
        f32::MAX,
        f32::MIN,
        1.0,
        -0.9,
        -0.0,
        f32::from_bits(0x7f81_2345),
    ];
    let mut surface = ControlSurface::default();
    surface.prepare(48_000.0, left.len());
    let check = |surface: &mut ControlSurface| {
        let mut l = left;
        let mut r = right;
        surface.process(&mut l, &mut r);
        assert_eq!(l.map(f32::to_bits), left.map(f32::to_bits));
        assert_eq!(r.map(f32::to_bits), right.map(f32::to_bits));
    };
    check(&mut surface);
    for i in 0..8 {
        for value in [0.0, 0.125, 0.5, 0.875, 1.0] {
            let mut p = ControlSurfaceParams::default();
            assert!(p.set(i, value));
            surface.set_params(&p);
            check(&mut surface);
        }
    }
    surface.set_params(&params([0.0, 1.0, 0.25, 0.75, 0.1, 0.9, 0.3, 0.7]));
    check(&mut surface);
    surface.process(&mut [], &mut []);
    assert_eq!(surface.latency_samples(), 0);
    assert_eq!(surface.tail_samples(), 0);
}

#[test]
fn each_knob_clamps_and_preserves_other_knobs() {
    for i in 0..8 {
        for (input, expected) in [(-9.0, 0.0), (9.0, 1.0), (0.375, 0.375)] {
            let mut values = [0.5; 8];
            values[i] = input;
            let raw = params(values);
            values[i] = expected;
            let mut surface = ControlSurface::default();
            surface.set_params(&raw);
            assert_eq!(surface.readout(), values);
            assert_eq!(raw.sanitized(), params(values));
            let mut edited = ControlSurfaceParams::default();
            assert!(edited.set(i, input));
            assert_eq!(edited, params(values));
        }
    }
}

#[test]
fn each_nonfinite_knob_becomes_its_default() {
    for i in 0..8 {
        for input in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut values = [0.25; 8];
            values[i] = input;
            let raw = params(values);
            values[i] = 0.5;
            let mut surface = ControlSurface::default();
            surface.set_params(&raw);
            assert_eq!(surface.readout(), values);
            assert_eq!(raw.sanitized(), params(values));
            let mut edited = params([0.25; 8]);
            assert!(edited.set(i, input));
            assert_eq!(edited, params(values));
        }
    }
}

#[test]
fn prepare_and_reset_preserve_sanitized_readout() {
    let mut surface = ControlSurface::default();
    surface.set_params(&params([-1.0, 2.0, f32::NAN, 0.75, 0.1, 0.9, 0.3, 0.7]));
    let expected = [0.0, 1.0, 0.5, 0.75, 0.1, 0.9, 0.3, 0.7];
    assert_eq!(surface.readout(), expected);
    surface.reset();
    assert_eq!(surface.readout(), expected);
    surface.prepare(f32::NAN, 0);
    assert_eq!(surface.readout(), expected);
    surface.prepare(96_000.0, 4096);
    assert_eq!(surface.readout(), expected);
}

#[test]
fn block_splits_match_with_parameter_edits() {
    let input_left: [f32; 513] = std::array::from_fn(|i| f32::from_bits(i as u32 * 8_000_003));
    let input_right: [f32; 513] = std::array::from_fn(|i| f32::from_bits(!(i as u32 * 8_000_003)));
    let mut whole_left = input_left;
    let mut whole_right = input_right;
    let mut split_left = input_left;
    let mut split_right = input_right;
    let mut whole = ControlSurface::default();
    let mut split = ControlSurface::default();
    whole.prepare(48_000.0, 513);
    split.prepare(48_000.0, 513);
    let mut start = 0;
    for (end, p) in [
        (137, ControlSurfaceParams::default()),
        (389, params([0.0, 1.0, 0.25, 0.75, 0.1, 0.9, 0.3, 0.7])),
        (513, params([f32::NAN; 8])),
    ] {
        whole.set_params(&p);
        split.set_params(&p);
        whole.process(&mut whole_left[start..end], &mut whole_right[start..end]);
        let mut at = start;
        for length in [1, 7, 64, 3, 127].into_iter().cycle() {
            if at == end {
                break;
            }
            let next = (at + length).min(end);
            split.process(&mut split_left[at..next], &mut split_right[at..next]);
            at = next;
        }
        assert_eq!(whole.readout(), split.readout());
        start = end;
    }
    assert_eq!(whole_left.map(f32::to_bits), input_left.map(f32::to_bits));
    assert_eq!(whole_right.map(f32::to_bits), input_right.map(f32::to_bits));
    assert_eq!(split_left.map(f32::to_bits), whole_left.map(f32::to_bits));
    assert_eq!(split_right.map(f32::to_bits), whole_right.map(f32::to_bits));
}
