# Stage Stack integration seam

Windfall's **Stage Stack** supplies partial parity for `fx-emphasis` and
`fx-emphasizer`. This GPL-3.0 implementation is not the existing Limiter,
not the multiband maximizer, and not a loudness meter.

Types in `crates/windfall-dsp/src/mastering/mod.rs`:

- `mastering::StageStack`: `Default`, implements `Effect<Params = StageStackParams>`.
- `mastering::StageStackParams`: `Copy`, `Default`, `PartialEq`, serde camelCase,
  `ts_rs::TS` and `ParamSet` via `param_set!`; display name `Stage Stack`.
  TS derives do not automatically export bindings.

Stable parameter order:

| Index | Rust field / JSON id | Range | Default |
| --- | --- | --- | --- |
| 0 | `drive_db` / `driveDb` | 0–36 dB | 0 |
| 1 | `tone` / `tone` | 0–1, odd to added even color | 0 |
| 2 | `ceiling_db` / `ceilingDb` | -24–0 dBFS | 0 |
| 3 | `mix` / `mix` | 0–1, dry to wet | 1 |

Stage one uses `t = tanh(input * driveGain / 2)` and
`2*t + tone * 0.5*t²*(1-t²)`. The transfer is smooth, bounded to magnitude
2.125, and has unity small-signal slope at neutral drive. Tone zero is odd;
increasing tone adds an even component with exact silence at zero input.
It has no hard ceiling. Intermediate input multiplication uses double precision
so maximum finite float input at maximum drive cannot overflow.

Stage two detects the current saturated stereo peak, applies instant shared
gain reduction at the ceiling, and recovers with a fixed 3 ms one-pole smoother
(63% recovery time). A final sample clip catches rounding. The retained gain
prevents recovery from chattering between peaks. No audio is buffered or delayed:
reported latency, warm-up and tail are zero. Silence produces exact silence.
This circuit has no look-ahead queue, crossover bands or loudness analysis.

The dry/wet blend follows both stages. Mix zero is exact unity for finite input,
including neutral drive and a 0 dB ceiling. The ceiling applies to the wet path;
partial mixes can exceed it because the dry path is unrestricted. Nonfinite
audio samples become zero, and a double precision convex blend keeps extreme
finite input finite. Invalid parameters revert to defaults; finite values clamp
to their ranges. Sample rates clamp to 1–384000 Hz; nonfinite rates use 48000 Hz.
Live control edits ramp over 5 ms, with a lowered ceiling enforced immediately.
Settings before the first sample after prepare/reset apply immediately. Reset
clears reduction and snaps controls to their current targets. Tempo is ignored.
Per-sample state advancement makes irregular block splits identical to one block
when parameter edits occur at the same sample positions.

Limits: nonlinear shaping can alias; there is no oversampling or true-peak
protection. Even color can introduce a DC component; no DC filter is included.
The safety stage can distort transients and duck both channels following a peak.
No meters, presets, UI, registry wiring, external assets or dependencies are
added. Construction and preparation need no heap storage; `process`,
`set_params`, `reset` and `set_tempo` never allocate, reallocate, free, lock or
perform IO. State is inline and independent of the maximum block size.

The integration owner must add `pub mod mastering;` to `lib.rs` and separately
wire effect registration, persistence, bindings and parity accounting. This
implementation leaves the declaration absent after testing.

From Git Bash (`C:\Program Files\Git\bin\bash.exe`) at the repository root,
temporarily add only `pub mod mastering;` to the current `lib.rs`, then run:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-dsp --lib mastering
```

Unit tests cover descriptors/serde/TS, exact dry unity, harmonic shape,
ceiling reduction and immediate output, linked release/reset, live automation
across irregular blocks, nonfinite/extreme audio and rates, and ceiling edits.
The standalone allocator probe avoids competing with the library test allocator:

```bash
source scripts/msvc-env.sh
cargo build -p windfall-dsp --lib
rustc --edition=2024 --test crates/windfall-dsp/src/mastering/allocator_probe.rs \
  --extern windfall_dsp=target/debug/libwindfall_dsp.rlib \
  -L dependency=target/debug/deps -o target/mastering-allocator-probe.exe
target/mastering-allocator-probe.exe
```

The probe exercises processing, live edits, empty blocks, tempo and reset,
requiring `[alloc, realloc, free] = [0, 0, 0]`. A positive control verifies all
three counters. Remove only `pub mod mastering;` afterward, preserving any
concurrent declarations, including speech and tuner.

Implementation verification: the required Cargo command passed all eight
mastering unit tests. Both standalone allocator tests passed, including zero
callback allocations, reallocations and frees. The new Rust files were formatted
with rustfmt, and the temporary mastering module declaration was removed.
