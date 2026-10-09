# EQ bank DSP seam

The new family lives entirely in `crates/windfall-dsp/src/eqbank/`. It is ready
for host integration but is not registered in the effect enum, project model,
engine, desktop editor, or bindings. The final source tree intentionally has no
`pub mod eqbank;` export in `lib.rs`.

## Types and parity scope

- `SevenBand` / `SevenBandParams`, display name **Seven Band**:
  seven serial bell filters centered at 63, 160, 400, 1000, 2500, 6300, and
  16000 Hz. Fixed Q is 1.4. `gainsDb` contains seven gains in ascending frequency
  order, each -18 to +18 dB and initially zero. Target parity ID:
  `fx-fruity-7-band-eq`.
- `MorphEq` / `MorphEqParams`, display name **Morph EQ**:
  two independent stored `snapshotA` / `snapshotB` values, each a
  `MorphSnapshotParams` containing four `MorphBandParams` bells. Each band has
  `frequencyHz` (20–20000), `gainDb` (-18 to +18), and `q` (0.1–10).
  Both snapshots default to flat bands at 150, 600, 2500, and 10000 Hz, Q 1.
  `morph` runs from 0 (A) to 1 (B), initially 0. Frequency and Q interpolate
  logarithmically, gain linearly in dB; endpoints use the snapshots exactly.
  `interpolated_bands()` exposes the sanitized target shapes without changing
  either snapshot. Target parity ID: `fx-equo`, **partial morph EQ only**.
- `FilterBank` / `FilterBankParams`, display name **Filter Bank**:
  eight `FilterSlotParams` slots with `FilterSlotMode::{Bypass, Lowpass,
  Bandpass, Highpass}`, `frequencyHz` (20–20000), and `q` (0.1–10). Defaults
  are bypass, 1000 Hz, and Q = 1/sqrt(2). `FilterRouting::{Serial, Parallel}`
  selects an eight-slot cascade or the mean of active independent branches;
  the default is serial. Bypass slots are excluded from the parallel mean,
  and an entirely bypassed bank passes dry audio. Target parity ID:
  `fx-fruity-love-philter`, **filter-bank half only**.

All parameter structs, including the band, snapshot, and slot helpers, are
`Copy`, `Default`, serde camelCase, `ts_rs::TS`, and implement `ParamSet` through
`param_set!`. Processor descriptor counts are 7, 25, and 25 respectively.
Array descriptors use dotted indexes such as `gainsDb.0`,
`snapshotA.bands.2.frequencyHz`, and `slots.7.mode`. No binding generation or
automatic TypeScript export tests are added by this family.

## Audio behavior and limits

All filters use the shared double-precision cookbook biquad and `CoeffRamp`.
The processors contain only fixed inline state; construction and preparation
also need no heap storage. `set_params`, `set_tempo`, `reset`, and `process`
allocate, reallocate, free, lock, and perform IO zero times. Tempo is ignored.
All three effects report **zero latency**; filter phase is not a compensated
sample delay. `tail_samples()` estimates filter decay to -120 dB from the
current and target pole radii, summing sections conservatively.

Initial settings and reset apply immediately. Later coefficient changes take
5 ms measured in samples. Morph targets describe the interpolated band shapes;
during that 5 ms transition the audio interpolates coefficients. Filter Bank
keeps separate serial and parallel histories running and crossfades routing
over 5 ms. Parallel activation weights also ramp, with dry audio filling a
total weight below one during the first activation or final deactivation.

Zero bell gains and bypassed slots use identity sections and clear stale
histories once their transition settles. Neutral audio is unity within numeric
guards: subnormal-scale outputs below 1e-20 are flushed. NaN/Inf audio becomes
zero before filtering, and output saturates only at the finite `f32` limit.
Parameter values are sanitized before filter design. Sample rates are held to
8–384000 Hz (nonfinite values use 48000); filter centers are held between
1 Hz and 0.49 times the sample rate. High-frequency bells therefore narrow
near Nyquist, and fixed centers above the rate limit are clamped.

This family does not replace the existing parametric EQ or single-filter
processors. Morph EQ has no spectral analysis, automatic matching/capture,
or broader curve editing. Filter Bank has no formula editor, envelope
controllers, modulation sequencer, per-slot gain/pan, or arbitrary routing.

## Validation

Temporarily add exactly `pub mod eqbank;` to
`crates/windfall-dsp/src/lib.rs`, then run in Git Bash
(`C:\Program Files\Git\bin\bash.exe`):

```bash
source scripts/msvc-env.sh
rustfmt --edition 2024 crates/windfall-dsp/src/eqbank/*.rs
cargo test -p windfall-dsp --lib eqbank
```

Remove only the temporary `pub mod eqbank;` line afterward, preserving any
concurrent edits to `lib.rs`.

The family tests measure boost/cut and stopband responses at 44.1, 48, and
96 kHz, all seven fixed centers, all eight filter slots and modes, unity,
independent stereo histories, morph endpoints and midpoint shapes, graph
reference outputs, live routing/mode/gain changes, irregular block splits,
reset behavior, finite malformed audio/parameters, tails, and serde/TS metadata.

`callback_allocator_probe` compiles the actual production library and
`eqbank/allocator_probe.rs` into an isolated test executable using the current
Cargo dependency directory. This avoids conflicting with the library tests'
existing global allocator. Its thread-local allocator counts alloc,
alloc_zeroed, realloc, and dealloc; a positive control proves detection before
the callback probe requires zero allocations, reallocations, and frees for all
three processors. Rustc and the initialized MSVC environment must remain
available while the test runs. Temporary probe artifacts are removed on exit.
