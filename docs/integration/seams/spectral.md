# Spectral DSP integration seam

Source is `crates/windfall-dsp/src/spectral/`. The integration owner must add
`pub mod spectral;` to `lib.rs` and wire the effect unions, registry, persistence,
bindings, UI and engine routing. This assignment changes only that directory
and this document. No parity rows are marked complete.

All five processors implement `crate::effect::Effect`, including default
construction. Their corresponding `*Params` structs implement `ParamSet`, are
`Copy`, sanitize values, and derive serde with camelCase field names and
missing-field defaults, plus `ts_rs::TS`. `PitchScale` also derives serde/TS.
TS types are available for the parent's explicit binding generation; this
module deliberately has no automatic TS export tests that write bindings.

| Type / suggested EffectKind | Windfall display name | Suggested serialized tag | Parity reference |
| --- | --- | --- | --- |
| `Convolver` / `ConvolverParams` | Convolver | `convolver` | `fx-fruity-convolver` |
| `FrequencyShifter` / `FrequencyShifterParams` | Frequency Shifter | `frequencyShifter` | `fx-frequency-shifter` |
| `PitchShift` / `PitchShiftParams` | Pitch Shift | `pitchShift` | `fx-pitch-shifter` |
| `PitchCorrect` / `PitchCorrectParams` | Pitch Correct | `pitchCorrect` | `fx-pitcher` |
| `Vocoder` / `VocoderParams` | Vocoder | `vocoder` | `fx-fruity-vocoder`, `fx-vocodex` |

## Controls and actual processing

- **Convolver:** mono IR applied independently to both stereo channels, using
  16 uniform 128-sample partitions, a prepared fixed 256-point radix-2 FFT and
  overlap-save history. `impulse: [f32; 2048]`, `impulseLength: u16` (0–2048),
  `mix` (0–1) and `gain` (0–4). **Length zero is unity**, with the same latency
  as a loaded IR; no built-in room replaces the empty impulse. Active IR
  samples are sanitized to ±8; inactive samples become zero. The fixed payload
  is edited atomically through params and is excluded from scalar automation
  descriptors. Serialization writes a 2048-element array; deserialization
  accepts a shorter array and zero-pads it, rejecting more than 2048 elements.
  The host must explicitly supply `impulseLength`; omitting it selects unity.
  IR updates compute all fixed FFT partitions in `set_params` without heap
  operations and crossfade two convolution results over 5 ms, beginning at the
  next internal partition. Repeated edits preserve the instantaneous blend.
  This bounded 8 KiB Copy payload is intentional for this assignment, not a
  general sample-asset API. IRs can implement FIR EQ, including linear-phase
  kernels; the kernel's intentional group delay is additional to transport.
- **Frequency Shifter:** `shiftHz` (−5000…5000) is additive translation, not a
  pitch ratio. A 255-tap Blackman-windowed Hilbert FIR makes an analytic signal;
  quadrature modulation shifts it up for positive Hz and down for negative Hz.
  `mix` blends with the equally delayed real input. Translation is additionally
  limited to ±0.45 × prepared sample rate. Finite FIR image rejection is weakest
  near DC and Nyquist. Content crossing Nyquist aliases; there is no oversampled
  anti-alias stage. Content crossing zero frequency folds as real audio.
- **Pitch Shift:** `semitones` (−24…24) maps to `2^(semitones/12)`; `mix` is
  0–1. Two linked stereo fractional-delay read heads use cubic interpolation
  and complementary raised-cosine windows over a fixed 2048-sample grain span.
  Unison uses the exact fixed dry delay, including after automation. Close to
  unison a narrow ratio range of ±0.0001 blends into that exact delay to avoid
  freezing an arbitrary grain offset. **No independent formant control or
  formant preservation:** formants move with pitch. This is a time-domain
  granular shifter, with possible grain modulation, transient smearing and
  high-frequency aliasing on large upward shifts; no voice/full-mix modes.
- **Pitch Correct:** `root` (C=0…B=11), `scale` (`chromatic`, `major`, `minor`,
  major `pentatonic`), `speedMs` (0–500), `amount` (0–1), `tuningHz` (400–480,
  default A4=440) and `mix`. Stereo average is lowpassed by two cascaded biquads
  and decimated by `D = max(1, round(sampleRate/6000))`. Normalized bounded
  autocorrelation uses a 256-sample ring, a 32-sample hop, mean removal,
  a 0.75 peak-confidence threshold and parabolic period refinement, targeting
  approximately 60–1000 Hz. The first credible local peak rejects longer period
  multiples. Unvoiced estimates target zero shift rather than retaining a note.
  Correction selects the closest permitted pitch class in equal temperament,
  applies `amount` and a per-sample exponential speed, and drives the stereo
  grain engine. Detector history is accommodated by a fixed extra audio delay.
  This is **monophonic**: stereo antiphase cancellation, noisy/breathy input,
  strong harmonics, polyphony and low notes may cause rejection or octave errors.
  It has no MIDI harmonies, voice generation, dedicated onset/voicing model,
  formant protection or offline neural tracker. The tracker and granular pitch
  engine are real bounded DSP, but do not complete the entire E5 Pitcher row.
- **Vocoder:** main input is the stereo carrier. `process_sidechain(left,
  right, Some(key))` supplies the stereo modulator, never adds key audio to the
  carrier, and treats missing frames in a short key slice as zero. `None` uses
  the input as its own modulator, so ordinary `process` produces audible output.
  `bandCount` (16–32, default 16) builds that actual number of logarithmically
  spaced bandpass filters from 80 Hz to min(12 kHz, 0.45 × sampleRate).
  Rectified, stereo-linked modulator envelopes multiply independent stereo
  carrier bands. Antiphase keys remain audible. `attackMs` (0.1–100),
  `releaseMs` (5–1000), `gain` (0–8), `mix` (0–1), and `color` (−1…1,
  envelope mapping offset by ±4 bands with interpolation) are live controls.
  Band-count changes crossfade two real filter banks over 5 ms. Edits during a
  fade coalesce until it completes, so an audible bank is never replaced.
  Reset installs the latest count immediately and clears all envelopes.
  Full-wet output is exact silence when the modulator envelope is zero; a
  modulator becoming silent retains the intentional release/filter tail.

For the shared vocoder engine, **16 bands / color 0** is the classic
`fx-fruity-vocoder` control view; **32 bands with exposed color** supplies the
higher-resolution `fx-vocodex` view. This is observable bank-count and envelope
mapping behavior. There is **no built-in carrier synth** (the carrier is always
the main input), so that part of the Vocodex summary remains missing. Neither
view establishes full proprietary acoustic equivalence or authorizes row closure.

## Latency, readiness and tails

Prepared sample rates are sanitized to 8000–384000 Hz; nonfinite rates become
48000 Hz. Timing and oscillator/tracker state advance per sample, independent
of the host's block length and `max_block`.

| Processor | `latency_samples()` | Warm-up / delay readiness | Conservative tail / silence gap |
| --- | --- | --- | --- |
| Convolver | 128 at every IR/mix setting | 128 | 2175 samples, retaining any fading old IR |
| Frequency Shifter | 127 (Hilbert group delay) | 254 | 254 samples |
| Pitch Shift | 1026 (fixed grain centre and exact dry/unison delay) | 2052 | 2052 samples |
| Pitch Correct | `256 × D + 1026` (3074 at 48 kHz) | `256 × D + 2052` (4100 at 48 kHz) | Same as readiness |
| Vocoder | 0 transport samples | 0 | `ceil(sampleRate × 32)` samples, covering a previous maximum release |

For shifted pitch, latency is the **fixed grain-centre reference**, not an exact
impulse peak or first nonzero sample. Pitch Shift wet reads span 2–2050 samples
of input age; Pitch Correct adds `256 × D` to those ages. Fractional interpolation
adds a small surrounding support. The pitch waveform is inherently dispersed
across those ages; PDC cannot collapse it into a pure delay. Dry and exact unison
use the reported centre. No runtime pitch parameter changes resize that window
or change reported latency. Pitch Correct's first estimate requires 256 tracker
samples, followed by estimates every 32 (42.67 ms window / 5.33 ms hop at 48 kHz);
the selected speed adds musical retuning time beyond detector acquisition.
The causal vocoder filters and attack/release change phase/envelope timing but
have no common transport delay.

## Realtime and import limits

Construction and `prepare` may allocate; `process`, `process_sidechain`,
`set_params`, `reset` and `set_tempo` never allocate, free, lock, wait or perform
IO. Convolver owns fixed-length vectors, and pitch effects own prepared delay
lines; none are resized or replaced by those realtime methods. Reset clears
storage in place. FFT scratch, tracker correlations and filter banks are fixed
arrays. All loops have fixed maxima. Ordinary continuous controls ramp over
5 ms; scales/root change at the next sample and correction follows `speedMs`.
The host should publish IR payloads as bounded edits rather than automate them
at audio rate: rebuilding 16 FFTs is bounded but appreciably more work than a
scalar change.

Nonfinite audio becomes zero, finite input is guarded to ±1000, and output is
guarded to ±1e6 with subnormal flushing. State uses finite guarded signals;
normal audio remains linear except for the specified envelopes and control
mapping. All effects ignore tempo. Dropping still frees owned memory and must
happen off the audio thread.

**IR file import is not included.** There is no file picker, decode, resampling,
trimming, arbitrary-length/stereo/true-stereo IR asset loader, or shipped
proprietary impulse content. The integration owner may decode/resample a file
off-thread and publish at most 2048 mono samples at the prepared sample rate.
Longer files must be explicitly rejected or deliberately trimmed by that host
workflow. At 48 kHz the IR capacity is about 42.67 ms, so this is not a long-hall
convolver. Sample-rate changes do not resample the stored payload.

## Focused verification

Tests live inside the spectral module: empty and explicit identity impulses,
2048-sample convolution against a direct FIR across every partition, signed
frequency zero-crossing measurements, semitone period changes, exact unison
delay, chromatic versus major scale correction, vocoder zero/nonzero/antiphase
key and no-key behavior, short-key semantics, damaged data finite output/reset,
serde/default/descriptor agreement, and partition invariance on all processors.
The focused standalone Rust harness reads the exact current `Effect` trait and
the existing parameter/balance/biquad/delay/smoothing/math implementations;
it runs only spectral tests and writes its executable outside the source tree.
All 13 module tests passed. An additional temporary harness test installed a
thread-local counting allocator around all five processors' live IR/band/pitch
edits, audio/sidechain processing, tempo changes and reset: zero allocations
and zero frees, with all 14 focused checks passing. The counting allocator is
kept out of the module to avoid conflicting with other families' test allocators.
No workspace test run, dependency changes, generated bindings, commits or PRs
are part of this assignment.
