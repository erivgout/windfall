# Control Surface integration seam

Parity id: `fx-control-surface`, **partial parity**. Windfall's display name is
**Control Surface**. This fixed surface provides eight knobs on one effect.
It does not provide a visual layout editor or MIDI learn.

Types in `crates/windfall-dsp/src/surface/mod.rs`:

- `ControlSurface`: `Default`, implements
  `Effect<Params = ControlSurfaceParams>`. Both stereo buffers pass through
  bit for bit, at defaults and at every knob position, including nonfinite
  audio and signed zero. Audio latency and tail are zero.
- `ControlSurfaceParams`: `Copy`, `Default`, `PartialEq`, serde camelCase,
  `ts_rs::TS`, and `ParamSet`, with display name `Control Surface`. The eight
  public `f32` fields and JSON keys are `knob1` through `knob8`, in that stable
  descriptor order. Each is a linear fraction from 0 to 1, default 0.5.
  `param_set!` clamps finite out-of-range values and replaces NaN and either
  infinity with 0.5. `set_params` sanitizes the whole incoming struct.
- `ControlSurface::readout(&self) -> [f32; 8]`: copies the eight sanitized
  values in knob1 through knob8 order, without allocating. Updates are
  immediate after `set_params`, independent of audio blocks. The host owns
  routing and publication to other threads; this is not a shared atomic meter.

Storage is a single parameter struct. Construction and `prepare` allocate
nothing. `process`, `set_params`, `reset`, and `readout` do not allocate,
reallocate, free, lock, or perform IO. `prepare` and `reset` preserve current
controls; there is no audio history or smoothing. Sample rate and maximum
block length do not affect the controls or audio. Empty blocks are supported.

Limits: fixed normalized controls only. No control routing, modulation output,
device access, custom widgets, layout serialization, or UI is implemented.
The integration owner must declare `pub mod surface;` and separately register
the effect and wire host parameters and bindings. The module declaration is
removed after verification. The TS derive does not auto-export binding files.
This code is part of Windfall under GPL-3.0.

At the repository root, re-read the current `lib.rs` and temporarily add only
`pub mod surface;` immediately before testing. Run from Git Bash
(`C:\Program Files\Git\bin\bash.exe`):

```bash
source scripts/msvc-env.sh
cargo test -p windfall-dsp --lib surface
```

The tests cover metadata and serialization, bit-exact unity, each knob's
clamping and nonfinite fallback, preserved readout on reset/prepare, and block
split equivalence across parameter edits. With the same temporary declaration
still present, run the standalone allocator probe against the production library:

```bash
cargo build -p windfall-dsp --lib
rustc --edition=2024 --test crates/windfall-dsp/src/surface/allocator_probe.rs \
  --extern windfall_dsp=target/debug/libwindfall_dsp.rlib \
  -L dependency=target/debug/deps -o target/surface-allocator-probe.exe
target/surface-allocator-probe.exe
```

The probe checks zero callback allocations, reallocations, and frees, including
parameter sanitization, reset, readout, empty processing, and preparation. A
positive control verifies the counter observes all three heap operations.
After validation, delete only the temporary `pub mod surface;` line; preserve
all other edits to `lib.rs`. Do not run binding generation for this seam.
