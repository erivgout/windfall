//! Authored deterministic audio only; frequency truth is oscillator phase math.
use std::cell::Cell;
use std::f64::consts::TAU;
use std::time::{Duration, Instant};
use windfall_analysis::pitch::*;
use windfall_analysis::{AnalysisError, AudioShape, CancelToken, FrameRange, Work};

struct TrackingAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    // Net live requested bytes, peak, allocation count. Tracking is scoped to
    // prepare+analyze on this test thread; output/analyzer dropped inside scope.
    static ALLOC: Cell<(isize, usize, usize)> = const { Cell::new((0, 0, 0)) };
}
fn allocation_event(bytes: isize, alloc: bool) {
    let _ = TRACK.try_with(|track| {
        if track.get() {
            ALLOC.with(|state| {
                let (live, peak, count) = state.get();
                let next = live + bytes;
                state.set((
                    next,
                    peak.max(next.max(0) as usize),
                    count + usize::from(alloc),
                ));
            });
        }
    });
}
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;
unsafe impl std::alloc::GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(layout) };
        if !p.is_null() {
            allocation_event(layout.size() as isize, true);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) {
        allocation_event(-(layout.size() as isize), false);
        unsafe {
            std::alloc::System.dealloc(p, layout);
        }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let next = unsafe { std::alloc::System.realloc(p, layout, size) };
        if !next.is_null() {
            allocation_event(size as isize - layout.size() as isize, true);
        }
        next
    }
}
fn work() -> Work {
    Work::new(
        CancelToken::default(),
        10_000_000_000_000_000,
        Duration::from_secs(300),
    )
    .unwrap()
}
fn config(rate: u32) -> PitchConfig {
    PitchConfig {
        sample_rate: rate,
        max_hz: 1000.0_f64.min(f64::from(rate) / 8.0),
        hop_frames: rate / 100,
        ..PitchConfig::default()
    }
}
fn source(pcm: &[f32], rate: u32, channels: u16) -> PitchSource<'_> {
    PitchSource {
        pcm,
        shape: AudioShape {
            frames: (pcm.len() / usize::from(channels)) as u64,
            channels,
            sample_rate: rate,
        },
        range: FrameRange {
            start: 0,
            end: (pcm.len() / usize::from(channels)) as u64,
        },
        frame_origin: 0,
    }
}
fn synth(rate: u32, seconds: f64, phase: impl Fn(f64) -> f64) -> Vec<f32> {
    (0..(f64::from(rate) * seconds) as usize)
        .map(|j| (0.6 * phase(j as f64 / f64::from(rate)).sin()) as f32)
        .collect()
}
fn tone(rate: u32, seconds: f64, hz: f64) -> Vec<f32> {
    synth(rate, seconds, |t| TAU * hz * t + 0.31)
}
fn analyze(pcm: &[f32], cfg: PitchConfig, channels: u16) -> PitchAnalysis {
    PitchAnalyzer::prepare(cfg, PitchLimits::default(), &mut work())
        .unwrap()
        .analyze(source(pcm, cfg.sample_rate, channels), &mut work())
        .unwrap()
}
fn cents(measured: f64, expected: f64) -> f64 {
    (1200.0 * (measured / expected).log2()).abs()
}
fn percentile(values: &mut [f64], fraction: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    values[((values.len() - 1) as f64 * fraction).ceil() as usize]
}
fn assert_complete(result: &PitchAnalysis) {
    for ranges in [
        result
            .estimates()
            .iter()
            .map(|e| e.range)
            .collect::<Vec<_>>(),
        result.segments().iter().map(|s| s.range).collect(),
    ] {
        if result.range().start == result.range().end {
            assert!(ranges.is_empty());
            continue;
        }
        assert_eq!(ranges.first().unwrap().start, result.range().start);
        assert_eq!(ranges.last().unwrap().end, result.range().end);
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
        assert!(ranges.iter().all(|r| r.start < r.end));
    }
    for e in result.estimates() {
        assert_eq!(
            e.center_frame,
            e.range.start + (e.range.end - e.range.start) / 2
        );
        assert!(e.support.start >= result.range().start && e.support.end <= result.range().end);
        assert!((0.0..=1.0).contains(&e.confidence) && e.rms.is_finite());
        assert_eq!(e.f0_hz.is_some(), e.confidence > 0.0);
    }
}

#[test]
fn calibrated_notes_all_rates_and_harmonic_octave_traps() {
    let mut errors = Vec::new();
    let mut total = 0;
    let mut voiced = 0;
    let mut octave = 0;
    for rate in [8000, 22050, 44100, 48000, 96000, 192000] {
        for hz in [55.0, 82.406_889, 110.0, 220.0, 440.0, 880.0] {
            let pcm = tone(rate, 0.3, hz);
            let result = analyze(&pcm, config(rate), 1);
            assert_complete(&result);
            for e in result.estimates() {
                total += 1;
                if let Some(f0) = e.f0_hz {
                    voiced += 1;
                    let err = cents(f0, hz);
                    octave += usize::from(err >= 600.0);
                    errors.push(err);
                }
            }
        }
    }
    // Even harmonics alone truly have a doubled F0. These fixtures include odd
    // partials, so 110/220 Hz are manually known periods despite spectral peaks.
    for hz in [110.0, 220.0] {
        for missing in [false, true] {
            let pcm: Vec<_> = (0..24000)
                .map(|j| {
                    let p = TAU * hz * j as f64 / 48000.0;
                    ((if missing { 0.0 } else { 0.2 * p.sin() })
                        + 0.6 * (2.0 * p).sin()
                        + 0.35 * (3.0 * p + 0.3).sin()
                        + 0.2 * (4.0 * p).sin()) as f32
                })
                .collect();
            for e in analyze(&pcm, config(48000), 1).estimates() {
                total += 1;
                if let Some(f0) = e.f0_hz {
                    voiced += 1;
                    let err = cents(f0, hz);
                    octave += usize::from(err >= 600.0);
                    errors.push(err);
                }
            }
        }
    }
    let p95 = percentile(&mut errors, 0.95);
    let max = *errors.last().unwrap();
    println!(
        "stationary/harmonics: total={total}, voiced={voiced}, p95={p95:.6} cents, max={max:.6}, octave={octave}"
    );
    assert!(voiced as f64 / total as f64 >= 0.99);
    assert!(p95 <= 5.0 && max <= 10.0);
    assert_eq!(octave, 0);
}

#[test]
fn glide_and_vibrato_phase_derivative_reference() {
    for vibrato in [false, true] {
        // Vibrato frequency = 220 + depth*sin(2*pi*5*t), exactly ±35 cents
        // approximately; independent analytic phase integral avoids drift.
        let depth = 220.0 * (2.0_f64.powf(35.0 / 1200.0) - 1.0);
        let pcm = synth(48000, 2.0, |t| {
            if vibrato {
                TAU * 220.0 * t + depth / 5.0 * (1.0 - (TAU * 5.0 * t).cos())
            } else {
                TAU * (180.0 * t + 45.0 * t * t)
            }
        });
        let result = analyze(&pcm, config(48000), 1);
        assert_complete(&result);
        let mut errors = Vec::new();
        let mut total = 0;
        for e in result.estimates() {
            let t = e.center_frame as f64 / 48000.0;
            if !(0.08..1.92).contains(&t) {
                continue;
            }
            total += 1;
            if let Some(f0) = e.f0_hz {
                let expected = if vibrato {
                    220.0 + depth * (TAU * 5.0 * t).sin()
                } else {
                    180.0 + 90.0 * t
                };
                errors.push(cents(f0, expected));
            }
        }
        let recall = errors.len() as f64 / total as f64;
        let p95 = percentile(&mut errors, 0.95);
        println!(
            "dynamic vibrato={vibrato}: cells={total}, recall={recall:.6}, p95={p95:.6}, max={:.6}",
            errors.last().unwrap()
        );
        assert!(recall >= 0.95 && p95 <= 25.0);
        assert!(errors.iter().all(|e| *e < 600.0));
    }
}

fn noise(frames: usize) -> Vec<f32> {
    let mut state = 0x7865_1234_u32;
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            ((f64::from(state) / f64::from(u32::MAX) * 2.0 - 1.0) * 0.4) as f32
        })
        .collect()
}
#[test]
fn silence_dc_broadband_noise_and_transients_do_not_fake_pitch() {
    let mut impulses = vec![0.0; 96000];
    for i in [1000, 24000, 55000, 80000] {
        impulses[i] = 1.0;
    }
    let mut total = 0;
    let mut false_voiced = 0;
    for pcm in [vec![0.0; 96000], vec![0.9; 96000], noise(96000), impulses] {
        let result = analyze(&pcm, config(48000), 1);
        assert_complete(&result);
        total += result.estimates().len();
        false_voiced += result
            .estimates()
            .iter()
            .filter(|e| e.f0_hz.is_some())
            .count();
    }
    println!("unvoiced: cells={total}, false_voiced={false_voiced}");
    assert!(false_voiced as f64 / total as f64 <= 0.01);
}

#[test]
fn stereo_policy_survives_antiphase_and_selects_explicit_or_stronger_channel() {
    let mono = tone(48000, 0.4, 220.0);
    let anti: Vec<_> = mono.iter().flat_map(|x| [*x, -*x]).collect();
    let result = analyze(&anti, config(48000), 2);
    assert!(
        result
            .estimates()
            .iter()
            .all(|e| e.channel == 0 && e.f0_hz.is_some_and(|f| cents(f, 220.0) < 5.0))
    );
    let other = tone(48000, 0.4, 330.0);
    let stereo: Vec<_> = mono
        .iter()
        .zip(other)
        .flat_map(|(a, b)| [*a * 0.2, b])
        .collect();
    for (policy, expected, channel) in [
        (ChannelPolicy::Strongest, 330.0, 1),
        (ChannelPolicy::Channel(0), 220.0, 0),
    ] {
        let result = analyze(
            &stereo,
            PitchConfig {
                channel_policy: policy,
                ..config(48000)
            },
            2,
        );
        assert!(
            result
                .estimates()
                .iter()
                .all(|e| e.channel == channel && e.f0_hz.is_some_and(|f| cents(f, expected) < 5.0))
        );
    }
}

#[test]
fn segmentation_has_exact_bounds_and_musically_useful_boundary_error() {
    let mut pcm = vec![0.0; 24000];
    pcm.extend(tone(48000, 0.5, 220.0));
    pcm.extend(tone(48000, 0.5, 330.0));
    pcm.extend(vec![0.0; 24000]);
    let result = analyze(&pcm, config(48000), 1);
    assert_complete(&result);
    for (boundary, left, right) in [
        (24000, None, Some(220.0)),
        (48000, Some(220.0), Some(330.0)),
        (72000, Some(330.0), None),
    ] {
        // Windows crossing a step may correctly be unvoiced or intermediate.
        // Measure the bracket of recognized states against the authored step.
        let close = |f: Option<f64>, target: Option<f64>| match (f, target) {
            (None, None) => true,
            (Some(a), Some(b)) => cents(a, b) < 25.0,
            _ => false,
        };
        let last_left = result
            .estimates()
            .iter()
            .rfind(|e| e.center_frame < boundary && close(e.f0_hz, left))
            .unwrap();
        let first_right = result
            .estimates()
            .iter()
            .find(|e| e.center_frame >= boundary && close(e.f0_hz, right))
            .unwrap();
        let error = last_left
            .range
            .end
            .abs_diff(boundary)
            .max(first_right.range.start.abs_diff(boundary));
        println!(
            "boundary {boundary}: error_frames={error}, ms={:.3}",
            error as f64 / 48.0
        );
        assert!(error <= 1920);
    }
    assert!(result.segments().len() >= 4);
}

#[test]
fn clipped_pcm_and_full_finite_f32_domain_are_not_sanitized() {
    let clipped: Vec<_> = tone(48000, 0.3, 220.0)
        .iter()
        .map(|x| (*x * 4.0).clamp(-1.0, 1.0))
        .collect();
    assert!(
        analyze(&clipped, config(48000), 1)
            .estimates()
            .iter()
            .all(|e| e.f0_hz.is_some_and(|f| cents(f, 220.0) < 5.0))
    );
    for value in [f32::MAX, f32::MIN_POSITIVE, f32::from_bits(1)] {
        assert!(
            analyze(&vec![value; 4800], config(48000), 1)
                .estimates()
                .iter()
                .all(|e| e.f0_hz.is_none())
        );
    }
    let huge: Vec<_> = tone(48000, 0.2, 220.0)
        .iter()
        .map(|x| *x * f32::MAX)
        .collect();
    assert!(
        analyze(&huge, config(48000), 1)
            .estimates()
            .iter()
            .all(|e| e.f0_hz.is_some_and(|f| cents(f, 220.0) < 5.0))
    );
}

#[test]
fn raw_nonfinite_rejected_even_outside_selection_or_on_unused_channel() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut pcm: Vec<_> = tone(48000, 0.3, 220.0)
            .iter()
            .flat_map(|x| [*x, 0.0])
            .collect();
        pcm[1] = bad;
        let mut a = PitchAnalyzer::prepare(
            PitchConfig {
                channel_policy: ChannelPolicy::Channel(0),
                ..config(48000)
            },
            PitchLimits::default(),
            &mut work(),
        )
        .unwrap();
        let mut s = source(&pcm, 48000, 2);
        s.range.start = 4800;
        assert!(matches!(
            a.analyze(s, &mut work()),
            Err(PitchError::NonfinitePcm { sample: 1 })
        ));
    }
}

#[test]
fn range_origin_empty_short_reset_and_partition_contract() {
    let pcm = tone(48000, 1.0, 220.0);
    let mut a = PitchAnalyzer::prepare(config(48000), PitchLimits::default(), &mut work()).unwrap();
    let mut s = source(&pcm, 48000, 1);
    s.range = FrameRange {
        start: 4800,
        end: 43111,
    };
    s.frame_origin = (1_u64 << 54) + 123;
    let result = a.analyze(s, &mut work()).unwrap();
    assert_complete(&result);
    assert_eq!(
        result.range(),
        FrameRange {
            start: s.frame_origin + 4800,
            end: s.frame_origin + 43111
        }
    );
    let repeat = a.analyze(s, &mut work()).unwrap();
    assert_eq!(result.estimates(), repeat.estimates());
    let quiet = vec![0.0; 4800];
    a.analyze(source(&quiet, 48000, 1), &mut work()).unwrap();
    assert_eq!(
        result.estimates(),
        a.analyze(s, &mut work()).unwrap().estimates()
    );
    s.range = FrameRange {
        start: 999,
        end: 999,
    };
    assert!(a.analyze(s, &mut work()).unwrap().estimates().is_empty());
    s.range.end += 1;
    assert!(matches!(
        a.analyze(s, &mut work()),
        Err(PitchError::TooShort { .. })
    ));
    let all = a.analyze(source(&pcm, 48000, 1), &mut work()).unwrap();
    let mut part = source(&pcm, 48000, 1);
    part.range = FrameRange {
        start: 24000,
        end: 48000,
    };
    let half = a.analyze(part, &mut work()).unwrap();
    for e in half
        .estimates()
        .iter()
        .filter(|e| e.support.start > 24000 && e.support.end < 48000)
    {
        assert_eq!(
            *e,
            all.estimates()
                .iter()
                .find(|p| p.center_frame == e.center_frame)
                .copied()
                .unwrap()
        );
    }
}

#[test]
fn rejects_invalid_config_shapes_arithmetic_and_resource_capacities() {
    for cfg in [
        PitchConfig {
            sample_rate: 0,
            ..config(48000)
        },
        PitchConfig {
            sample_rate: 192001,
            ..config(48000)
        },
        PitchConfig {
            min_hz: f64::NAN,
            ..config(48000)
        },
        PitchConfig {
            min_hz: 39.9,
            ..config(48000)
        },
        PitchConfig {
            max_hz: f64::INFINITY,
            ..config(48000)
        },
        PitchConfig {
            min_hz: 1000.0,
            max_hz: 1000.0,
            ..config(48000)
        },
        PitchConfig {
            hop_frames: 0,
            ..config(48000)
        },
        PitchConfig {
            hop_frames: u32::MAX,
            ..config(48000)
        },
        PitchConfig {
            yin_threshold: 0.0,
            ..config(48000)
        },
        PitchConfig {
            rms_floor: f64::NAN,
            ..config(48000)
        },
        PitchConfig {
            segment_jump_cents: 0.0,
            ..config(48000)
        },
        PitchConfig {
            channel_policy: ChannelPolicy::Channel(8),
            ..config(48000)
        },
    ] {
        assert!(matches!(
            PitchAnalyzer::prepare(cfg, PitchLimits::default(), &mut work()),
            Err(PitchError::Invalid(_))
        ));
    }
    for lim in [
        PitchLimits {
            estimates: 0,
            ..PitchLimits::default()
        },
        PitchLimits {
            source_bytes: u64::MAX,
            ..PitchLimits::default()
        },
        PitchLimits {
            analyzer_bytes: 1,
            ..PitchLimits::default()
        },
    ] {
        assert!(PitchAnalyzer::prepare(config(48000), lim, &mut work()).is_err());
    }
    let pcm = tone(48000, 0.2, 220.0);
    let mut a = PitchAnalyzer::prepare(config(48000), PitchLimits::default(), &mut work()).unwrap();
    for s in [
        PitchSource {
            frame_origin: u64::MAX,
            ..source(&pcm, 48000, 1)
        },
        PitchSource {
            shape: AudioShape {
                frames: u64::MAX,
                channels: 8,
                sample_rate: 48000,
            },
            ..source(&pcm, 48000, 1)
        },
        PitchSource {
            shape: AudioShape {
                frames: 9601,
                channels: 1,
                sample_rate: 48000,
            },
            ..source(&pcm, 48000, 1)
        },
        PitchSource {
            range: FrameRange {
                start: 100,
                end: 99,
            },
            ..source(&pcm, 48000, 1)
        },
        PitchSource {
            range: FrameRange {
                start: 0,
                end: 9601,
            },
            ..source(&pcm, 48000, 1)
        },
        source(&pcm, 44100, 1),
    ] {
        assert!(a.analyze(s, &mut work()).is_err());
    }
    for lim in [
        PitchLimits {
            output_bytes: 1,
            ..PitchLimits::default()
        },
        PitchLimits {
            source_bytes: 1,
            ..PitchLimits::default()
        },
        PitchLimits {
            selected_frames: 1,
            ..PitchLimits::default()
        },
        PitchLimits {
            estimates: 1,
            ..PitchLimits::default()
        },
    ] {
        let mut a = PitchAnalyzer::prepare(config(48000), lim, &mut work()).unwrap();
        assert!(matches!(
            a.analyze(source(&pcm, 48000, 1), &mut work()),
            Err(PitchError::Limit(_))
        ));
    }
    let mut a = PitchAnalyzer::prepare(
        PitchConfig {
            channel_policy: ChannelPolicy::Channel(1),
            ..config(48000)
        },
        PitchLimits::default(),
        &mut work(),
    )
    .unwrap();
    assert!(matches!(
        a.analyze(source(&pcm, 48000, 1), &mut work()),
        Err(PitchError::Invalid(_))
    ));
}

#[test]
fn cancellation_deadline_work_and_segment_refusal_return_no_partial_success() {
    let cfg = config(48000);
    let lim = PitchLimits::default();
    let token = CancelToken::default();
    token.cancel();
    let mut cancelled = Work::new(token, 1_000_000_000, Duration::from_secs(60)).unwrap();
    assert!(matches!(
        PitchAnalyzer::prepare(cfg, lim, &mut cancelled),
        Err(PitchError::Work(AnalysisError::Cancelled))
    ));
    let pcm = tone(48000, 0.4, 220.0);
    let mut a = PitchAnalyzer::prepare(cfg, lim, &mut work()).unwrap();
    assert!(matches!(
        a.analyze(source(&pcm, 48000, 1), &mut cancelled),
        Err(PitchError::Work(AnalysisError::Cancelled))
    ));
    let mut expired = Work::new(
        CancelToken::default(),
        1_000_000_000,
        Duration::from_nanos(1),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(1));
    assert!(matches!(
        a.analyze(source(&pcm, 48000, 1), &mut expired),
        Err(PitchError::Work(AnalysisError::Deadline))
    ));
    let req = a.requirements(source(&pcm, 48000, 1)).unwrap();
    let mut under = Work::new(
        CancelToken::default(),
        req.work_units - 1,
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(matches!(
        a.analyze(source(&pcm, 48000, 1), &mut under),
        Err(PitchError::Work(AnalysisError::Budget(_)))
    ));
    assert!(under.completed() > pcm.len() as u64); // failure after some estimates
    let mut exact = Work::new(
        CancelToken::default(),
        req.work_units,
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(a.analyze(source(&pcm, 48000, 1), &mut exact).is_ok());
    assert_eq!(exact.completed(), req.work_units);
    let mut changed = pcm.clone();
    changed.extend(tone(48000, 0.4, 330.0));
    let mut restricted =
        PitchAnalyzer::prepare(cfg, PitchLimits { segments: 1, ..lim }, &mut work()).unwrap();
    assert!(matches!(
        restricted.analyze(source(&changed, 48000, 1), &mut work()),
        Err(PitchError::Limit("segment count"))
    ));
    assert_eq!(
        restricted
            .analyze(source(&pcm, 48000, 1), &mut work())
            .unwrap()
            .segments()
            .len(),
        1
    );
}

#[test]
fn long_clip_heap_output_work_and_cpu_measurement() {
    let pcm = tone(48000, 30.0, 220.0); // caller input excluded from analyzer heap
    let cfg = config(48000);
    let mut prepare_work = work();
    let mut analyze_work = work();
    ALLOC.with(|s| s.set((0, 0, 0)));
    TRACK.with(|s| s.set(true));
    let started = Instant::now();
    let mut a = PitchAnalyzer::prepare(cfg, PitchLimits::default(), &mut prepare_work).unwrap();
    let preparation = started.elapsed();
    let resources = a.resources();
    let req = a.requirements(source(&pcm, 48000, 1)).unwrap();
    let started = Instant::now();
    let result = a
        .analyze(source(&pcm, 48000, 1), &mut analyze_work)
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(result.estimates().len(), 3000);
    assert!(
        result
            .estimates()
            .iter()
            .all(|e| e.f0_hz.is_some_and(|f| cents(f, 220.0) < 5.0))
    );
    drop(result);
    drop(a);
    TRACK.with(|s| s.set(false));
    let (live, peak, allocs) = ALLOC.with(Cell::get);
    assert_eq!(live, 0);
    assert!(peak as u64 <= resources.analyzer_peak_bytes + req.output_peak_bytes);
    assert_eq!(analyze_work.completed(), req.work_units);
    assert!(
        allocs < 30,
        "allocation count must not grow per hop: {allocs}"
    );
    println!(
        "resources: {resources:?}; output={req:?}; heap_peak={peak}, allocation_calls={allocs}, prepare_us={}, analyze_ms={}, audio_seconds=30, speed={:.3}x",
        preparation.as_micros(),
        elapsed.as_millis(),
        30.0 / elapsed.as_secs_f64()
    );
}

#[test]
fn largest_window_frequency_limits_exact_support_and_eight_channels() {
    let cfg = PitchConfig {
        sample_rate: 192000,
        min_hz: 40.0,
        max_hz: 2000.0,
        hop_frames: 1920,
        ..PitchConfig::default()
    };
    let mut a = PitchAnalyzer::prepare(cfg, PitchLimits::default(), &mut work()).unwrap();
    assert_eq!(a.resources().max_lag, 4800);
    assert_eq!(a.resources().fft_len, 32768);
    assert!(a.resources().analyzer_peak_bytes < 4 * 1024 * 1024);
    for hz in [40.1, 1990.0] {
        let pcm = tone(192000, 0.15, hz);
        let result = a.analyze(source(&pcm, 192000, 1), &mut work()).unwrap();
        assert!(
            result
                .estimates()
                .iter()
                .all(|e| e.f0_hz.is_some_and(|f| cents(f, hz) < 5.0))
        );
    }
    let pcm = tone(
        192000,
        a.resources().support_frames as f64 / 192000.0,
        440.0,
    );
    let result = a.analyze(source(&pcm, 192000, 1), &mut work()).unwrap();
    assert_complete(&result);
    assert!(
        result
            .estimates()
            .iter()
            .all(|e| e.support.start == 0 && e.support.end == pcm.len() as u64)
    );
    let pcm = tone(48000, 0.2, 220.0);
    let eight: Vec<_> = pcm
        .iter()
        .flat_map(|x| [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, *x])
        .collect();
    assert!(
        analyze(&eight, config(48000), 8)
            .estimates()
            .iter()
            .all(|e| e.channel == 7 && e.f0_hz.is_some())
    );
    for hz in [45.0, 1100.0] {
        let pcm = tone(48000, 0.2, hz);
        let result = analyze(&pcm, config(48000), 1);
        assert!(
            result.estimates().iter().all(|e| e.f0_hz.is_none()),
            "out-of-range tone {hz} must not become a subharmonic"
        );
    }
}

#[test]
fn cancellation_and_deadline_during_long_analysis_leave_reusable_analyzer() {
    let pcm = tone(48000, 30.0, 220.0);
    let mut a = PitchAnalyzer::prepare(config(48000), PitchLimits::default(), &mut work()).unwrap();
    let token = CancelToken::default();
    let child_token = token.clone();
    let cancel_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(15));
        child_token.cancel();
    });
    let mut interrupted = Work::new(token, 1_000_000_000_000, Duration::from_secs(60)).unwrap();
    let result = a.analyze(source(&pcm, 48000, 1), &mut interrupted);
    cancel_thread.join().unwrap();
    assert!(matches!(
        result,
        Err(PitchError::Work(AnalysisError::Cancelled))
    ));
    assert!(interrupted.completed() > pcm.len() as u64);
    let mut deadline = Work::new(
        CancelToken::default(),
        1_000_000_000_000,
        Duration::from_millis(15),
    )
    .unwrap();
    assert!(matches!(
        a.analyze(source(&pcm, 48000, 1), &mut deadline),
        Err(PitchError::Work(AnalysisError::Deadline))
    ));
    let small = tone(48000, 0.1, 330.0);
    assert!(
        a.analyze(source(&small, 48000, 1), &mut work())
            .unwrap()
            .estimates()
            .iter()
            .all(|e| e.f0_hz.is_some_and(|f| cents(f, 330.0) < 5.0))
    );
}

#[test]
#[ignore = "explicit 30-minute native resource/CPU measurement (~345.6 MB borrowed input)"]
fn thirty_minute_valid_clip_measurement() {
    // Repeating a one-second authored 220 Hz cycle keeps generation inexpensive
    // and is phase-continuous because it contains an integer number of periods.
    let cycle = tone(48000, 1.0, 220.0);
    let pcm: Vec<_> = cycle
        .iter()
        .copied()
        .cycle()
        .take(48000 * 60 * 30)
        .collect();
    let cfg = config(48000);
    let mut pw = work();
    let mut aw = work();
    ALLOC.with(|s| s.set((0, 0, 0)));
    TRACK.with(|s| s.set(true));
    let mut a = PitchAnalyzer::prepare(cfg, PitchLimits::default(), &mut pw).unwrap();
    let r = a.resources();
    let req = a.requirements(source(&pcm, 48000, 1)).unwrap();
    let start = Instant::now();
    let result = a.analyze(source(&pcm, 48000, 1), &mut aw).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(result.estimates().len(), 180000);
    assert_eq!(result.segments().len(), 1);
    assert!(
        result
            .estimates()
            .iter()
            .all(|e| e.f0_hz.is_some_and(|f| cents(f, 220.0) < 5.0))
    );
    drop(result);
    drop(a);
    TRACK.with(|s| s.set(false));
    let (live, peak, allocs) = ALLOC.with(Cell::get);
    assert_eq!(live, 0);
    assert!(peak as u64 <= r.analyzer_peak_bytes + req.output_peak_bytes);
    assert!(allocs < 30);
    assert_eq!(aw.completed(), req.work_units);
    println!(
        "30-minute: source_bytes={}, output={req:?}, analyzer={r:?}, heap_peak={peak}, allocation_calls={allocs}, elapsed_ms={}, speed={:.3}x",
        pcm.len() * 4,
        elapsed.as_millis(),
        1800.0 / elapsed.as_secs_f64()
    );
}
