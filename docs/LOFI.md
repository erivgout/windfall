# Lo-fi reduction DSP foundation

This source-only E4 leaf is based directly on `427ed0a7a07f3cae41ab1b91133b5e317b0f3dc9`. It advances `fx-fruity-squeeze` without closing that row or the full E4 family. `windfall_dsp::lofi::{Lofi, LofiParams}` implements the existing public `Effect` and `ParamSet` contracts. There is no registry entry, built-in alias, project integration or new dependency.

## Scope, provenance and license

The [Image-Line public manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Squeeze.htm), consulted 2026-10-08, describes bit reduction, alternating preserved/replaced sample runs, lowpass frequency/resonance, pre/post placement, mix and gain. This leaf implements an original resolution reducer and drive with post filtering; the timed replacement controls, resonance and placement choice remain gaps. No acoustic equivalence is claimed. The name above is a plain documentation reference only.

The [Cycling '74 signal reducer reference](https://docs.cycling74.com/reference/degrade~/), consulted on the same date, documents separate bit-depth and resampling controls and intentionally omits interpolation and dither for lo-fi effects. That is behavioral context for a distinct sample-rate reduction path here, not a copied transfer or UI.

All equations, processor code and fixtures in this leaf are original. Existing `clean`, gain conversion, frame-count conversion and finite-state flush helpers are reused under the repository's GPL-3.0-or-later license. The existing smoothing/filter blocks were examined; the private ramp avoids accumulated f32 slope error, and the private convex filter supports the stated full-scale headroom bound. No vendor source, binary, screenshot, preset, sound, artwork or UI was used. No assets or dependencies were added. The applicable license text is [GNU GPL v3](https://www.gnu.org/licenses/gpl-3.0.html) and the existing root `LICENSE`; no third-party implementation is imported by this leaf.

## Stable parameter contract

`LofiParams` is `Debug + Clone + Copy + PartialEq + Serialize + Deserialize + TS + Default + ParamSet`. JSON is camelCase, with struct-level serde defaults: `{}` gives the complete audible default below. Integer JSON must be a valid `u8`; out-of-type JSON is rejected. Typed values are sanitized by `set_params`; descriptors and `ParamSet::set` use the existing shared sanitization contract. Finite floats clamp to the declared range, NaN and either infinity become that control's default. `bits` clamps to 2–16; numeric descriptor writes round halfway away from zero. Serialization/TS derivation itself does not sanitize raw user-created structs. The leaf deliberately omits `#[ts(export)]` and does not generate checked-in bindings. A later integration owner can export the existing derived type.

Indices below are the future persistence/automation contract: append controls only. All are continuous floats except index 0, an integer. Fraction controls have linear descriptor scales; ratio and cutoff have logarithmic scales. Processor smoothing uses signal-space coordinates independently of descriptor scales.

| Index | Rust / JSON | Plain label | Unit and range | Default |
| --- | --- | --- | --- | --- |
| 0 | `bits` / `bits` | Resolution | integer bits, 2–16 | 8 |
| 1 | `quantize` / `quantize` | Quantization | fraction, 0–1 | 1 |
| 2 | `rate_ratio` / `rateRatio` | Rate reduction | host/capture rate, 1–64 | 4 |
| 3 | `drive_db` / `driveDb` | Drive | dB, 0–36 | 6 |
| 4 | `distortion` / `distortion` | Distortion | fraction, 0–1 | 0.25 |
| 5 | `cutoff_hz` / `cutoffHz` | Tone cutoff | Hz, 20–20000 | 8000 |
| 6 | `filter` / `filter` | Tone filter | fraction, 0–1 | 1 |
| 7 | `mix` / `mix` | Mix | fraction, 0–1 | 1 |
| 8 | `output_db` / `outputDb` | Output | dB, -24–12 | -3 |

`LofiParams::neutral()` sets quantization, distortion and filter amounts to zero, ratio to one, output to 0 dB and mix to one; bits and cutoff then have no audible effect. Neutral preserves finite floating-point input exactly, including signed zero and samples above ±1 up to `f32::MAX`. The dry endpoint (`mix=0`, `outputDb=0`) also preserves headroom regardless of active wet settings. Histories continue running at either mix endpoint; there is no extra effect bypass flag.

Prepare accepts finite sample rates clamped to 1–384000 Hz. NaN/Inf use 48000 Hz, following the direct `ParamSet` style fallback; zero, negative and tiny positive rates clamp to 1 Hz. A finite positive maximum block is not required: `max_block` is unused because the processor has no block-sized storage. Processing any nonempty block length is valid, including greater than the advisory maximum. The host supplies equal-length stereo slices and finite samples. Debug builds assert on invalid signal samples; release passes each offending NaN/Inf through visibly without writing it into that channel's history. It never silently turns an invalid signal into plausible finite audio. This input policy differs intentionally from parameter sanitization.

## Original transfer and rate clock

Each channel has its own capture phase, held sample and two filter histories. Controls advance once per stereo frame, before either channel processes it. Equal rate controls synchronize the two deterministic clocks; the audio histories and arithmetic are independent. Neither channel's impulse or noise enters the other channel, and odd transfer functions preserve antiphase.

Order: input → drive blend → quantization blend → sample-and-hold → post-filter blend → dry/wet mix → final output gain. All transfer arithmetic and channel histories use f64; signal-space ramp values and final audio are f32.

For drive gain `G=10^(driveDb/20)`, driven saturation is `S(x)=G*x/(1+abs(G*x))`. Distortion amount `d` gives `(1-d)*x+d*S(x)`. The saturator is odd, monotonic and strictly bounded by ±1 for finite input; its small-signal slope is G. There is no automatic loudness compensation. `d=0` preserves raw headroom. Partial distortion blends retain part of above-full-scale input.

For integer depth `b`, positive endpoint code `M=2^(b-1)-1` and step `Δ=1/M`, the quantizer is

```
Q_b(x) = round_away_from_zero(clamp(x, -1, 1) * M) / M
```

There are `2^b-1` symmetric midtread levels: exact zero and equally spaced signed endpoints ±1. One possible b-bit code is intentionally unused to keep symmetry. Two bits give {-1,0,+1}; a one-bit mode is deliberately excluded so silence remains zero. Half-step ties round away from zero; the input is saturated before rounding. Within full scale, absolute quantization error is at most `Δ/2`. No dither or random noise is added. Quantization amount `q` blends the pre-quantized signal and `Q_b`; full quantization legitimately clips input above ±1, while partial quantization retains its unquantized share of headroom.

Resolution changes interpolate adjacent integer quantizers rather than abruptly switching grids. The ramp coordinate is step size `s_b=1/(2^(b-1)-1)`. Four fixed comparisons locate neighboring depths l and h=l+1 with `s_l >= s >= s_h`; their output blend weight is `(s_l-s)/(s_l-s_h)`. This gives coarse grids, which have greater signal error, a larger share of the same 5 ms transition. At settled integer depth the transfer is exactly `Q_b`. During smoothing the interpolated result can occupy levels between either grid; it is not a claim of a new integer PCM format.

The persistent capture phase starts at one on prepare/reset. At a frame with phase ≥1, capture the current drive/quantizer output, subtract one, then add `1/rateRatio` for the next frame. Frames in between reuse the held value, with no interpolation. Roundoff within `1e-12` of a capture-cycle crossing is treated as the crossing. For constant ratio R, capture times are `ceil(k*R)` starting at frame zero; over N frames the capture count is `floor((N-1)/R)+1`. Fractional ratios alternate neighboring integer hold lengths. Rate edits retain phase, continuously changing its increment; phase never restarts at a block boundary. Settled unity captures every frame and reanchors its phase. Ratios never exceed 64, so a stored nonzero sample is replaced by silence within 64 frames after input stops under frozen controls.

## Filtering, smoothing and realtime bounds

The optional post-filter is two cascaded nonresonant real poles. Effective pole frequency `f=min(cutoffHz,0.45*sampleRate)` and `α=1-exp(-2π*f/sampleRate)` are computed only during prepare/reset/actual parameter writes. Each stage updates `y[n]=(1-α)*y[n-1]+α*u[n]`; the second consumes the first stage's current output. This is a monotonic lowpass with exact DC gain one. No pre-filter, resonance, cabinet or anti-alias stage is implied. The pole-frequency control is not an exact digital -3 dB corner of the entire cascade. For low normalized frequencies the cascade approaches -6 dB at f and 12 dB/octave asymptotic attenuation; near Nyquist its response follows the exact discrete equation:

```
ρ = 1 - α
h[n] = α² * (n+1) * ρ^n, n >= 0
|H(ω)| = α² / (1 + ρ² - 2ρ cos(ω))
```

Both histories are convex combinations of finite inputs, so each stays within the largest prior magnitude; cutoff automation remains stable and does not create filter overshoot. Filter amount blends its output with the held sample. Tiny filter state below `1e-20` is zeroed every frame, preserving finite checks rather than hiding NaN/Inf.

Each actual target change ramps in `N=max(1,round(0.005*sampleRate))` stereo frames. Gain ramps use linear gain, ratio uses host/capture ratio, filter uses α, amounts use their fractions and resolution uses quantization step size. The private frame ramp evaluates `start+(target-start)*elapsed/N` in f64 and emits f32, snapping exactly to the target at N; it does not repeatedly accumulate an f32 slope. Retargeting starts from the last emitted value. Identical sanitized writes and unrelated unchanged targets do not extend ramps. Empty blocks advance neither ramps nor clocks and do not consume fresh state. Prepare/reset clear both channels and snap current target settings; parameter writes before the first actual frame also snap. Tempo is ignored. Intrinsic quantizer and sample-hold steps remain intentional: 5 ms automation smoothing is not a claim that arbitrary lo-fi waveforms are continuously differentiable.

There is no oversampling or anti-alias promise. Drive harmonics above host Nyquist fold; quantizer nonlinear harmonics and capture-rate aliases are intentional. The post lowpass can reduce high-frequency output, but cannot undo aliases already folded into the passband. Full saturation and quantization bound the corresponding paths by ±1; partial effects, dry mixing and output trim need caller headroom. At ordinary finite input magnitude H, wet paths remain within `max(H,1)`, so final magnitude is at most `10^(12/20)*max(H,1)`; final f32 conversion can overflow for extreme near-`f32::MAX` input with positive output gain. This is exposed as Inf, not silently limited. Neutral unity does not overflow or clip.

All processor storage is fixed; prepare allocates zero bytes. Process/set/reset/tempo contain no allocation, reallocation, free, lock, wait, IO, unbounded loop or per-frame transcendental call. The adjacent-grid quantizer uses four bounded comparisons, shifts, rounding and arithmetic. Gain/exp coefficient calculations are a fixed amount of work on actual target writes, not per sample. This is a realtime-safe direct DSP implementation; device deadline and listening qualification remain separate.

Latency, warm-up and delay readiness report zero frames: there is no fixed delay needing PDC. Hold age is a time-varying 0–63-frame coloration; the lowpass has frequency-dependent phase/group delay and direct feedthrough, not a compensatable fixed latency. Tail and gap are deliberately conservative and frozen at prepare, independent of current parameters:

```
f_min = min(20, 0.45*sampleRate)
tail = gap = ceil(128*sampleRate/(2π*f_min)) + 64 + N
```

After a complete frozen-target ramp and maximal hold, two geometric poles decay with at most a polynomial `(n+1)` factor. 128 time constants cover even finite `f32::MAX` filter history below -90 dBFS, plus rounding margin. This is a conservative termination bound rather than an audible decay duration; it assumes no continuing signal or new control edits during tail termination. A later host may use actual quiet measurements but must preserve the bound.

## Usable source-side examples

The default gives eight-bit grids, quarter-rate capture, gentle blended drive, an 8 kHz post pole and -3 dB final trim. It is audible immediately, unlike neutral. Own deterministic test signals are generated from explicit fixed seeds; no external content is needed.

```rust
use windfall_dsp::{Effect, ParamSet};
use windfall_dsp::lofi::{Lofi, LofiParams};
let mut effect = Lofi::default();
effect.prepare(48_000.0, 512);
// A gentler drum-texture starting point, authored for Windfall.
let mut settings = LofiParams {
    bits: 10, quantize: 0.6, rate_ratio: 2.5,
    drive_db: 6.0, distortion: 0.2, cutoff_hz: 6_000.0,
    filter: 0.75, mix: 0.4, output_db: -3.0,
};
settings.set(LofiParams::index_of("bits").unwrap(), 7.0);
effect.set_params(&settings);
let (mut left, mut right) = ([0.0; 137], [0.0; 137]);
effect.process(&mut left, &mut right);
```

## Acceptance and reproduction

The owned auto-discovered `tests/lofi.rs` uses nearest-code distance decisions, analytic capture times/counts, a closed-form two-pole impulse and DTFT, native analytic drive references and a separately synthesized 16× high-rate transfer. References do not call production quantizer/filter/clock helpers. It checks all 2–16-bit codes and signed boundaries, integer/fractional clocks over irregular partitions, independent stereo impulses/noise/antiphase, representative 44.1/48/96/192 kHz plus 384 kHz/invalid/tiny rates, filter DC/impulses/spectra/stability, intentional alias energy, headroom and mix endpoints, fast retargets/noops/empty/reset/silence, exact bit parity at blocks 1/7/64/137/512, frozen tails and a four-entry allocator guard. The guard's positive control proves alloc/zeroed-alloc/realloc/free observability. Conditional invalid-signal tests exercise the debug assertion and visible release propagation respectively; neither signal case is an optional ignore.

Run in Git Bash from this bound worktree, with jobs/tests restricted to one:

```bash
source scripts/msvc-env.sh
export CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target/e4-native
export TS_RS_EXPORT_DIR=C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-lofi-e4/target/e4-native/ts-bindings
cargo test -p windfall-dsp --test lofi -- --test-threads=1
cargo clippy -p windfall-dsp --lib --test lofi -- -D warnings
cargo test --release -p windfall-dsp --test lofi -- --test-threads=1
cargo test --release -p windfall-dsp --test lofi scoped_cpu_throughput -- --ignored --exact --nocapture --test-threads=1
rustfmt --edition 2024 --config skip_children=true --check crates/windfall-dsp/src/lofi.rs crates/windfall-dsp/tests/lofi.rs crates/windfall-dsp/src/lib.rs
git diff --check
```

`target/e4-native` and its task-local TS directory are excluded by the existing root target ignore. The only intentional optional ignore is `scoped_cpu_throughput`, explicitly invoked above. No broad workspace build, generator, installation, checked-in artifact, PR, release or push belongs to this leaf.

First-run failures and repairs, with all authored numerical thresholds retained:

- Test compilation: `E0061`, `LofiParams::decl()` lacked the ts-rs 12 `&Config` argument; changed to `decl(&Config::default())`. `E0308`, a consumed output iterator yielded f32 while the test pattern expected `&f32`; corrected the pattern.
- `callback_allocation_zero_for_all_extremes_and_fast_automation`: the processor guard returned `[0,0,0,0]`, but the guard's positive-control assertion expected `[1,1,1,2]` and got `[0,1,0,1]`. Optimization removed the unused alloc/realloc/free group. Opaque `black_box` pointer/layout use made all entry points observable; the exact positive-control assertion passes. A separate zero-call construction/maximum-prepare guard was then added.
- `smoothing_noops_fast_retargets_empty_reset_and_step_bounds`: at frame 238, `abs(current-previous) <= 0.02` failed for `0.9392756 → 0.95935667`. A linear bit-number ramp spent too little time near the coarse grid. Changed the internal resolution coordinate to quantization step size with bounded adjacent-grid interpolation, retaining the same 240-frame arrival and 0.02 assertion. The unchanged assertion passes.
- Strict Clippy: `manual_range_contains` on the impulse output bound `y >= 0 && y <= 1`; used `(0..=1).contains(&y)` with identical bounds. Strict scoped Clippy then passed.
- Expanded acceptance: all 225 resolution-depth pairs at seven constant amplitudes passed the unchanged 0.02 step bound. `drive_arrives_in_five_ms_at_representative_rates_with_analytic_ramp_reference` then failed `abs(actual-reference) < 3e-6` at 96000 Hz/frame 436: actual `0.9070949`, analytic `0.9070918881618402`. Replaced only the private lo-fi ramp with fixed-start/integer-frame interpolation in f64; the same 3e-6 assertion passes at all four representative rates. No shared smoothing block was changed.

Final verification on Windows x64, 2026-10-08: scoped debug and release suites each passed **15 tests**, with zero failures and only the explicitly optional CPU test ignored. That CPU test was separately invoked and passed all eight cases. Strict scoped Clippy with `-D warnings`, owned-file rustfmt check and `git diff --check` passed. Construction/maximum prepare and automated process/set/reset/tempo guards each reported **[0,0,0,0]** alloc/zeroed/realloc/free calls; the allocator positive control reported **[1,1,1,2]**. No signal test is hidden behind an optional ignore. Debug invalid input asserts; the corresponding release test passes visible NaN/Inf through.

The final processor occupies **304 bytes** on this x64 target, including its stored **36-byte** parameter struct, nine ramps and both channels' histories. Maximum prepare storage remains 304 bytes, with **zero heap bytes**, at every supported sample rate and advisory block size (including `usize::MAX`). The conservative frozen tail/gap is 45205/49197/98329/196594 frames at 44.1/48/96/192 kHz; the formula above also applies at the supported rate limits.

The final release headless run measured 30000 blocks × 128 stereo frames per case with seeded input, buffer copies, `black_box` observability, and parameter writes included in automated timing. Hardware reported `Intel64 Family 6 Model 183 Stepping 1, GenuineIntel`, 32 logical processors; build jobs and test threads were one. These are one-run wall-clock throughput figures, without CPU pinning, hardware audio, a deadline guarantee or a listening claim.

| Host rate | Default, M stereo frames/s | Default, µs/128 frames | Automated, M stereo frames/s | Automated, µs/128 frames |
| --- | --- | --- | --- | --- |
| 44100 | 28.617 | 4.473 | 60.607 | 2.112 |
| 48000 | 28.464 | 4.497 | 57.554 | 2.224 |
| 96000 | 28.494 | 4.492 | 38.905 | 3.290 |
| 192000 | 29.008 | 4.413 | 25.038 | 5.112 |

The automated case cycles descriptor indices and alternating minimum/maximum writes, changing one control per block. It sometimes bypasses quantizer work at zero amount, so a faster automated result does not imply that automation is free or that this is a worst-case CPU ceiling. At 192 kHz the measured default/automated walltime fractions relative to generated audio duration were 0.662%/0.767%. Maximum memory is fixed; unqualified maximum realtime CPU safety is not claimed.

## Remaining integration gates

Parent-controlled integration must adopt the actual processor in `AnyEffect`/`EffectParams`/`EffectSlot`, preserving append-only indices and legacy project defaults. Persistence/migration, checked commands and history, all-control automation in native/browser UI, TS/descriptors/WASM regeneration, engine plan preparation and identity preservation, realtime/offline/render/stem parity and useful native UI remain open. The E4 drawn waveshaper, modular distortion rack and actual guitar/pedal/cabinet behaviors remain separately owned future work. The manual's timed fixed-value replacement/resonance/pre-post features require their own reviewed design and evidence before any row closure. This leaf edits neither roadmap nor parity accounting and makes no phase-completion or listening/hardware/platform claim.
