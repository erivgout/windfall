# Analog instrument seam

The implementation lives entirely in `crates/windfall-dsp/src/analog/`.
Each processor implements the existing `crate::instrument::Instrument` trait.
The production module declaration and host integration are deliberately absent.

## Types and parity targets

| Instrument | Parameters | Parity row ids | Implemented scope |
| --- | --- | --- | --- |
| `AcidLine` | `AcidLineParams`, `AcidStep` | `inst-transistor-bass` | Dedicated mono resonant bass, velocity/manual or per-step accent, held-note slide, note-gated internal sixteen-step sequencer. |
| `TripleOsc` | `TripleOscParams`, `AnalogOscParams`, `AnalogEnvelopeParams` | `inst-simsynth`, `inst-poizone` | Three independent waveform, pitch, detune and level controls per voice; one mixed filter and envelope path. |
| `WaveLane` | `WaveLaneParams`, `AnalogEnvelopeParams` | `inst-sawer` | Small original harmonic wavetable, continuous scan, filter and amplitude envelope. This row is a partial synthesis target, not a full feature or sonic match. |
| `MacroVoice` | `MacroVoiceParams` | `inst-flex`, `inst-gms` | **Partial macro instrument**: four macros crossfade filtered oscillator, two-operator phase-modulation FM and harmonic wavetable engines. No preset packs or full source-instrument parity. |

All parameters are `Copy`, `Default`, serde camelCase, `ts_rs::TS`, and
`ParamSet` through `param_set!`. Nested oscillator/envelope parameters have
their own descriptors. Display names describe controls plainly. Nothing uses
third-party product names in processor names, strings or code comments.

Indices are append-only in each descriptor table. Initial layouts:

- `AcidLineParams`: 0–10 global controls; 11–74 are sixteen step groups of
  enabled, pitch offset, accent, slide. `steps.15.slide` is index 74.
- `TripleOscParams`: 0–11 are three oscillator groups of waveform, semitones,
  detune cents, level; 12–14 filter; 15–18 envelope; 19 output level.
- `WaveLaneParams`: 0 scan; 1–3 filter; 4–7 envelope; 8 output level.
- `MacroVoiceParams`: 0 engine blend; 1 brightness; 2 modulation depth;
  3 harmonic shape. Blend 0 is the filtered oscillator, 0.5 FM, 1 wavetable,
  with linear crossfades between adjacent engines.
- `AnalogOscParams` and `AnalogEnvelopeParams`: four controls each, in their
  corresponding nested order above.

## Scheduling, lifetime and limits

`TripleOsc`, `WaveLane` and `MacroVoice` each have eight fixed voices,
including releases and steal fades. Allocation policy is first idle slot,
then oldest released voice, then oldest held voice. Ages are compacted bounded
ranks. A stolen voice carries its last sample into a five-millisecond decay
while the new envelope attacks. This is a short de-click fade, not an extra
rendered voice. Repeated same-key notes occupy independent slots; `note_off`
releases all held instances of that key. Instance ids, per-note expression,
pan, MPE and external modulation routing are not implemented.

`AcidLine` is monophonic with last-note priority and no held-note stack.
An older key's note-off cannot release the newest key; releasing the newest
key does not restore an older note. Manual slide applies to overlapping held
notes when `manualSlide` is on. Manual accent uses velocity at least 0.8.
Accent changes amplitude and filter-envelope brightness.

With `sequencer` enabled, note-on sets the transposition root, starts step 0
immediately and restarts the pattern. Steps are fixed sixteenth notes;
`set_tempo` supplies 20–400 BPM, with nonfinite input falling back to 120.
Tempo edits preserve fractional step progress. Each enabled step has an
integer ±24-semitone offset, accent and outgoing slide. Slide holds the gate
and connects to the following enabled step without envelope retriggering;
a disabled step breaks that connection and releases the voice. Non-slide
steps release at the configured gate fraction. Note-off and all-notes-off
stop the clock. An armed sequencer counts as one active voice during rests
because it can generate another internal note. Reset returns to step 0 with
the current controls and tempo but no held note. Toggling the sequencer while
held restarts step 0 or returns to the manually played root.

All continuous controls use five-millisecond sample-driven smoothing, with
negligible changes snapped to their target. Controls set before the first
sample after prepare/reset apply immediately. Discrete controls jump.
Slide is a linear pitch ramp in semitones. All-notes-off uses a five-millisecond
fade independent of envelope release. Latency and post-voice tails are zero.
Stereo outputs currently carry the same mono mix.

Wave tables are `Copy` inline arrays: three shapes × four harmonic caps
(1, 2, 4, 8) × 256 samples. `prepare` builds them from sines; playback linearly
interpolates phase and adjacent shapes. Harmonic mip selection avoids partials
above 0.45 of the sample rate but changes levels discretely; this is not a
full anti-aliasing or spectral-quality guarantee. There is no file/image
import, user wavetable editor or external content. FM uses bounded phase
modulation without oversampling and does not claim alias-free output.

Sample rates are sanitized to 8–192 kHz, falling back to 48 kHz. Nonfinite
controls use descriptor defaults; finite extremes clamp to the documented
ranges. Oscillator increments are bounded by the reusable block's limit.
Output is finite and bounded by tanh. Callback paths hold no heap-owned state
and do not allocate, reallocate, free, lock or do I/O. Blocks use only their
sample counts and are deterministic across splitting, including automation,
tempo changes and note events applied at the same sample.

## Validation

Temporarily add exactly `pub mod analog;` to `crates/windfall-dsp/src/lib.rs`.
From Git Bash (`C:\Program Files\Git\bin\bash.exe`), at the repository root:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-dsp --lib analog
```

Module tests cover sound and silence, release and emergency stop, parameter
defaults/indices/serialization and TypeScript declarations, direct nonfinite
fields, extreme rates/notes/controls, block splitting, reset repeatability,
bounded steal priority and ages, mono priority, slide, sequencer timing/tempo,
disabled steps, accent, oscillator independence, harmonic tables and macros.

The library already installs a global allocator in another family's unit
tests. `analog/verify.rs` is therefore a standalone probe linked against the
real library, with its own thread-local counting allocator. While the
temporary module declaration is still present, run:

```bash
cargo build -p windfall-dsp --lib
rlib=$(ls -t target/debug/deps/libwindfall_dsp-*.rlib | head -n 1)
rustc --edition=2024 --test crates/windfall-dsp/src/analog/verify.rs \
  --extern "windfall_dsp=$rlib" -L dependency=target/debug/deps \
  -o target/debug/analog-allocator-probe.exe
target/debug/analog-allocator-probe.exe
```

The probe first proves that it observes allocation, reallocation and free,
then requires `[0, 0, 0]` for prepared audio callbacks across all four
instruments, including parameter edits, notes, stealing, reset, tempo, stop,
queries and empty blocks. Prepare and destruction are outside the probe.

Remove only the temporary `pub mod analog;` line afterward, including after
failures. Rustfmt only the new analog files. Do not generate bindings.

## Remaining integration

An integration owner must add the production module declaration, central
instrument kind/parameter/runtime variants, descriptor dispatch, project
serialization/validation, bindings and host/UI wiring. No such changes are
included here. No parity row is marked complete; the row ids above are
directional integration mappings, not evidence of full feature coverage,
compatible presets/projects or a matched sound. No factory packs, complete
patch browser, sequencer editor, automation editor, expression integration or
reference-product measurement is claimed.
