# Multiband dynamics integration seam

Implemented exclusively in `crates/windfall-dsp/src/multiband/`. All processors
implement `crate::effect::Effect` and are public through `windfall_dsp::multiband`.
Each uses `Default` for construction and a distinct, public `*Params` type.
Parameters are Copy, defaulted camelCase serde data, derive `ts_rs::TS`, and
implement the descriptor/get/set/sanitized/approach interface through `param_set!`.
Existing effect unions, project commands, registry, UI and parity statuses are
intentionally integration-owner work; no other source files were edited.

## Types and suggested registry variants

- `BandSplit` / `BandSplitParams`: suggested `EffectKind::BandSplit`, displayed
  **Band Split**. Maps to `fx-frequency-splitter`. Two or three bands, editable
  crossover frequencies and per-band gains. `process()` recombines the outputs.
  `split_frame([left, right]) -> [[f32; 2]; 3]` permits independent host routing;
  canonical order is low/mid/high, and in two-band mode mid is zero and high
  contains the entire upper band. Returned frames include smoothed band gains.
- `MultibandCompressor` / `MultibandCompressorParams`: suggested
  `EffectKind::MultibandCompressor`, displayed **Multiband Compressor**. Maps to
  `fx-fruity-multiband-compressor`. Three independent stereo-linked peak detectors,
  hard-knee transfer curves and gain-reduction envelopes. Each band exposes
  `BandDynamicsParams`: threshold, ratio, attack, release and makeup gain.
- `MultibandMaximizer` / `MultibandMaximizerParams`: suggested
  `EffectKind::MultibandMaximizer`, displayed **Multiband Maximizer**. Maps to
  `fx-maximus`. Editable three-band compression followed by an independent
  look-ahead limiter on each band and another ceiling limiter on the sum.
  Exposes independent band ceilings plus input gain, master ceiling and master
  limiter release. Band limiters use their band's release, clamped to the reused
  limiter's supported 1–1000 ms range; compressor release supports 5–2500 ms.
- `TransientShaper` / `TransientShaperParams`: suggested
  `EffectKind::TransientShaper`, displayed **Transient Shaper**. Maps to
  `fx-transient-processor`. Independent attack and sustain amounts (-1 to 1,
  corresponding to up to 18 dB cut/boost), plus contrast sensitivity. A fast
  envelope (0.15 ms rise, 5 ms fall) is followed by a slower envelope (15 ms
  rise, 120 ms fall); their normalized positive contrast identifies attacks.
  The complementary region controls the sustain/body gain.
- `TransientSplit` / `TransientSplitParams`: suggested `EffectKind::TransientSplit`,
  displayed **Transient Split**. Maps to `fx-transmitter`, whose actual summary
  is transient/sustained separation, not excitation. `split_frame([left, right])
  -> [[f32; 2]; 2]` returns independently gained transient/sustain frames. Before
  gains, sustain is exactly input minus transient; unity gains reconstruct within
  floating-point tolerance. `process()` recombines them. Controls are transient
  gain, sustain gain and sensitivity, distinct from Transient Shaper. This is
  envelope-mask separation, not source separation or a spectral decomposition.
- `OneKnob` / `OneKnobParams`: suggested `EffectKind::OneKnob`, displayed
  **One Knob**. Maps to `fx-soundgoodizer`. Only public control is amount (0–1).
  The constrained maximizer recipe progressively lowers different thresholds,
  increases ratios and makeup, drives input up to 8 dB, and lowers the master
  ceiling from 0 to -0.3 dB. Fixed band attack/release pairs are 15/180, 8/100
  and 2/70 ms. At zero amount compression and makeup are unity; crossover phase,
  latency and sample ceiling protection still apply.
- `BassHarmonics` / `BassHarmonicsParams`: suggested `EffectKind::BassHarmonics`,
  displayed **Bass Harmonics**. Maps to `fx-low-lifter`. Low-band saturated
  squaring generates even harmonics (notably an octave above the fundamental).
  The generated path is high-passed at half the bass cutoff to remove DC, then
  mixed additively. Controls: bass cutoff, drive and amount.
- `Exciter` / `ExciterParams`: suggested `EffectKind::Exciter`, displayed
  **Exciter**. Separate high-band nonlinear residual with editable excitation
  frequency, drive, mix and asymmetric color. Small-signal tangent subtraction
  avoids a simple broadband gain boost. Does not claim `fx-transmitter` parity;
  there is no exciter row among the supplied ids. No additional row is closed.

## Crossover, latency and audio contracts

The LR4 crossover cascades two second-order Butterworth sections per branch.
For lower-edge branches L1/H1 and upper-edge branches L2/H2:
`low = L1*(L2+H2)`, `mid = H1*L2`, `high = H1*H2`.
The upper-edge all-pass on the low band ensures the total is
`(L1+H1)*(L2+H2)`: unity magnitude with frequency-dependent all-pass phase.
Two-band mode merges mid and high and retains the compensation stage.
This is a causal IIR crossover, not a linear-phase crossover. Reported
crossover latency is zero; a dry parallel blend can exhibit phase interaction.
Steady unity recombination is tested within 0.02 dB across both modes at
44.1, 48 and 96 kHz, including both crossover points and 20 Hz–16 kHz probes.
Coefficient transitions are not guaranteed to retain the steady-state response.

Requested frequencies are range sanitized, then constrained internally:
lower edge at most 0.35 times the sample rate; upper edge at least 1.25 times
the effective lower edge and at most 0.45 times the sample rate. Damaged sample
rates fall back to 48 kHz, finite rates are constrained to 8–384000 Hz.

- Band Split, Multiband Compressor, Transient Shaper and Transient Split: **0 samples**.
- Multiband Maximizer and One Knob: **two fixed 1 ms look-ahead stages**, each
  rounded to at least one frame. At 48 kHz total latency is **96 samples**;
  query `latency_samples()` after prepare. The existing prepared `Limiter` is
  reused for each stage, with stereo linking and smoothed gain.
- Bass Harmonics and Exciter: **13 samples**, exported as
  `HARMONICS_LATENCY_SAMPLES`. The generated path runs at 2x with causal linear
  interpolation and the existing halfband decimator (12 samples), plus one
  interpolation frame. The recombined dry path uses a matching fixed delay.
  Oversampling reduces aliasing; this does not promise alias-free excitation.

Continuous gains, ratios, envelope coefficients and amounts glide over 5 ms;
crossover and wet high-pass coefficients glide over 20 ms. Changes before the
first frame after prepare/reset snap to current targets. Every state advances
per frame, including denormal flushing, so partitioning is deterministic.
`set_tempo` is the no-op trait implementation. Construction and prepare may
allocate (the reused limiters have preallocated delay and envelope buffers).
All process paths, split-frame seams, parameter updates and resets use only
fixed state/prepared storage, with no allocation, frees, locks, waits or IO.

Compressor and Maximizer opt into `process_sidechain`. The external stereo key
passes through a separate matched three-band crossover and drives only the band
detectors. Missing/short key frames are zero; the audible signal always remains
the main input. Per-band and master ceiling limiters protect the actual audible
signal independently of the key. Other processors use the default trait path.

Inputs replace nonfinite values with silence and bound extreme finite values
to +/-1000. Outputs are finite and similarly guarded. Compressor/crossover and
harmonic tails report conservative IIR decay bounds (64 periods of their lowest
filter edge, plus any explicit latency). Transient processors have no audible
tail on zero input. Limiting guarantees sample peaks, not intersample true peaks;
no calibrated loudness target, LUFS or true-peak claim is made.

## Deferred parity and integration

`fx-emphasis` is **skipped**. Its summary calls for a multistage mastering limiter
that preserves dynamics and tonal balance. A tilt-plus-transient alias would not
implement that summary, and another maximizer alias would duplicate the existing
workflow. A future distinct mastering processor needs its own control contract
and dynamics/tonal-balance measurements. No proprietary product names, presets,
curves or content are used.

Independent-output engine routing still needs the host to consume `split_frame`
for Band Split and Transient Split; the Effect trait itself has only recombined
stereo outputs. The integration owner should add union variants, descriptor
dispatch, serde project wiring, automation/UI and generated bindings without
changing these public parameter descriptor indices. Parity rows should remain
partial until these application seams and appropriate UI are integrated.

## Verification

Only this module is rustfmt formatted. Targeted validation:
`cargo test -p windfall-dsp --lib multiband`, with `scripts/msvc-env.sh` sourced
in Git Bash and `TS_RS_EXPORT_DIR` pointing to temporary generated bindings.
Module tests measure unity crossover magnitude, independent low-band compression,
bass/high-band harmonics, transient attack versus sustained gain, complementary
split outputs, sample ceiling and reported latency, detector-only sidechain,
finite output, serde/default/descriptor agreement and exact partition invariance
through control changes, empty blocks and reset. No workspace suite or UI/CI work.
