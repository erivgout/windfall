# Tuner integration seam

Parity id: `fx-tuner`, partial parity. Windfall's **Tuner** estimates a single
fundamental from the audio already passing through the effect. It is not a
microphone input device or a hardware calibration certificate.

Types in `crates/windfall-dsp/src/tuner/mod.rs`:

- `Tuner`: `Default`, implements `Effect<Params = TunerParams>`; audio is
  bit-for-bit unity passthrough, with zero audio latency and no tail.
- `TunerParams`: `Copy`, `Default`, serde camelCase, `ts_rs::TS`, `ParamSet`.
  Display name `Tuner`; `reference_hz` / JSON `referenceHz` is MIDI note 69's
  reference, 400–480 Hz, default 440 Hz. Invalid values are sanitized through
  `param_set!`. Reference edits update the display immediately without changing
  the estimated frequency or audio.
- `TunerReadout`: `Copy` snapshot returned by `Tuner::readout()` after each
  block, with `frequency_hz`, `midi_note: Option<u8>`, `cents_error` and
  `confidence` in 0–1. Unavailable estimates have zero frequency/cents and
  `None` for note. Confidence measures periodicity, not a calibrated probability.
  The host owns publication of snapshots to other threads.

The detector averages stereo to mono, decimates with a fixed box average to
6–12 kHz, and applies a cumulative-mean normalized difference detector with
fractional-period interpolation. Storage is fixed: a 1024-sample history plus
fixed analysis scratch arrays. Each analysis checks at most 256 lags against
768 samples; it runs every 256 analysis samples after the window fills.
`process`, parameter edits, readout and reset have no heap activity, locks or IO.
Analysis delay is roughly 85–171 ms initially, with updates every 21–43 ms;
this does not delay the audio. Results persist between analysis hops. Empty,
near-silent (mono RMS below 1e-5) or nonfinite-input blocks immediately invalidate
the readout and history. Reset/prepare clear history and preserve parameters.

Limits: monophonic periodic audio, nominally 50–2000 Hz; sample rates sanitized
to 8–384 kHz (nonfinite rates use 48 kHz). A pure 440 Hz sine at 48 kHz is tested
within one cent at high confidence; 220 and 880 Hz are tested independently.
Noise and DC produce low confidence and no note once the analysis window
contains that input. A transition can retain the previous estimate until its
next analysis window settles. Polyphony, strong missing fundamentals, transients,
stereo cancellation and frequencies outside the range are not reliable tuning
sources. Box averaging is not a full antialias filter; high-frequency content
may alias into the detector. No correction, retuning, device capture, UI,
serialization of detector history or shared atomic meter is provided.

The owning integration agent must add `pub mod tuner;` to `lib.rs` and separately
wire effect registration, parameters and bindings. This implementation leaves
that declaration absent after verification. TS derives do not auto-export files.

From Git Bash (`C:\Program Files\Git\bin\bash.exe`) at the repository root,
temporarily add only `pub mod tuner;` to the current `lib.rs`, then run:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-dsp --lib tuner
```

The standalone allocator probe uses its own global allocator to avoid conflicting
with the library test allocator. With the same temporary declaration still present:

```bash
cargo build -p windfall-dsp --lib
rustc --edition=2024 --test crates/windfall-dsp/src/tuner/allocator_probe.rs \
  --extern windfall_dsp=target/debug/libwindfall_dsp.rlib \
  -L dependency=target/debug/deps -o target/tuner-allocator-probe.exe
target/tuner-allocator-probe.exe
```

The probe verifies zero callback allocations, reallocations and frees, and checks
that its counter detects all three operations. Remove only the temporary
`pub mod tuner;` line afterward, preserving concurrent module declarations.
