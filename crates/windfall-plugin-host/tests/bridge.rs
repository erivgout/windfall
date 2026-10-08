//! Real helper processes and callback allocator/time guards. Windows headless
//! evidence does not establish packaged installation or hardware deadlines.
#![cfg(windows)]
mod common;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use windfall_plugin_host::{
    PluginHost, PluginState,
    bridge::{
        adapter::{Audio, ParameterSpec, Signals},
        protocol::{Config, Identity, Kind},
        slots::{LocalWords, Region},
        supervisor::{self, Control, Launch},
    },
};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
struct Allocator;
fn count() {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    }
}
// SAFETY: unchanged System allocation contract; counters are thread-local.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count();
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn guarded(work: impl FnOnce()) -> usize {
    CALLS.set(0);
    WATCH.set(true);
    work();
    WATCH.set(false);
    CALLS.get()
}
fn fixture(format: &str) -> PathBuf {
    if let Some(source) = std::env::var_os("WINDFALL_BRIDGE_FIXTURE") {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bridge-fixture");
        std::fs::create_dir_all(&folder).unwrap();
        let target = folder.join(format!("bridge.{format}"));
        if !target.exists() {
            std::fs::copy(source, &target).unwrap();
        }
        target
    } else {
        common::plugin_file("bridge-fixture", &format!("bridge.{format}"))
    }
}
fn options(
    format: &str,
    id: &str,
    kind: Kind,
    parameters: Vec<ParameterSpec>,
    audio_timeout: Duration,
) -> Launch {
    let identity = supervisor::fresh_identity(1, 7, 99).unwrap();
    Launch {
        helper: PathBuf::from(env!("CARGO_BIN_EXE_windfall-plugin-audio")),
        plugin: fixture(format),
        id: id.into(),
        format: format.into(),
        config: Config {
            identity,
            sample_rate: 48_000,
            block: 64,
            native_latency: 0,
            kind,
        },
        approved_binary: windfall_plugin_host::paths::plugin_file_identity(&fixture(format))
            .unwrap(),
        parameters,
        state: vec![],
        offline: false,
        startup_timeout: Duration::from_secs(5),
        audio_timeout,
    }
}
fn launch(
    format: &str,
    id: &str,
    kind: Kind,
    parameters: Vec<ParameterSpec>,
    audio_timeout: Duration,
) -> (Control, Audio) {
    let options = options(format, id, kind, parameters, audio_timeout);
    let (control, audio, _) = supervisor::launch(options).unwrap();
    (control, audio)
}
fn gain(format: &str) -> ParameterSpec {
    ParameterSpec {
        id: 7,
        min: 0.0,
        max: if format == "clap" { 2.0 } else { 1.0 },
        value: 0.5,
        read_only: false,
        stepped: false,
    }
}
fn vst_id(index: usize) -> String {
    format!("{:032X}", index + 1)
}
fn wait_failed(control: &Control) {
    let end = Instant::now() + Duration::from_secs(3);
    while !control.status().reaped {
        assert!(
            Instant::now() < end,
            "helper was not reclaimed: {:?}",
            control.status()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(control.status().failed);
}

#[test]
fn callback_guards_cover_startup_full_late_reset_saturation_and_failed_removal() {
    let region = Region::initialize(
        LocalWords::new(),
        Config {
            identity: Identity {
                session: 1,
                token: 1,
                revision: 1,
                binding: 1,
            },
            sample_rate: 48_000,
            block: 256,
            native_latency: 17,
            kind: Kind::Effect,
        },
    )
    .unwrap();
    let signals = Arc::new(Signals::default());
    let mut audio = Audio::new(region, signals.clone(), &[gain("clap")]).unwrap();
    let mut left = [0.25; 512];
    let mut right = left;
    let mut max_callback = Duration::ZERO;
    let mut total = Duration::ZERO;
    let mut callbacks = 0;
    // Calibration proves this counter observes alloc, realloc and free.
    assert!(
        guarded(|| {
            let mut value = Vec::with_capacity(1);
            value.push(1);
            value.reserve(100);
            std::hint::black_box(value);
        }) >= 3
    );
    for frames in [1, 7, 64, 480, 512] {
        for _ in 0..64 {
            let started = Instant::now();
            let calls = guarded(|| {
                audio.set_param(7, 0.75);
                audio.note_on(60, 1.0);
                audio.note_off(60);
                audio.process(&mut left[..frames], &mut right[..frames]);
            });
            let elapsed = started.elapsed();
            max_callback = max_callback.max(elapsed);
            total += elapsed;
            callbacks += 1;
            assert_eq!(calls, 0);
            assert!(left[..frames].iter().all(|sample| sample.is_finite()));
        }
    }
    assert_eq!(
        guarded(|| {
            for _ in 0..1024 {
                audio.note_on(60, 1.0);
            }
            for _ in 0..8 {
                for key in 0..128 {
                    audio.note_off(key);
                }
                audio.all_notes_off();
            }
            audio.reset_timeline();
            signals.failed.store(true, Ordering::Release);
            audio.process(&mut left, &mut right);
        }),
        0
    );
    // Actual heap/map retirement remains control-side, outside the guard.
    drop(audio);
    eprintln!(
        "bridge callback headless guard: {callbacks} calls, max={}us average={}us, allocator/free calls=0",
        max_callback.as_micros(),
        total.as_micros() / callbacks
    );
}

#[test]
fn native_clap_vst3_healthy_audio_and_state_use_independent_helpers() {
    for format in ["clap", "vst3"] {
        let id = if format == "clap" {
            common::GAIN.to_owned()
        } else {
            vst_id(0)
        };
        let (control, mut audio) = launch(
            format,
            &id,
            Kind::Effect,
            vec![gain(format)],
            Duration::from_secs(2),
        );
        let (other, mut independent) = launch(
            format,
            &id,
            Kind::Effect,
            vec![gain(format)],
            Duration::from_secs(2),
        );
        assert_ne!(control.status().process_id, other.status().process_id);
        assert_eq!(audio.latency(), if format == "clap" { 128 } else { 145 });
        assert!(control.editor(true).unwrap_err().contains("unsupported"));
        assert!(audio.set_param(7, 0.75));
        let mut left = [1.0; 64];
        let mut right = left;
        for _ in 0..8 {
            left.fill(1.0);
            right.fill(1.0);
            assert_eq!(guarded(|| audio.process(&mut left, &mut right)), 0);
            std::thread::sleep(Duration::from_millis(3));
        }
        assert!(audio.health().completed_blocks >= 4, "{:?}", audio.health());
        assert!(audio.health().acknowledged_generation >= 2);
        let expected = if format == "clap" { 0.75 } else { 1.5 };
        assert!(
            left.iter().all(|sample| (*sample - expected).abs() < 1e-7),
            "{format}: {left:?}"
        );
        let captured = control
            .capture(
                1,
                audio.desired_generation(),
                &[ParameterSpec {
                    value: 0.75,
                    ..gain(format)
                }],
                Duration::from_secs(2),
            )
            .unwrap();
        assert!(!captured.state.is_empty());
        assert!(control.last_valid_state().is_some());
        assert!((captured.parameters[0].value - 0.75).abs() < 1e-9);
        let host = PluginHost::windfall();
        let module = host.load(&fixture(format)).unwrap();
        let mut reference = module.create(&id).unwrap();
        reference
            .load_state(&PluginState::from_bytes(captured.state))
            .unwrap();
        assert!((reference.param_value(7).unwrap() - 0.75).abs() < 1e-9);
        let mut native = reference.activate(48_000.0, 64).unwrap();
        left.fill(1.0);
        right.fill(1.0);
        native.process(&mut left, &mut right);
        assert!(left.iter().all(|sample| (*sample - expected).abs() < 1e-7));
        reference.deactivate(native).unwrap();
        // Retiring one process does not end the other owner's process/audio.
        drop(audio);
        drop(control);
        for _ in 0..5 {
            left.fill(1.0);
            right.fill(1.0);
            independent.process(&mut left, &mut right);
            std::thread::sleep(Duration::from_millis(3));
        }
        assert!(!other.status().failed);
        assert!(independent.health().completed_blocks > 0);
        drop(independent);
        drop(other);
    }
}

#[test]
fn unprocessed_capture_intent_is_separate_from_native_dsp_acknowledgement() {
    for format in ["clap", "vst3"] {
        let id = if format == "clap" {
            common::GAIN.to_owned()
        } else {
            vst_id(0)
        };
        let (control, mut audio) = launch(
            format,
            &id,
            Kind::Effect,
            vec![gain(format)],
            Duration::from_secs(2),
        );
        assert!(audio.set_param(7, 0.75));
        let pending = [ParameterSpec {
            value: 0.75,
            ..gain(format)
        }];
        let captured = control
            .capture(
                1,
                audio.desired_generation(),
                &pending,
                Duration::from_secs(2),
            )
            .unwrap();
        assert_eq!(captured.processed_generation, 0);
        assert_eq!(audio.health().acknowledged_generation, 0);
        assert_eq!(captured.parameters[0].value, 0.75);
        assert_eq!(
            captured.reconciled_generation,
            if format == "clap" { 0 } else { 2 }
        );
        let host = PluginHost::windfall();
        let module = host.load(&fixture(format)).unwrap();
        let mut restored = module.create(&id).unwrap();
        restored
            .load_state(&PluginState::from_bytes(captured.state.clone()))
            .unwrap();
        // CLAP opaque state remains actual native state; the pending value is
        // a separate restore companion. VST3 reconciles while inactive.
        assert_eq!(
            restored.param_value(7).unwrap(),
            if format == "clap" { 0.5 } else { 0.75 }
        );
        let malformed = [ParameterSpec {
            id: 123,
            ..pending[0]
        }];
        assert!(
            control
                .capture(1, 2, &malformed, Duration::from_secs(2))
                .is_err()
        );
        assert_eq!(control.last_valid_state().unwrap().state, captured.state);
        assert!(!control.status().failed);
        drop(audio);
        drop(control);
    }
}

#[test]
fn default_block_native_helpers_measure_extra_512_frames() {
    for format in ["clap", "vst3"] {
        let id = if format == "clap" {
            common::GAIN.to_owned()
        } else {
            vst_id(0)
        };
        let mut options = options(
            format,
            &id,
            Kind::Effect,
            vec![gain(format)],
            Duration::from_secs(2),
        );
        options.config.block = windfall_plugin_host::bridge::protocol::DEFAULT_BLOCK;
        options.offline = true;
        let (control, mut audio, _) = supervisor::launch(options).unwrap();
        let mut left = [0.0; 1024];
        let mut right = left;
        left[0] = 1.0;
        right[0] = 1.0;
        audio
            .process_offline(
                &mut left,
                &mut right,
                Instant::now() + Duration::from_secs(2),
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(left.iter().position(|value| *value != 0.0), Some(512));
        assert_eq!(left[512], if format == "clap" { 0.5 } else { 1.0 });
        assert_eq!(audio.latency(), if format == "clap" { 512 } else { 529 });
        assert_eq!(audio.health().missed_blocks, 0);
        drop(audio);
        drop(control);
    }
}

#[test]
fn native_process_crash_nan_and_stall_leave_parent_alive_and_dry_bounded() {
    for (id, timeout) in [
        (common::PROCESS_PANIC, Duration::from_secs(1)),
        (common::NAN, Duration::from_secs(1)),
        (common::SLOW, Duration::from_millis(10)),
    ] {
        let (control, mut audio) = launch("clap", id, Kind::Effect, vec![], timeout);
        let mut left = [0.25; 64];
        let mut right = left;
        audio.process(&mut left, &mut right);
        wait_failed(&control);
        for _ in 0..6 {
            left.fill(0.25);
            right.fill(0.25);
            assert_eq!(guarded(|| audio.process(&mut left, &mut right)), 0);
        }
        assert_eq!(left, [0.25; 64]);
        assert_eq!(audio.health().acknowledged_generation, 0);
        assert!(
            control
                .capture(1, 0, &[], Duration::from_millis(100))
                .is_err()
        );
        drop(audio);
        drop(control);
    }
}

#[test]
fn native_vst3_error_and_nan_do_not_escape_the_process_boundary() {
    for index in [4, 5] {
        let (control, mut audio) = launch(
            "vst3",
            &vst_id(index),
            Kind::Effect,
            vec![],
            Duration::from_secs(1),
        );
        let mut left = [0.5; 64];
        let mut right = left;
        audio.process(&mut left, &mut right);
        wait_failed(&control);
        for _ in 0..5 {
            left.fill(0.5);
            right.fill(0.5);
            assert_eq!(guarded(|| audio.process(&mut left, &mut right)), 0);
        }
        assert_eq!(left, [0.5; 64]);
        drop(audio);
        drop(control);
    }
}

#[test]
fn independent_offline_helpers_match_native_audio_for_unsplit_variable_callbacks() {
    for format in ["clap", "vst3"] {
        for frames in [1, 7, 64, 480, 512] {
            let id = if format == "clap" {
                common::GAIN.to_owned()
            } else {
                vst_id(0)
            };
            let mut options = options(
                format,
                &id,
                Kind::Effect,
                vec![gain(format)],
                Duration::from_secs(2),
            );
            options.offline = true;
            let (control, mut audio, _) = supervisor::launch(options).unwrap();
            let host = PluginHost::windfall();
            let module = host.load(&fixture(format)).unwrap();
            let mut reference = module.create(&id).unwrap();
            assert!(reference.set_param(7, 0.5));
            let mut native = reference.activate(48_000.0, 64).unwrap();
            native.set_realtime(false);
            let mut native_left: Vec<f32> =
                (0..4096).map(|index| (index % 31) as f32 / 31.0).collect();
            let mut native_right: Vec<f32> = native_left.iter().map(|value| -*value).collect();
            native.process(&mut native_left, &mut native_right);
            reference.deactivate(native).unwrap();
            let cancelled = std::sync::atomic::AtomicBool::new(false);
            let mut result = vec![];
            for source in (0..4096).step_by(frames) {
                let count = frames.min(4096 - source);
                let mut left = vec![0.0; count];
                let mut right = left.clone();
                for (index, (left, right)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
                    let input = ((source + index) % 31) as f32 / 31.0;
                    *left = input;
                    *right = -input;
                }
                audio
                    .process_offline(
                        &mut left,
                        &mut right,
                        Instant::now() + Duration::from_secs(2),
                        &cancelled,
                    )
                    .unwrap();
                result.extend_from_slice(&left);
            }
            // Measure actual added bridge delay, independently of a native
            // fixture's advertised latency. This VST3 fixture advertises 17
            // but intentionally has no native audio delay implementation.
            let extra = 128;
            assert!(result[..extra].iter().all(|sample| *sample == 0.0));
            for (index, actual) in result[extra..].iter().enumerate() {
                let expected = native_left[index];
                assert!(
                    (*actual - expected).abs() < 1e-7,
                    "{format} callback {frames}, sample {index}: {actual} vs {expected}"
                );
            }
            assert_eq!(audio.health().missed_blocks, 0);
            assert!(audio.health().completed_blocks > 0);
            drop(audio);
            drop(control);
        }
    }
}

#[test]
fn clap_instrument_measures_real_native_plus_bridge_latency_and_note_release() {
    let parameter = ParameterSpec {
        id: 1,
        min: 0.0,
        max: 1.0,
        value: 0.5,
        read_only: false,
        stepped: false,
    };
    let mut options = options(
        "clap",
        common::SINE,
        Kind::Instrument,
        vec![parameter],
        Duration::from_secs(2),
    );
    options.offline = true;
    let (control, mut audio, _) = supervisor::launch(options).unwrap();
    assert_eq!(audio.latency(), 160);
    assert!(audio.note_on(69, 1.0));
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let mut left = [0.0; 512];
    let mut right = left;
    audio
        .process_offline(
            &mut left,
            &mut right,
            Instant::now() + Duration::from_secs(2),
            &cancelled,
        )
        .unwrap();
    // A sine begins at zero; first nonzero is one frame after the exact onset.
    assert_eq!(
        left.iter().position(|sample| sample.abs() > 1e-6),
        Some(161)
    );
    assert!(audio.note_off(69));
    for _ in 0..3 {
        audio
            .process_offline(
                &mut left,
                &mut right,
                Instant::now() + Duration::from_secs(2),
                &cancelled,
            )
            .unwrap();
    }
    assert!(left.iter().all(|sample| *sample == 0.0));
    assert_eq!(audio.voices(), 0);
    drop(audio);
    drop(control);
}
