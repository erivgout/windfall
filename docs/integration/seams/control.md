# Control-source DSP seam

The parent can expose `pub mod control;` from `windfall-dsp/src/lib.rs`.
This implementation deliberately leaves that file and the effect unions alone.
All seven parameter types are `Copy`, serde camelCase with missing-field
defaults, `ts_rs::TS`, and `ParamSet` with stable descriptor indices.

| Processor / params | Windfall name | Suggested EffectKind | Parity reference |
| --- | --- | --- | --- |
| `FormulaSource` / `FormulaSourceParams` | Formula | `FormulaSource` (control-source registry) | `fx-fruity-formula-controller` |
| `XyPad` / `XyPadParams` | XY Pad | `XyPad` | `fx-fruity-x-y-controller` |
| `XyzPad` / `XyzPadParams` | XYZ Pad | `XyzPad` | `fx-fruity-x-y-z-controller` |
| `PanLfo` / `PanLfoParams` | Pan Motion | `PanLfo` | `fx-fruity-panomatic` |
| `EnvelopeFollower` / `EnvelopeFollowerParams` | Envelope Follower | `EnvelopeFollower` | `fx-fruity-peak-controller` |
| `NoteEnvelope` / `NoteEnvelopeParams` | Note Envelope | `NoteEnvelope` | `core-fruity-envelope-controller` |
| `KeyboardSource` / `KeyboardSourceParams` | Keyboard Source | `KeyboardSource` (control-source registry) | `core-fruity-keyboard-controller` |

`core-fruity-peak-controller` is absent from the current inventory; the effect
row above is the existing peak-controller reference. These are DSP building
blocks for roadmap family E7, not closure of controller routing or UI parity.

## Host API and output types

Construct with `Default`, call `prepare(sample_rate, max_block)`, then
`set_params(&params)`. XY Pad, XYZ Pad, Pan Motion, Envelope Follower and
Note Envelope implement `Effect` for stereo audio slices. Formula and Keyboard
Source are control-only sources with inherent `prepare`, `reset`, and
`set_params` methods; wrapping them as audio effects would need a real host
destination binding.

Every source exposes `process_control<const N: usize>` and fills every entry of
a caller-owned fixed output array. There is no internal output buffer or
block-size allocation. Formula and Envelope Follower take two read-only
`&[f32; N]` audio inputs plus `&mut [f32; N]` output. Note Envelope takes only
`&mut [f32; N]`. XY, XYZ and Pan Motion take respectively `&mut [[f32; 2]; N]`,
`&mut [[f32; 3]; N]`, and `&mut [[f32; 2]; N]`. Keyboard Source takes
`&mut [KeyboardControl; N]`.

The five audio effects additionally expose `process_with_control(left, right,
output)` with mutable audio arrays of the same compile-time length. This
applies the audible behavior and captures that sample's controls in one pass.
Calling `process` followed by `process_control` advances the source twice;
use the combined method when both audio and controls are needed. XYZ's
control-only method advances its axes without processing filter history.

- Formula returns a normalized 0..1 signal. Its `a`, `b` and `base` parameters
  are 0..1, and `amount` is -4..4. Defaults are 0.5, 0.5, 0, and 1.
- XY outputs smoothed normalized X/Y, each 0..1 and default 0.5. Audio maps
  X to stereo balance `2*x - 1` and Y to linear gain `2*y` (0..2).
  At center both channels have exact unity. Balance attenuates the opposite
  channel without boosting the favored channel.
- XYZ outputs normalized X/Y/Z, default 0.5/1/0. X applies the same balance.
  Y sets a stereo 12 dB/octave TPT lowpass cutoff `20 * 1000^y` Hz, held below
  Nyquist by the shared SVF. Z independently raises Q from about 0.707 to 8.
  Both stereo filter histories persist across blocks; this is actual resonant
  audio filtering. The default high cutoff still filters; it is not bypass.
- Pan Motion outputs `[pan, gain]`, with pan -1..1 and gain 0..2. A free-running
  sine LFO (0.01..20 Hz, default 1) moves pan around `pan` (-1..1, default 0)
  with `depth` (0..1, default 1), clipping at the pan endpoints. `volumeDepth`
  (0..1, default 0) attenuates gain by the unipolar LFO; `gain` defaults to 1.
  Phase starts at zero on reset. The resulting pan and gain process audio.
- Envelope Follower outputs a normalized control. It follows the maximum
  absolute stereo sample, clamped to 0..1, multiplied by `inputGain` (0..16,
  default 1), and clamped again. Separate `attackMs` (0..2000, default 10) and
  `releaseMs` (0..10000, default 100) are exponential 63% time constants.
  Control is `clamp(base + amount * (envelope + lfoDepth * unipolarSine), 0, 1)`.
  `base` is 0..1/default 0, `amount` -4..4/default 1, `lfoDepth` 0..1/default 0,
  and `rateHz` 0.01..20/default 1. Set `inputGain` to zero for LFO-only control.
  `duck` (0..1, default 0) applies audio gain `1 - duck * control`, using the
  pre-duck input detector. Signed amount with positive base allows inversion.
- Note Envelope outputs ADSR level 0..1. A rising `trigger` boolean starts
  attack; holding it sustains; a falling edge releases. Reapplying true does
  not retrigger. `trigger()` explicitly retriggers from the current level and
  sets the internal gate true; send false through `set_params` to release.
  Attack/decay/release are 0..10000 ms, default 10/100/200; sustain is
  0..1/default 0.7. Segments use the shared exponential ADSR and zero times
  transition on the next sample. Audio gain is
  `gain * (1 - depth + depth * envelope)`, with depth 0..1/default 1 and
  gain 0..2/default 1. It starts silent with the default gate off. Reset
  clears the envelope and restarts attack if its current gate is true.
- Keyboard Source receives `set_note(key: u8, velocity: f32)` from the host.
  Key is clamped to MIDI 0..127 and velocity is normalized 0..1; invalid velocity
  becomes zero. `KeyboardControl` is a Copy serde/TS struct containing
  `pitchSemitones` and `gainOffset`. Pitch is
  `(key - rootKey) * pitchAmount + transpose` (-302..302 semitones).
  Gain offset is `velocityAmount * (velocity - 1)` (-1..0); add it to unity
  gain before applying it. Params are rootKey 0..127/default 60, pitchAmount
  0..2/default 1, transpose -48..48/default 0, velocityAmount 0..1/default 1.
  The last supplied key and velocity win; pass the held key and zero velocity
  to release. Reset restores the root key and zero velocity. There is no
  polyphonic note stack or audio pitch-shifting stage.

## Formula expression subset

`FormulaSource::set_expression(&str) -> Result<(), FormulaError>` installs a
bounded compiled postfix program. Default expression: `a * b + peak`.
Persist the expression text separately from the Copy numeric parameter struct,
then compile it when restoring a project. The host owns the editor text;
the audio source owns only a fixed operation array. There is no scripting VM,
heap AST, external lookup, assignment, loop, or user-defined function.

Accepted syntax is case-sensitive: `a`, `b`, `peak`; decimal floating-point
literals with optional scientific notation; `+`, `*`, unary `-`; parentheses;
`sin(expr)`, `abs(expr)`, `min(expr, expr)`, `max(expr, expr)`; and ASCII
whitespace. Multiplication binds more tightly than addition. Sine takes
radians. Subtraction can be written as `a + -b`; division, exponentiation,
implicit multiplication and other identifiers are rejected.

For example, `min(max(abs(-a), b), 0.7) + sin(peak) * 0.1` is accepted.
`peak` means the instantaneous maximum absolute input stereo sample clamped
to 0..1, not the maximum over a whole block. This keeps results independent
of host block boundaries.

Limits: 256 UTF-8 bytes of text, 64 emitted operations, 16 syntax levels
(the outer atom occupies one level), and a 64-entry evaluation stack.
Numeric literals must be finite and at most 1,000,000 in magnitude; binary
intermediates saturate to +/-1,000,000. The final normalized control is
`clamp(base + amount * expression, 0, 1)`. Compilation returns `TooLong`,
`TooComplex`, `Syntax` or `InvalidNumber`; any failure keeps the previous
program intact. Expression changes take effect on the next processed sample.
Parsing and evaluation allocate nothing and cannot execute unbounded work.

## Runtime, integration and verification

All processors own fixed-size state. Construction, preparation, parameter
changes, note/gate calls, expression installation, reset and processing
allocate and free nothing, take no locks, and perform no I/O. Parameters
sanitize through `ParamSet`; invalid sample rates become 48 kHz, valid rates
clamp to 1..384000 Hz. Damaged audio is replaced with zero and finite sample
magnitude is capped at 1000 before stateful processing. Output is finite.

Axes, formula knobs, keyboard pitch/gain destinations, pan/LFO controls,
ducking and audio gains ramp linearly over 5 ms. Initial settings after
prepare/reset snap. Envelope segment times update coefficients between
blocks without resetting the envelope level; sustain uses the shared ADSR's
10 ms glide while held. Trigger edges and program replacements are discrete
events. All state advances per sample, including zero-input processing.
There is no tempo sync in this seam; `Effect::set_tempo` uses its no-op default.

All audio effects report zero latency. XYZ reports a conservative ten-second
filter tail with a 20,000-sample minimum for unusually low sample rates;
the other audio effects store no delayed audio and report zero
audio tail. Control scheduling must continue while a controller is active,
even when its audio input is silent: LFOs, held notes, formula constants,
keyboard state and envelope releases can still produce control values.
The parent owns control port binding, destination ranges, cycle protection,
sample-accurate event splitting, project persistence of expression text,
effect-union additions, TypeScript export registration, and the controls UI.
Arbitrary drawn envelopes and external send routing are outside this module.

Module tests cover formula precedence/functions/rejection bounds, XY balance
and gain, XYZ resonance, follower attack/release and audible ducking, ADSR
attack/sustain/release/retrigger, audible pan/volume LFO, keyboard offsets,
finite output, parameter descriptor/default/serde consistency, and exact
audio block invariance during parameter glides. An isolated rustc harness
compiled this module with the existing Effect trait and parameter/block
implementations and ran only `control::tests::` (11 tests passed), without
adding the module to `lib.rs` or running a workspace suite. A separate
temporary callback probe reused the existing realtime allocation counter;
construction, prepare, parameter/expression/note/gate changes, reset, audio,
control-only and combined callbacks made zero allocator calls. The probe
passed and temporary harness files were removed. Only this module's Rust
files were formatted.
