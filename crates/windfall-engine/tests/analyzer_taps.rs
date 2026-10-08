//! Public production-interface tests; fixture DFT and PCM are authored here.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};
use std::time::{Duration, Instant};

use windfall_engine::analyzers::*;
use windfall_project::{EffectId, TrackId};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 4]> = const { Cell::new([0; 4]) };
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
    static BUFFER_CREDIT: Cell<usize> = const { Cell::new(0) };
    static EARLY_CREDIT_RELEASE: Cell<bool> = const { Cell::new(false) };
}

struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn count(kind: usize, delta: isize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        if delta <= -4096 {
            let required = BUFFER_CREDIT.try_with(Cell::get).unwrap_or(0);
            if required > 0 && usage().reserved_bytes < required {
                let _ = EARLY_CREDIT_RELEASE.try_with(|flag| flag.set(true));
            }
        }
        let _ = CALLS.try_with(|calls| {
            let mut c = calls.get();
            c[kind] += 1;
            calls.set(c);
        });
        let _ = LIVE.try_with(|live| {
            let now = live.get() + delta;
            live.set(now);
            let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
        });
    }
}

// SAFETY: all allocation contracts are forwarded unchanged to System; only
// plain TLS counters are touched in addition. No external code is copied.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count(0, layout.size() as isize);
        // SAFETY: forwarded GlobalAlloc contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count(1, layout.size() as isize);
        // SAFETY: forwarded GlobalAlloc contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        count(2, -(layout.size() as isize));
        // SAFETY: forwarded GlobalAlloc contract.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count(3, new_size as isize - layout.size() as isize);
        // SAFETY: forwarded GlobalAlloc contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn allocations(work: impl FnOnce()) -> ([usize; 4], isize, isize) {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            WATCH.set(false);
            BUFFER_CREDIT.set(0);
        }
    }
    CALLS.set([0; 4]);
    LIVE.set(0);
    PEAK.set(0);
    EARLY_CREDIT_RELEASE.set(false);
    WATCH.set(true);
    let guard = Guard;
    work();
    drop(guard);
    (CALLS.get(), LIVE.get(), PEAK.get())
}

fn selection() -> Selection {
    Selection {
        project: ProjectGeneration(3),
        generation: SelectionGeneration(7),
        source: Source::MixOutput,
    }
}
fn stamp(first_frame: u64) -> Stamp {
    Stamp {
        selection: selection(),
        clock: ClockEpoch(2),
        first_frame,
        sample_rate: 48_000,
        pdc_frames: 12,
        device_latency_frames: Some(480),
        gain_reduction: None,
    }
}
fn config(n: usize, window: Window) -> Config {
    Config {
        fft_size: n,
        hop_size: n,
        window,
        input_packets: 16,
        snapshot_capacity: 2,
    }
}
fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}, tolerance {tolerance}"
    );
}
fn sine(n: usize, cycles: f64, gain: f64) -> Vec<[f32; 2]> {
    (0..n)
        .map(|i| {
            let s = (gain * (TAU * cycles * i as f64 / n as f64).sin()) as f32;
            [s, s]
        })
        .collect()
}
fn feed(tap: &mut AudioTap, worker: &mut AnalyzerWorker, pcm: &[[f32; 2]], base: u64) {
    for (i, block) in pcm.chunks(MAX_PUBLICATION_FRAMES).enumerate() {
        tap.publish(stamp(base + (i * MAX_PUBLICATION_FRAMES) as u64), block)
            .unwrap();
        worker.pump();
    }
}

/// Independent O(N^2) real DFT, returning complex bins. No RustFFT and no
/// implementation helper/window coefficients are used in this reference.
fn dft(pcm: &[[f32; 2]], window: Window, side: usize) -> Vec<(f64, f64)> {
    let n = pcm.len();
    (0..=n / 2)
        .map(|k| {
            pcm.iter()
                .enumerate()
                .fold((0.0, 0.0), |(re, im), (t, frame)| {
                    let w = match window {
                        Window::Rectangular => 1.0,
                        Window::PeriodicHann => {
                            (std::f64::consts::PI * t as f64 / n as f64).sin().powi(2)
                        }
                    };
                    let angle = TAU * (k * t) as f64 / n as f64;
                    let x = f64::from(frame[side]) * w;
                    (re + x * angle.cos(), im - x * angle.sin())
                })
        })
        .collect()
}

#[test]
fn sine_sample_peak_rms_dbfs_and_bin_calibration() {
    for window in [Window::Rectangular, Window::PeriodicHann] {
        let (mut tap, mut worker, mut reader) = prepare(selection(), config(256, window)).unwrap();
        feed(&mut tap, &mut worker, &sine(256, 8.0, 0.5), 0);
        let s = reader.poll(selection()).unwrap();
        for side in 0..2 {
            let l = s.channels[side].levels.unwrap();
            near(l.sample_peak, 0.5, 1e-7);
            near(l.rms, 0.5 * FRAC_1_SQRT_2, 1e-7);
            assert_eq!(s.channels[side].clip_samples, 0);
            let Dbfs::Finite(db) = l.rms_dbfs else {
                panic!("valid rms")
            };
            near(db, -9.03089987, 1e-6);
            near(s.spectrum[8].coherent_amplitude[side], 0.5, 1e-7);
            near(
                s.spectrum.iter().map(|bin| bin.mean_square[side]).sum(),
                0.125,
                1e-7,
            );
        }
        near(
            s.calibration.coherent_gain,
            if window == Window::Rectangular {
                1.0
            } else {
                0.5
            },
            1e-14,
        );
        near(
            s.calibration.enbw_bins,
            if window == Window::Rectangular {
                1.0
            } else {
                1.5
            },
            1e-14,
        );
        near(
            s.calibration.mean_square_gain,
            if window == Window::Rectangular {
                1.0
            } else {
                3.0 / 8.0
            },
            1e-14,
        );
        near(s.stereo.correlation.unwrap(), 1.0, 1e-14);
    }
}

#[test]
fn fractional_bin_window_leakage_and_scalar_reference() {
    let n = 128;
    let pcm = sine(n, 11.5, 0.8);
    let mut distant_power = [0.0; 2];
    for (index, window) in [Window::Rectangular, Window::PeriodicHann]
        .into_iter()
        .enumerate()
    {
        let reference = dft(&pcm, window, 0);
        let (mut tap, mut worker, mut reader) = prepare(selection(), config(n, window)).unwrap();
        feed(&mut tap, &mut worker, &pcm, 900);
        let s = reader.poll(selection()).unwrap();
        for (k, &(re, im)) in reference.iter().enumerate() {
            let bin = &s.spectrum[k];
            let scalar_magnitude = re.hypot(im);
            let coherent_sum = n as f64
                * if window == Window::Rectangular {
                    1.0
                } else {
                    0.5
                };
            let reconstructed_magnitude = bin.coherent_amplitude[0] * coherent_sum
                / if k == 0 || k == n / 2 { 1.0 } else { 2.0 };
            near(reconstructed_magnitude, scalar_magnitude, 2e-12 * n as f64);
            if scalar_magnitude > 1e-8 {
                let phase_error = (bin.phase_radians[0] - im.atan2(re) + std::f64::consts::PI)
                    .rem_euclid(TAU)
                    - std::f64::consts::PI;
                assert!(phase_error.abs() < 1e-9);
            }
            if !(8..=15).contains(&k) {
                distant_power[index] += bin.mean_square[0];
            }
        }
        let max = s
            .spectrum
            .iter()
            .map(|b| b.coherent_amplitude[0])
            .fold(0.0_f64, f64::max);
        assert!(
            max < 0.8 && max > 0.45,
            "fractional-bin scalloping is explicit"
        );
    }
    assert!(distant_power[1] < distant_power[0] / 100.0);
}

#[test]
fn impulse_dc_nyquist_silence_noise_and_log_sweep_energy() {
    let n = 64;
    let mut random = 0x9182_7ab3_u32;
    let noise: Vec<_> = (0..n)
        .map(|_| {
            let mut side = || {
                random ^= random << 13;
                random ^= random >> 17;
                random ^= random << 5;
                ((random as f64 / u32::MAX as f64 - 0.5) * 0.8) as f32
            };
            [side(), side()]
        })
        .collect();
    let sweep: Vec<_> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let phase = TAU * 2.0 * (12.0_f64.powf(t) - 1.0) / 12.0_f64.ln();
            [(0.3 * phase.sin()) as f32, (0.2 * phase.cos()) as f32]
        })
        .collect();
    let mut impulse = vec![[0.0; 2]; n];
    impulse[17] = [1.0, -0.5];
    for pcm in [
        vec![[0.25, -0.75]; n],
        (0..n)
            .map(|i| [if i % 2 == 0 { 0.6 } else { -0.6 }, 0.0])
            .collect(),
        vec![[0.0; 2]; n],
        impulse,
        noise,
        sweep,
    ] {
        for window in [Window::Rectangular, Window::PeriodicHann] {
            let (mut tap, mut worker, mut reader) =
                prepare(selection(), config(n, window)).unwrap();
            feed(&mut tap, &mut worker, &pcm, 0);
            let s = reader.poll(selection()).unwrap();
            assert_eq!(s.spectrum.len(), n / 2 + 1);
            for side in 0..2 {
                let reference = dft(&pcm, window, side);
                let squared_weight_sum: f64 = (0..n)
                    .map(|i| {
                        if window == Window::Rectangular {
                            1.0
                        } else {
                            (std::f64::consts::PI * i as f64 / n as f64).sin().powi(4)
                        }
                    })
                    .sum();
                let integrated: f64 = s.spectrum.iter().map(|b| b.mean_square[side]).sum();
                let windowed_energy: f64 = pcm
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let w = if window == Window::Rectangular {
                            1.0
                        } else {
                            (std::f64::consts::PI * i as f64 / n as f64).sin().powi(2)
                        };
                        (f64::from(f[side]) * w).powi(2)
                    })
                    .sum::<f64>()
                    / squared_weight_sum;
                near(integrated, windowed_energy, 1e-12);
                for (k, &(re, im)) in reference.iter().enumerate() {
                    let factor = if k == 0 || k == n / 2 { 1.0 } else { 2.0 };
                    near(
                        s.spectrum[k].coherent_amplitude[side],
                        factor * re.hypot(im)
                            / (n as f64
                                * if window == Window::Rectangular {
                                    1.0
                                } else {
                                    0.5
                                }),
                        1e-12,
                    );
                }
            }
            if pcm.iter().all(|f| f[0] == 0.25) {
                near(s.spectrum[0].coherent_amplitude[0], 0.25, 1e-12);
            }
            if pcm[0][0] == 0.6 {
                near(
                    s.spectrum[n / 2].coherent_amplitude[0],
                    f64::from(0.6_f32),
                    1e-12,
                );
            }
            if pcm.iter().flatten().all(|&v| v == 0.0) {
                assert_eq!(s.channels[0].levels.unwrap().rms_dbfs, Dbfs::Silence);
                assert_eq!(s.stereo.correlation, None);
            }
        }
    }
}

#[test]
fn anti_phase_distinct_channels_vectors_and_invalid_pcm_are_explicit() {
    for anti in [false, true] {
        let mut pcm = sine(128, 7.0, 0.5);
        if anti {
            for f in &mut pcm {
                f[1] = -f[0];
            }
        }
        let (mut tap, mut worker, mut reader) =
            prepare(selection(), config(128, Window::Rectangular)).unwrap();
        feed(&mut tap, &mut worker, &pcm, 0);
        let s = reader.poll(selection()).unwrap();
        near(
            s.stereo.correlation.unwrap(),
            if anti { -1.0 } else { 1.0 },
            1e-14,
        );
        near(
            if anti {
                s.stereo.mid_rms.unwrap()
            } else {
                s.stereo.side_rms.unwrap()
            },
            0.0,
            1e-14,
        );
        assert!(s.vectorscope.iter().all(|p| p[usize::from(!anti)] == 0.0));
    }
    let mut pcm = sine(128, 3.0, 0.5);
    for (i, f) in pcm.iter_mut().enumerate() {
        f[1] = (0.3 * (TAU * 13.0 * i as f64 / 128.0).sin()) as f32;
    }
    let (mut tap, mut worker, mut reader) =
        prepare(selection(), config(128, Window::PeriodicHann)).unwrap();
    feed(&mut tap, &mut worker, &pcm, 0);
    let s = reader.poll(selection()).unwrap();
    near(s.stereo.correlation.unwrap(), 0.0, 1e-7);
    near(s.spectrum[3].coherent_amplitude[0], 0.5, 1e-7);
    near(s.spectrum[13].coherent_amplitude[1], 0.3, 1e-7);
    pcm[1][0] = f32::NAN;
    pcm[2][0] = f32::INFINITY;
    pcm[3][0] = f32::NEG_INFINITY;
    pcm[4][1] = 1.25;
    pcm[5][1] = -1.0;
    feed(&mut tap, &mut worker, &pcm, 128);
    let s = reader.poll(selection()).unwrap();
    assert_eq!(s.channels[0].invalid_samples, 3);
    assert_eq!(s.channels[0].levels, None);
    assert_eq!(s.spectrum_valid, [false, true]);
    assert!(s.spectrum.iter().all(|b| b.mean_square[0].is_nan()));
    assert_eq!(s.stereo.correlation, None);
    assert_eq!(s.channels[1].clip_samples, 2);
    assert_eq!(
        s.envelope.iter().map(|e| e.invalid_samples[0]).sum::<u32>(),
        3
    );
    assert_eq!(reader.counters().invalid_samples, 3);
}

#[test]
fn variable_partitions_preserve_hop_clocks_and_bounded_histories() {
    let mut reference = None;
    for block in [1, 7, 64, 127, 256] {
        let mut c = config(128, Window::PeriodicHann);
        c.hop_size = 32;
        c.snapshot_capacity = 8;
        let (mut tap, mut worker, mut reader) = prepare(selection(), c).unwrap();
        let pcm = sine(1024, 23.0, 0.4);
        let mut end = 0;
        for chunk in pcm.chunks(block) {
            tap.publish(stamp(end), chunk).unwrap();
            end += chunk.len() as u64;
            worker.pump();
            reader.poll(selection());
        }
        let s = reader.poll(selection()).unwrap();
        assert_eq!((s.range.first_frame, s.range.end_frame), (896, 1024));
        assert_eq!(s.history_first_frame, 768);
        assert_eq!(s.history.len(), 256);
        assert_eq!(s.envelope.len(), 128);
        assert_eq!(s.envelope.first().unwrap().first_frame, 768);
        assert_eq!(s.envelope.last().unwrap().end_frame, 1024);
        assert_eq!(s.spectrogram_slices().len(), SPECTROGRAM_SLICES);
        assert!(
            s.spectrogram_slices()
                .windows(2)
                .all(|p| p[1].range.end_frame - p[0].range.end_frame == 32)
        );
        let result = (
            s.channels,
            s.spectrum
                .iter()
                .map(|b| b.coherent_amplitude)
                .collect::<Vec<_>>(),
        );
        if let Some(old) = &reference {
            assert_eq!(old, &result);
        } else {
            reference = Some(result);
        }
        assert_eq!(reader.counters().analyzed_windows, 29);
    }
}

#[test]
fn queue_overrun_and_stalled_ui_are_bounded_and_counted() {
    let mut c = config(32, Window::Rectangular);
    c.input_packets = 1;
    c.snapshot_capacity = 1;
    let (mut tap, mut worker, mut reader) = prepare(selection(), c).unwrap();
    let pcm = [[0.5; 2]; 32];
    tap.publish(stamp(0), &pcm).unwrap();
    assert_eq!(tap.publish(stamp(32), &pcm), Err(Refusal::QueueFull));
    assert_eq!(worker.pump().stale_packets, 1);
    assert!(reader.poll(selection()).is_none());
    for base in [64, 96, 128, 160] {
        tap.publish(stamp(base), &pcm).unwrap();
        worker.pump();
    }
    assert_eq!(reader.counters().dropped_packets, 1);
    assert_eq!(reader.counters().dropped_frames, 32);
    assert_eq!(reader.counters().stale_frames, 32);
    assert_eq!(reader.counters().published_frames, 160);
    assert_eq!(reader.counters().consumed_frames, 128);
    assert_eq!(reader.counters().dropped_snapshots, 3);
    assert_eq!(reader.poll(selection()).unwrap().range.first_frame, 64);
    tap.publish(stamp(192), &pcm).unwrap();
    worker.pump();
    let s = reader.poll(selection()).unwrap();
    assert_eq!(s.range.first_frame, 192);
    assert_eq!(s.spectrogram_slices().len(), 5);
    assert_eq!(reader.layout().snapshot_pool, 3);
    assert_eq!(reader.counters().pool_faults, 0);
}

#[test]
fn gaps_reset_rates_latency_and_selection_replacement_clear_stale_evidence() {
    let c = config(64, Window::Rectangular);
    let (mut tap, mut worker, mut reader) = prepare(selection(), c).unwrap();
    let pcm = [[0.25; 2]; 64];
    feed(&mut tap, &mut worker, &pcm, 1000);
    assert!(reader.poll(selection()).is_some());
    tap.reset().unwrap();
    assert!(reader.poll(selection()).is_none());
    worker.pump();
    tap.publish(stamp(1064), &pcm[..32]).unwrap();
    worker.pump();
    tap.publish(stamp(1200), &pcm[..32]).unwrap();
    worker.pump();
    assert!(reader.poll(selection()).is_none());
    tap.publish(stamp(1232), &pcm[..32]).unwrap();
    worker.pump();
    let s = reader.poll(selection()).unwrap();
    assert_eq!(s.range.first_frame, 1200);
    assert_eq!(s.history.len(), 64);
    assert_eq!(s.spectrogram_slices().len(), 1);
    let mut different_rate = stamp(1264);
    different_rate.sample_rate = 96_000;
    tap.publish(different_rate, &pcm).unwrap();
    assert!(reader.poll(selection()).is_none());
    worker.pump();
    let s = reader.poll(selection()).unwrap();
    assert_eq!(s.range.sample_rate, 96_000);
    assert_eq!(s.history_first_frame, 1264);
    different_rate.first_frame = 1328;
    different_rate.pdc_frames = 24;
    tap.publish(different_rate, &pcm).unwrap();
    worker.pump();
    assert_eq!(reader.poll(selection()).unwrap().history.len(), 64);
    let replacement = Selection {
        project: ProjectGeneration(4),
        generation: SelectionGeneration(8),
        source: Source::TrackPostFader(TrackId(4)),
    };
    assert!(reader.poll(replacement).is_none());
    let bad_stamp = Stamp {
        selection: replacement,
        ..stamp(1392)
    };
    assert_eq!(tap.publish(bad_stamp, &pcm), Err(Refusal::WrongSelection));
    reader.cancel();
    assert_eq!(tap.publish(stamp(1392), &pcm), Err(Refusal::Cancelled));
}

#[test]
fn clocks_keep_integer_frame_precision_and_explicit_latency_labels() {
    let (mut tap, mut worker, mut reader) =
        prepare(selection(), config(32, Window::Rectangular)).unwrap();
    let pcm = [[0.1; 2]; 32];
    let first = (1_u64 << 54) + 31;
    feed(&mut tap, &mut worker, &pcm, first);
    let r = reader.poll(selection()).unwrap().range;
    assert_eq!(r.first_frame, first);
    assert_eq!(r.end_frame, first + 32);
    assert_eq!(r.pdc_aligned_first_frame(), i128::from(first) - 12);
    assert_eq!(
        r.estimated_heard_first_frame(),
        Some(i128::from(first) - 492)
    );
    assert_eq!(
        ClockRange {
            first_frame: 48_012,
            ..r
        }
        .pdc_time_label(),
        "00:00:01.000"
    );
    assert_eq!(
        ClockRange {
            first_frame: 0,
            ..r
        }
        .pdc_time_label(),
        "-00:00:00.000"
    );
    assert_eq!(
        ClockRange {
            device_latency_frames: None,
            ..r
        }
        .estimated_heard_first_frame(),
        None
    );
    assert_eq!(
        tap.publish(stamp(u64::MAX - 16), &pcm),
        Err(Refusal::FrameOverflow)
    );
    assert!(reader.poll(selection()).is_none());
}

#[test]
fn actual_effect_metadata_is_tagged_and_never_inferred() {
    let (mut tap, mut worker, mut reader) =
        prepare(selection(), config(64, Window::Rectangular)).unwrap();
    let pcm = [[0.01; 2]; 32];
    let mut metadata = stamp(0);
    metadata.gain_reduction = Some(GainReduction {
        effect: EffectId(23),
        db: 4.0,
    });
    tap.publish(metadata, &pcm).unwrap();
    metadata.first_frame = 32;
    metadata.gain_reduction.as_mut().unwrap().db = 9.0;
    tap.publish(metadata, &pcm).unwrap();
    worker.pump();
    assert_eq!(
        reader.poll(selection()).unwrap().gain_reduction,
        Some(GainReduction {
            effect: EffectId(23),
            db: 9.0
        })
    );
    metadata.first_frame = 64;
    metadata.gain_reduction.as_mut().unwrap().db = f32::NAN;
    tap.publish(metadata, &pcm).unwrap();
    tap.publish(stamp(96), &pcm).unwrap();
    worker.pump();
    let s = reader.poll(selection()).unwrap();
    assert_eq!(s.gain_reduction, None);
    assert_eq!(s.invalid_metadata, 32);
    assert_eq!(reader.counters().invalid_metadata, 1);
}

#[test]
fn preparation_bounds_preserve_old_path_and_aggregate_credit_until_all_owners_leave() {
    let before = usage();
    let mut measured = None;
    let (_, live, peak) = allocations(|| {
        measured = Some(prepare(selection(), Config::default()).unwrap());
    });
    let (mut old_tap, mut old_worker, mut old_reader) = measured.take().unwrap();
    assert!(live > 0 && peak >= live);
    assert!(peak as usize <= old_reader.layout().reserved_bytes);
    assert_eq!(usage().taps, before.taps + 1);
    for invalid in [
        Config {
            fft_size: 31,
            ..Config::default()
        },
        Config {
            fft_size: 8192,
            ..Config::default()
        },
        Config {
            hop_size: 17,
            ..Config::default()
        },
        Config {
            input_packets: 65,
            ..Config::default()
        },
        Config {
            snapshot_capacity: 9,
            ..Config::default()
        },
    ] {
        assert!(prepare(selection(), invalid).is_err());
    }
    let mut kept = Vec::new();
    let huge = Config {
        fft_size: 4096,
        hop_size: 4096,
        window: Window::Rectangular,
        input_packets: 64,
        snapshot_capacity: 8,
    };
    let mut measured_maximum = None;
    let (_, maximum_live, maximum_peak) = allocations(|| {
        measured_maximum = Some(prepare(selection(), huge).unwrap());
    });
    assert!(maximum_live > 0 && maximum_peak >= maximum_live);
    assert!(maximum_peak as usize <= huge.layout().unwrap().reserved_bytes);
    drop(measured_maximum);
    let error = loop {
        match prepare(selection(), huge) {
            Ok(prepared) => kept.push(prepared),
            Err(e) => break e,
        }
    };
    assert_eq!(error, PrepareError::ByteLimit);
    assert!(usage().reserved_bytes <= MAX_RETAINED_BYTES);
    drop(kept);
    let pcm = sine(1024, 16.0, 0.25);
    feed(&mut old_tap, &mut old_worker, &pcm, 0);
    assert!(old_reader.poll(selection()).is_some());
    drop(old_tap);
    drop(old_worker);
    assert_eq!(usage().taps, before.taps + 1, "reader retains pool credit");
    drop(old_reader);
    assert_eq!(usage(), before);
    // Observe public quota during actual large-buffer reclamation, with the
    // deterministic worker as the final owner (no thread handle retains credit).
    let (tap, worker, reader) = prepare(selection(), Config::default()).unwrap();
    let required_credit = usage().reserved_bytes;
    drop(tap);
    drop(reader);
    BUFFER_CREDIT.set(required_credit);
    let (calls, _, _) = allocations(|| drop(worker));
    assert!(calls[2] > 0, "worker storage was actually reclaimed");
    assert!(!EARLY_CREDIT_RELEASE.get(), "buffers outlived quota credit");
    assert_eq!(usage(), before);
}

#[test]
fn instance_and_installation_bounds_refuse_without_unbounded_backlog() {
    let mut kept = Vec::new();
    for _ in 0..MAX_LIVE_TAPS {
        kept.push(prepare(selection(), config(32, Window::Rectangular)).unwrap());
    }
    assert!(matches!(
        prepare(selection(), config(32, Window::Rectangular)),
        Err(PrepareError::InstanceLimit)
    ));
    drop(kept);
    let mut slots = Vec::new();
    for _ in 0..MAX_TAP_SLOTS {
        slots.push(tap_slot().unwrap());
    }
    assert!(matches!(tap_slot(), Err(PrepareError::SlotLimit)));
    drop(slots);
}

#[test]
fn callback_paths_allocate_reallocate_and_free_zero_including_replacement_backpressure() {
    let (mut tap, mut worker, mut reader) = prepare(
        selection(),
        Config {
            input_packets: 1,
            ..config(32, Window::Rectangular)
        },
    )
    .unwrap();
    let pcm = [[0.1; 2]; 32];
    let huge = [[f32::NAN; 2]; 257];
    let mut results = [Ok(()); 8];
    let (calls, _, _) = allocations(|| {
        results[0] = tap.publish(stamp(0), &pcm);
        results[1] = tap.publish(stamp(32), &pcm);
        results[2] = tap.reset();
        results[3] = tap.publish(stamp(64), &huge);
        results[4] = tap.publish(
            Stamp {
                sample_rate: 0,
                ..stamp(64)
            },
            &pcm,
        );
        results[5] = tap.publish(
            Stamp {
                selection: Selection {
                    generation: SelectionGeneration(8),
                    ..selection()
                },
                ..stamp(64)
            },
            &pcm,
        );
        results[6] = tap.publish(stamp(u64::MAX), &pcm);
        results[7] = tap.publish(stamp(96), &pcm);
    });
    assert_eq!(calls, [0; 4]);
    assert_eq!(results[1], Err(Refusal::QueueFull));
    worker.pump();
    reader.cancel();
    assert_eq!(
        allocations(|| {
            let _ = tap.publish(stamp(128), &pcm);
        })
        .0,
        [0; 4]
    );
    drop(tap);
    drop(worker);
    drop(reader);
    let (mut slot, mut installer) = tap_slot().unwrap();
    let mut readers = Vec::new();
    let mut workers = Vec::new();
    for generation in 0..3 {
        let selected = Selection {
            generation: SelectionGeneration(generation),
            ..selection()
        };
        let (tap, worker, reader) = prepare(selected, config(32, Window::Rectangular)).unwrap();
        assert!(installer.stage(tap).is_ok());
        workers.push(worker);
        readers.push(reader);
        let mut installed = false;
        assert_eq!(
            allocations(|| {
                installed = slot.boundary();
            })
            .0,
            [0; 4]
        );
        assert_eq!(installed, generation < 2);
    }
    assert_eq!(installer.deferred_installs(), 1);
    assert!(readers[0].is_cancelled());
    assert_eq!(
        slot.active().unwrap().selection().generation,
        SelectionGeneration(1)
    );
    assert!(installer.collect_retired());
    assert_eq!(
        allocations(|| {
            assert!(slot.boundary());
        })
        .0,
        [0; 4]
    );
    assert!(readers[1].is_cancelled());
    assert!(installer.collect_retired());
    // These heap-owning endpoints are all destroyed on the test/control thread.
    drop(slot);
    drop(installer);
    drop(workers);
    drop(readers);
}

#[test]
fn native_worker_starts_publishes_and_joins_on_shutdown_or_drop() {
    let before = usage();
    for explicit in [true, false] {
        let (mut tap, worker, mut reader) =
            prepare(selection(), config(64, Window::PeriodicHann)).unwrap();
        let native = worker.spawn().unwrap();
        tap.publish(stamp(0), &sine(64, 8.0, 0.5)).unwrap();
        let started = Instant::now();
        while reader.poll(selection()).is_none() {
            assert!(started.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(1));
        }
        near(
            reader.poll(selection()).unwrap().spectrum[8].coherent_amplitude[0],
            0.5,
            1e-7,
        );
        if explicit {
            native.shutdown().unwrap();
        } else {
            drop(native);
        }
        assert!(reader.is_cancelled());
        assert!(reader.poll(selection()).is_none());
        assert_eq!(
            tap.publish(stamp(64), &[[0.0; 2]; 64]),
            Err(Refusal::Cancelled)
        );
        drop(reader);
        drop(tap);
    }
    assert_eq!(usage(), before);
}

#[test]
fn staged_sources_and_deselection_retire_without_stale_successor_results_or_callback_frees() {
    let (mut slot, mut installer) = tap_slot().unwrap();
    let a = selection();
    let b = Selection {
        generation: SelectionGeneration(8),
        source: Source::TrackPostFader(TrackId(4)),
        ..a
    };
    let c = Selection {
        project: ProjectGeneration(4),
        generation: SelectionGeneration(9),
        ..a
    };
    let (tap_a, mut worker_a, mut reader_a) = prepare(a, config(64, Window::Rectangular)).unwrap();
    let (tap_b, mut worker_b, mut reader_b) = prepare(b, config(64, Window::Rectangular)).unwrap();
    let (tap_c, mut worker_c, mut reader_c) = prepare(c, config(64, Window::Rectangular)).unwrap();
    assert!(installer.stage(tap_a).is_ok());
    assert!(slot.boundary());
    slot.active()
        .unwrap()
        .publish(stamp(0), &[[0.1; 2]; 64])
        .unwrap();
    worker_a.pump();
    assert!(reader_a.poll(a).is_some());
    assert!(installer.stage(tap_b).is_ok());
    let tap_c = match installer.stage(tap_c) {
        Err(tap) => tap,
        Ok(()) => panic!("full staging queue"),
    };
    assert!(reader_a.poll(a).is_some());
    assert!(!reader_c.is_cancelled());
    assert_eq!(
        allocations(|| {
            assert!(slot.boundary());
        })
        .0,
        [0; 4]
    );
    assert!(reader_a.poll(a).is_none());
    assert!(reader_a.poll(b).is_none());
    assert!(installer.stage(tap_c).is_ok());
    assert!(!installer.clear());
    assert_eq!(
        allocations(|| {
            assert!(!slot.boundary());
        })
        .0,
        [0; 4]
    );
    slot.active()
        .unwrap()
        .publish(
            Stamp {
                selection: b,
                ..stamp(64)
            },
            &[[0.2; 2]; 64],
        )
        .unwrap();
    worker_b.pump();
    assert_eq!(reader_b.poll(b).unwrap().selection, b);
    assert!(installer.collect_retired());
    assert_eq!(
        allocations(|| {
            assert!(slot.boundary());
        })
        .0,
        [0; 4]
    );
    assert!(reader_b.poll(b).is_none());
    slot.active()
        .unwrap()
        .publish(
            Stamp {
                selection: c,
                ..stamp(128)
            },
            &[[0.3; 2]; 64],
        )
        .unwrap();
    worker_c.pump();
    assert_eq!(reader_c.poll(c).unwrap().selection, c);
    assert!(installer.clear());
    assert!(!slot.boundary());
    assert!(installer.collect_retired());
    assert_eq!(
        allocations(|| {
            assert!(slot.boundary());
        })
        .0,
        [0; 4]
    );
    assert!(slot.active().is_none());
    assert!(reader_c.poll(c).is_none());
    assert!(installer.collect_retired());
    assert!(!installer.collect_retired());
}

#[test]
fn native_worker_with_reader_stalled_keeps_a_finite_snapshot_pool() {
    let mut c = config(64, Window::Rectangular);
    c.snapshot_capacity = 1;
    let (mut tap, worker, mut reader) = prepare(selection(), c).unwrap();
    let native = worker.spawn().unwrap();
    tap.publish(stamp(0), &[[0.25; 2]; 256]).unwrap();
    let start = Instant::now();
    while reader.counters().analyzed_windows < 4 {
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1));
    }
    // Allow the last publication decision after its analysis counter store.
    while reader.counters().dropped_snapshots < 3 {
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(reader.poll(selection()).unwrap().range.end_frame, 64);
    assert_eq!(reader.counters().dropped_snapshots, 3);
    assert_eq!(reader.layout().snapshot_pool, 3);
    native.shutdown().unwrap();
    assert!(reader.poll(selection()).is_none());
}

#[test]
fn finite_float_extremes_and_invalid_latency_or_metadata_never_fabricate_healthy_evidence() {
    let (mut tap, mut worker, mut reader) =
        prepare(selection(), config(32, Window::Rectangular)).unwrap();
    let pcm = [[f32::MAX, f32::MIN_POSITIVE]; 32];
    assert_eq!(
        allocations(|| {
            tap.publish(stamp(0), &pcm).unwrap();
        })
        .0,
        [0; 4]
    );
    worker.pump();
    let s = reader.poll(selection()).unwrap();
    assert!(s.channels[0].levels.unwrap().rms.is_finite());
    assert!(
        s.spectrum
            .iter()
            .all(|b| b.mean_square.iter().all(|p| p.is_finite()))
    );
    assert_eq!(s.channels[0].clip_samples, 32);
    assert_eq!(
        tap.publish(
            Stamp {
                pdc_frames: 48_001,
                ..stamp(32)
            },
            &pcm
        ),
        Err(Refusal::InvalidClock)
    );
    assert!(reader.poll(selection()).is_none());
    let invalid_pcm = [[f32::NAN, 0.0]; 32];
    assert_eq!(
        allocations(|| {
            tap.publish(stamp(64), &invalid_pcm).unwrap();
        })
        .0,
        [0; 4]
    );
    worker.pump();
    assert_eq!(
        reader.poll(selection()).unwrap().spectrum_valid,
        [false, true]
    );
    assert_eq!(reader.counters().invalid_samples, 32);
}

#[test]
#[ignore = "scoped headless CPU measurement; invoke explicitly in release"]
fn release_cpu_throughput() {
    let (mut tap, mut worker, mut reader) = prepare(selection(), Config::default()).unwrap();
    let pcm = sine(256, 7.25, 0.25);
    let start = Instant::now();
    let mut callback_max = Duration::ZERO;
    for i in 0..18_750 {
        let callback_start = Instant::now();
        tap.publish(stamp(i * 256), &pcm).unwrap();
        callback_max = callback_max.max(callback_start.elapsed());
        worker.pump();
        reader.poll(selection());
    }
    let elapsed = start.elapsed();
    println!(
        "frames=4800000 rate=48000 fft=1024 hop=256 elapsed_ms={:.3} audio_seconds_per_cpu_second={:.2} max_copy_us={:.3} payload_bytes={} reserved_bytes={} drops={:?}",
        elapsed.as_secs_f64() * 1000.0,
        100.0 / elapsed.as_secs_f64(),
        callback_max.as_secs_f64() * 1e6,
        reader.layout().payload_bytes,
        reader.layout().reserved_bytes,
        reader.counters()
    );
    assert_eq!(reader.counters().dropped_packets, 0);
    assert_eq!(reader.counters().dropped_snapshots, 0);
    println!(
        "maximum_layout={:?}",
        Config {
            fft_size: MAX_FFT_SIZE,
            hop_size: MAX_FFT_SIZE / 4,
            input_packets: MAX_INPUT_PACKETS,
            snapshot_capacity: MAX_SNAPSHOTS,
            ..Config::default()
        }
        .layout()
        .unwrap()
    );
}
