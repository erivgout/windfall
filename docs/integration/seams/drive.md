# Drive DSP seam

The public API is `windfall_dsp::drive`. Each processor implements `Effect`,
processes stereo slices in place, and exposes a `Copy`, serde camelCase,
`ts_rs::TS` parameter struct implementing `ParamSet`.

| Processor / params | Product name / suggested EffectKind | Parity reference |
| --- | --- | --- |
| `Waveshaper` / `WaveshaperParams` | Waveshaper / `Waveshaper` | `fx-fruity-waveshaper` |
| `Overdrive` / `OverdriveParams` | Overdrive / `Overdrive` | `fx-fruity-blood-overdrive` |
| `GuitarRack` / `GuitarRackParams` | Guitar Rack / `GuitarRack` | `fx-hardcore-11-guitar-fx` |
| `DriveChain` / `DriveChainParams` | Drive Chain / `DriveChain` | `fx-distructor` |

## Parameters and algorithms

- Waveshaper has `inputDb` (-24..36 dB), `outputDb` (-24..12 dB), and
  `curve: [number; 33]`. The fixed input knots span -1..1 in steps of 1/16.
  Each output knot is sanitized to -1..1. Lookup is linear between knots
  and holds the endpoints outside the domain. Default gains are 0 dB and
  `IDENTITY_CURVE` gives exact unity for modest finite audio. Descriptor
  indices 0 and 1 are gains; indices 2..34 address `curve.0`..`curve.32`.
  A curve editor can write the whole struct or use these descriptor indices.
  An arbitrary curve can introduce a DC offset, including nonzero output
  from zero input when its center knot is nonzero.
- Overdrive has `driveDb` (0..36, default 18), `asymmetry` (0..1, default
  0.7), `emphasis` (0..1, default 0.5), `toneHz` (200..18000, default 6000),
  and `outputDb` (-24..12, default -6). A 900 Hz highpass component boosts
  treble ahead of a rational clip with unequal positive/negative ceilings.
  A 12 dB/octave lowpass follows clipping, then a 5 Hz DC blocker.
  `OverdriveParams::transfer` measures the static curve without the filters.
- Guitar Rack selects **one** `GuitarModel` through `model`. `driveDb`
  (0..36, default 6) precedes the selected algorithm; `outputDb` (-24..12,
  default -6) follows it. Its eleven models are clean boost, an asymmetric
  cubic tube knee, hard fuzz, full-wave octave rectification with DC removal,
  treble boost, a 750 Hz mid scoop, a 450..1800 Hz bandpass swept at 0.65 Hz,
  envelope-controlled compression (5:1 above linear level 0.18, 2 ms attack,
  90 ms release), a hysteretic gate (0.045 open / 0.025 close), short spring
  slap, and cabinet lowpass. The gate has a 0.5/35 ms detector and a 2/20 ms
  gain envelope. Spring slap uses 37/53 ms taps, damping, allpass dispersion,
  and feedback 0.35. It is a short metallic echo approximation. Cabinet is
  two cascaded 12 dB/octave 3800 Hz lowpasses, **not an IR or convolution
  cabinet**; no cabinet asset files are needed. There is no eleven-slot
  simultaneous pedal rack in this API.
- Drive Chain exposes `stages: [DriveStageParams; 3]` and `outputDb`
  (-24..12, default 0). Each stage has `model: DriveStage` and `gainDb`
  (-24..36, default 0). `DriveStage` is `bypass`, `overdrive`, `waveshape`,
  or `fuzz` in descriptor order. Stages run in index order. Bypass ignores
  its gain and passes through. Overdrive uses the asymmetric rational curve;
  waveshape uses a fixed cubic curve; fuzz uses a hard clip. Stage overdrive
  is the static nonlinearity, without the standalone processor's filters.
  There is no multiband split, chorus, or IR cabinet. Defaults bypass all
  stages. Swapping stages changes the measured output. Descriptor indices
  are model/gain pairs for stages 0, 1, 2, then output at index 6.

## Runtime and limits

Construction and `prepare` are control-thread operations. Guitar Rack
allocates its delay lines there. `process`, `reset`, `set_params`, and
`set_tempo` allocate and free nothing, hold no locks, and perform no I/O.
Processing and smoothing advance per sample, independent of block size.
Gains, curve knots, filter tone, and model mixtures ramp over 5 ms. Initial
settings after prepare/reset snap immediately. Rapid model edits retarget
from the current mixture; all guitar model histories continue running so
selection needs no state allocation or destruction.

All processors report zero latency. Overdrive and Guitar Rack conservatively
report a two-second tail; Guitar Rack reports the 53 ms maximum slap gap.
Waveshaper and Drive Chain are memoryless and report zero tail. A Waveshaper
whose center knot is nonzero generates an offset continuously, so the host
must not infer silence from its input alone. Invalid rates and parameters
are sanitized. Input and final output use the shared finite audio guard
(nonfinite values become zero, finite values clamp to +/-1000).

These implementations operate at the host sample rate with **no
oversampling**. Fuzz, rectification, extreme input gain, and steep or
nonmonotonic drawn curves can create audible aliasing, especially near
Nyquist. Overdrive's post lowpass reduces high-frequency energy but cannot
remove already folded aliases. No alias rejection or CPU budget is claimed;
production acceptance still needs spectral and CPU measurements at target
sample rates. Existing Distortion retains its separate oversampled path,
and existing SoftClipper is unchanged.

## Integration boundary and verification

The owner must add these four variants to `EffectKind`, `EffectParams`,
`AnyEffect`, registry descriptor/default/latency matches, and project/command
integration, then regenerate bindings. None of those files is changed by
this module delivery. Suggested serialized variant names are `waveshaper`,
`overdrive`, `guitarRack`, and `driveChain`. The parity IDs above describe
reference scope, not completed product parity: registry, persistence across
the host union, editors, presets, broader reference behaviors, and aliasing /
CPU acceptance remain integration work. In particular, the modular chain
does not claim the reference unit's chorus or speaker cabinets.

Drive-module tests cover identity and bent curves, asymmetric peaks, all
eleven pairwise guitar responses, octave harmonics, delayed slap, cabinet
treble attenuation, gate/compression behavior, chain order, descriptor /
serde / TypeScript contracts, gain smoothing/reset, damaged input, and
bit-identical output under changing block partitions and parameter edits.
Run only the focused package test, redirecting TS exports outside tracked
bindings: `cargo test -p windfall-dsp --lib drive`.

Verification completed: `cargo build -p windfall-dsp --lib` succeeded and
`cargo test -p windfall-dsp --lib drive` passed all 19 selected tests (12
behavior/contract tests and 7 derived binding exports). The only build
warning came from the concurrently implemented spatial family.

`drive/verify.rs` is an independent test root using the compiled real
library. Its counting allocator checks allocations, zeroed allocations,
reallocations, and frees without replacing the package's existing library
test allocator. Both tests passed: the guard detected known heap activity,
and every processor's callbacks produced zero heap activity during 128
rounds of extreme parameter edits, processing, tempo updates, and reset.
After the package build, it can be rerun in the configured MSVC shell with:

```sh
rustc --edition 2024 --test --crate-name windfall_drive_verify \
  crates/windfall-dsp/src/drive/verify.rs -L dependency=target/debug/deps \
  --extern windfall_dsp=target/debug/libwindfall_dsp.rlib \
  -o target/debug/windfall-drive-verify.exe
target/debug/windfall-drive-verify.exe drive
```

Only the drive source files were formatted, with rustfmt's `skip_children`
enabled for the independent verification root. No workspace suite or
frontend checks were run.
