use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bits(samples: &[f32]) -> Vec<u32> {
    samples.iter().map(|sample| sample.to_bits()).collect()
}

#[test]
fn dry_is_bit_exact_at_every_level() {
    let original = [
        0.0,
        -0.0,
        0.25,
        -1.0,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
    ];
    let mut effect = SendTap::default();
    effect.prepare(48_000.0, original.len());
    for step in 0..=100 {
        effect.set_params(&SendTapParams {
            level: step as f32 / 100.0,
        });
        let mut left = original;
        let mut right = original;
        right.reverse();
        let expected_right = bits(&right);
        effect.process(&mut left, &mut right);
        assert_eq!(bits(&left), bits(&original));
        assert_eq!(bits(&right), expected_right);
    }
}

#[test]
fn tap_gain_and_unity_overwrite_the_destination() {
    let mut effect = SendTap::default();
    effect.prepare(48_000.0, 4);
    let mut left = [0.5, -1.0, 0.0, -0.0];
    let mut right = [-0.25, 0.75, -0.0, 0.0];
    for level in [0.125, 0.5, 1.0] {
        effect.set_params(&SendTapParams { level });
        effect.process(&mut left, &mut right);
        let mut send_left = [99.0; 4];
        let mut send_right = [99.0; 4];
        effect.send(&left, &right, &mut send_left, &mut send_right);
        assert_eq!(bits(&send_left), bits(&left.map(|x| x * level)));
        assert_eq!(bits(&send_right), bits(&right.map(|x| x * level)));
    }
    // Unity is a copy even for nonfinite and subnormal samples.
    effect.set_params(&SendTapParams { level: 1.0 });
    left = [
        f32::from_bits(0x7fc0_5678),
        f32::INFINITY,
        f32::from_bits(1),
        -0.0,
    ];
    let mut output = [0.0; 4];
    let mut output_right = [0.0; 4];
    effect.process(&mut left, &mut right);
    effect.send(&left, &right, &mut output, &mut output_right);
    assert_eq!(bits(&output), bits(&left));
}

#[test]
fn default_and_zero_level_produce_exact_silence() {
    let mut effect = SendTap::default();
    let mut left = [f32::NAN, f32::INFINITY, -1.0, -0.0];
    let mut right = [f32::NEG_INFINITY, f32::from_bits(1), 1.0, 0.0];
    let mut output_left = [1.0; 4];
    let mut output_right = [1.0; 4];
    for level in [0.0, 1.0, 0.0] {
        effect.set_params(&SendTapParams { level });
        effect.process(&mut left, &mut right);
        effect.send(&left, &right, &mut output_left, &mut output_right);
        if level == 0.0 {
            assert_eq!(bits(&output_left), vec![0; 4]);
            assert_eq!(bits(&output_right), vec![0; 4]);
        }
    }
}

#[test]
fn parameters_sanitize_and_describe_level() {
    assert_eq!(SendTapParams::NAME, "Send Tap");
    let descriptors = SendTapParams::descriptors();
    assert_eq!(descriptors.len(), 1);
    let descriptor = descriptors[0];
    assert_eq!(descriptor.id, "level");
    assert_eq!(
        (descriptor.min, descriptor.max, descriptor.default),
        (0.0, 1.0, 0.0)
    );
    assert_eq!(descriptor.unit, crate::ParamUnit::Gain);
    assert_eq!(SendTapParams::index_of("level"), Some(0));
    let mut params = SendTapParams::default();
    assert_eq!(params.get(0), Some(0.0));
    assert_eq!(params.get(1), None);
    assert!(!params.set(1, 1.0));
    assert!(params.set(0, 0.25));
    assert_eq!(params.get(0), Some(0.25));
    assert_eq!(
        serde_json::to_value(params).unwrap(),
        serde_json::json!({"level": 0.25})
    );
    assert_eq!(
        serde_json::from_str::<SendTapParams>("{}").unwrap(),
        SendTapParams::default()
    );
    assert!(SendTapParams::decl(&ts_rs::Config::default()).contains("level: number"));
    let mut effect = SendTap::default();
    for (input, expected) in [
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
        (f32::NEG_INFINITY, 0.0),
        (-1.0, 0.0),
        (2.0, 1.0),
    ] {
        effect.set_params(&SendTapParams { level: input });
        let mut left = [1.0];
        let mut right = [-1.0];
        effect.process(&mut left, &mut right);
        let mut output_left = [99.0];
        let mut output_right = [99.0];
        effect.send(&left, &right, &mut output_left, &mut output_right);
        assert_eq!(output_left[0], expected);
        assert_eq!(output_right[0], -expected);
    }
}

#[test]
fn block_splits_and_reset_match() {
    let original_left = std::array::from_fn::<_, 257, _>(|i| (i as f32 * 0.17).sin());
    let original_right = original_left.map(|x| x * -0.3);
    let mut whole = SendTap::default();
    let mut split = SendTap::default();
    whole.prepare(44_100.0, 257);
    split.prepare(44_100.0, 257);
    for level in [0.0, 0.75, 1.0, 0.125, 0.0] {
        whole.set_params(&SendTapParams { level });
        split.set_params(&SendTapParams { level });
        let mut left = original_left;
        let mut right = original_right;
        let mut split_left = original_left;
        let mut split_right = original_right;
        let mut send_left = [99.0; 257];
        let mut send_right = send_left;
        let mut split_send_left = send_left;
        let mut split_send_right = send_left;
        whole.process(&mut left, &mut right);
        whole.send(&left, &right, &mut send_left, &mut send_right);
        for (start, end) in [(0, 1), (1, 18), (18, 18), (18, 101), (101, 257)] {
            split.process(&mut split_left[start..end], &mut split_right[start..end]);
            split.send(
                &split_left[start..end],
                &split_right[start..end],
                &mut split_send_left[start..end],
                &mut split_send_right[start..end],
            );
        }
        assert_eq!(bits(&split_left), bits(&left));
        assert_eq!(bits(&split_right), bits(&right));
        assert_eq!(bits(&split_send_left), bits(&send_left));
        assert_eq!(bits(&split_send_right), bits(&send_right));
        split.reset();
        split.send(&left, &right, &mut split_send_left, &mut split_send_right);
        assert_eq!(bits(&split_send_left), bits(&send_left));
        assert_eq!(bits(&split_send_right), bits(&send_right));
    }
    assert_eq!(whole.latency_samples(), 0);
    assert_eq!(whole.tail_samples(), 0);
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn dependency(directory: &Path, name: &str) -> PathBuf {
    let prefix = format!("lib{name}-");
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&prefix)
                && path
                    .extension()
                    .is_some_and(|extension| extension == "rlib")
        })
        .max_by_key(|path| path.metadata().unwrap().modified().unwrap())
        .unwrap_or_else(|| panic!("missing {name} dependency in {}", directory.display()))
}

#[test]
fn callback_allocator_probe() {
    // Isolate the probe because the library tests already own an allocator.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let scratch =
        std::env::temp_dir().join(format!("windfall-sendtap-probe-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(scratch.clone());
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let library = scratch.join("libwindfall_dsp.rlib");
    let mut compile = Command::new(&rustc);
    compile
        .arg(manifest.join("src/lib.rs"))
        .args([
            "--crate-name",
            "windfall_dsp",
            "--crate-type",
            "rlib",
            "--edition=2024",
            "-Copt-level=1",
            "-L",
        ])
        .arg(format!("dependency={}", deps.display()))
        .arg("-o")
        .arg(&library);
    for name in ["serde", "ts_rs", "windfall_core"] {
        compile
            .arg("--extern")
            .arg(format!("{name}={}", dependency(&deps, name).display()));
    }
    run(&mut compile);
    let executable = scratch.join(if cfg!(windows) {
        "allocator_probe.exe"
    } else {
        "allocator_probe"
    });
    run(Command::new(&rustc)
        .arg(manifest.join("src/sendtap/allocator_probe.rs"))
        .args(["--test", "--edition=2024", "-Copt-level=1", "--extern"])
        .arg(format!("windfall_dsp={}", library.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("-o")
        .arg(&executable));
    run(Command::new(executable).arg("--nocapture"));
}
