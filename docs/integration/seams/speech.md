# Speech Voice integration seam

`crates/windfall-dsp/src/speech/` implements `speech::SpeechVoice` using the
existing `Instrument` trait. `speech::SpeechVoiceParams` implements `ParamSet`
through `param_set!`, with display name **Speech Voice**. Public phrase types
are `speech::Phoneme` and `speech::PhonemeBuffer`; `MAX_PHONEMES` is 32.
Parameters and phrase types derive `Copy`, serde with camelCase, and `ts_rs::TS`.
No automatic TypeScript export tests are enabled for these new types.

Parity id: `inst-speech-synthesizer`. This is a **partial stand-in**: a pulse
and deterministic noise source excite three parallel formant resonators.
It supplies vowel-like `ah`, `ee`, `oo`, `eh`, `oh` codes, plus `silence`,
`noiseBurst`, and `nasal`. It accepts no natural-language text and provides
no recorded speaker, voice samples, text-to-speech engine, language model,
pronunciation dictionary, or intelligibility guarantee. No external assets,
network access, or added dependencies are required.

The integration owner must add the module declaration, instrument registry
variants, persistence/UI support, bindings, parameter coverage, and parity
accounting. This implementation does not register the instrument or claim
complete parity.

## Parameters and automation

| Index | JSON path | Range | Default |
| --- | --- | --- | --- |
| 0 | `rate` | 0.5–30 codes/second | 6 |
| 1 | `pitch` | -24–24 semitones relative to key | 0 |
| 2 | `releaseMs` | 5–2000 milliseconds | 80 |
| 3 | `level` | 0–1 gain | 0.8 |
| 4 | `phrase.length` | 0–32 codes | 5 |
| 5–36 | `phrase.codes.0` through `phrase.codes.31` | Phoneme choice | See below |

Choice order is silence, ah, ee, oo, eh, oh, noise burst, nasal. The default
phrase is ah, ee, oo, eh, oh; unused slots contain silence. `PhonemeBuffer`
holds `[Phoneme; 32]` and a `u8` length. `from_slice` truncates at 32; zero
length means an empty phrase. The parameters contain no heap storage.
Nonfinite scalar parameters revert to defaults; finite values clamp to the
descriptor ranges. Phrase length clamps to 32. All slots are individually
addressable through `ParamSet` even when outside the current phrase length.

## Runtime behavior and limits

- One voice; `MAX_VOICES = 1`, `LATENCY_SAMPLES = 0`, and no tail after the
  voice becomes inactive. Stereo output is dual mono, replacing both slices.
- `note_on` starts the phrase at MIDI key pitch and velocity. Retriggering
  replaces the old voice and resets oscillator, filter, envelope and noise
  state. Velocity at or below zero, or nonfinite velocity, releases that key.
- Each code lasts `1 / rate` seconds. Phrase completion releases the final
  code; there is no looping. `note_off` releases only the current key and
  freezes phrase progression. Release duration is latched when release starts.
  `all_notes_off` shortens any held or releasing voice to a 5 ms fade.
- Pitch, rate, release control and output gain glide on live changes over
  approximately 5 ms. Phrase edits jump immediately without restarting the
  phrase clock; shortening below the current position starts release with
  silence. Formant coefficients glide between code colors. Silence codes
  output exact zero while clocks and envelope continue.
- `prepare` clamps sample rate to 8–384 kHz and uses 48 kHz for nonfinite
  input. Excitation pitch is capped at one fifth of the sample rate; formants
  stay below 45% of the sample rate. High MIDI notes retain distinct formant
  colors but do not promise alias-free excitation.
- All synthesis state is inline. Construction and preparation are also
  allocation-free; notes, parameter changes, reset and processing never
  allocate, reallocate, free, lock or perform IO. Output is finite and bounded
  to magnitude 1. Timing and smoothing advance per sample, so the same event
  timeline produces identical output across block splits.

## Verification

With `pub mod speech;` temporarily present in `crates/windfall-dsp/src/lib.rs`,
run from Git Bash:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-dsp --lib speech
```

Tests cover audible level and distinct spectral colors for all sounding codes,
exact silence, deterministic block splits with control/note events, pitch and
velocity response, phrase rate/completion, release, retrigger/reset, empty and
oversized phrases, serde/TS/descriptors, and finite extreme inputs.

The allocator test compiles a separate executable with `rustc`, using the
actual speech/parameter/math sources and the actual `Instrument` trait extracted
from its source. It uses built dependency rlibs beside the test executable.
This avoids competing with the test library's existing global allocator.
A positive control detects alloc/realloc/free, then callback exercises require
zero of all three. Compilation and subprocess IO happen only in the test host,
outside the measured callback interval. The probe requires the workspace's
Rust toolchain and linker environment used by the test command.

Implementation validation: all six speech tests passed in an isolated `rustc
--test` build against the actual instrument trait and DSP sources, including
the allocator subprocess. The requested Cargo command was attempted but blocked
by an unrelated `tuner/tests.rs:96` call to `TunerParams::decl()` without the
required `ts_rs::Config` argument. The temporary speech module declaration was
removed; no other module was repaired as part of this work.
