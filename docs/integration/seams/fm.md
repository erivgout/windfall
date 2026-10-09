# Seam: FM instruments

The owned `windfall_dsp::fm` module adds three polyphonic instruments
through the existing `Instrument` and `ParamSet` contracts. Nothing outside
`crates/windfall-dsp/src/fm/` and this file was touched, so the module is
complete but not yet reachable from a project: the parent still has to add
the `InstrumentKind` variants listed below.

No parity status is changed here.

## Files created

| File | Contents |
| --- | --- |
| `crates/windfall-dsp/src/fm/mod.rs` | Module docs, re-exports, `MAX_POLYPHONY` |
| `crates/windfall-dsp/src/fm/params.rs` | Every parameter struct and its `param_set!` table |
| `crates/windfall-dsp/src/fm/common.rs` | `Controls`, `Envelope`, `Voice`, `Bank`, output guard |
| `crates/windfall-dsp/src/fm/four_op.rs` | `FourOp` |
| `crates/windfall-dsp/src/fm/matrix.rs` | `MatrixFm` |
| `crates/windfall-dsp/src/fm/ring.rs` | `RingHybrid` |
| `crates/windfall-dsp/src/fm/tests.rs` | Measurements for all three |

`common.rs` and `params.rs` are private to the module; only the types below
leave it.

## Public types

Instruments, each implementing `Instrument`:

- `fm::FourOp`
- `fm::MatrixFm`
- `fm::RingHybrid`

Parameter structs, each deriving `Serialize`, `Deserialize`, `TS`, `Default`
and `ParamSet` the way `balance.rs` does, with `#[serde(rename_all =
"camelCase", default)]` and `#[ts(export)]`:

- `fm::FourOpParams` — `ParamSet::NAME` is `"Four Operator"`, 27 controls
- `fm::MatrixFmParams` — `"Matrix FM"`, 91 controls
- `fm::RingHybridParams` — `"Ring Hybrid"`, 21 controls

Supporting parameter types, also exported to TypeScript:

- `fm::FmEnvelopeParams` — linear ADSR, shared by all three
- `fm::FmOperatorParams` — ratio, level, envelope (`FourOp`)
- `fm::MatrixOperatorParams` — adds feedback, key scaling, carrier send
- `fm::HybridOscillatorParams` — waveform, ratio, level (`RingHybrid`)
- `fm::FourOpAlgorithm` — `Stack`, `Pairs`, `FanIn`, `FanOut`, `Branch`,
  `Merge`, `Triple`, `Parallel`
- `fm::HybridWaveform` — `Sine`, `Triangle`, `Saw`, `Square`

Constant:

- `fm::MAX_POLYPHONY` — 16, the fixed voice count of all three

## Parity ids

| Instrument | Parity id | FL feature |
| --- | --- | --- |
| `FourOp` | `inst-fruity-dx10` | Fruity DX10 |
| `MatrixFm` | `inst-sytrus` | Sytrus |
| `RingHybrid` | `inst-toxic-biohazard` | Toxic Biohazard |

These ids are still `status: todo` in `docs/parity/parity.json`, and
`windfall_name` is still null for all three. Setting them is the parent's
call, not this module's, and it should not happen before the instruments are
reachable from a project.

The three instruments are Windfall designs that cover the same ground as the
named FL plugins. No proprietary code, presets or measured fixtures are
used, and none of the three claims identical DSP.

## What makes each one distinct

`MatrixFm` is not a `FourOp` preset and `RingHybrid` is not a pure FM patch.
The differences are structural, not cosmetic:

**`FourOp`** routes four sine operators through one of eight **fixed acyclic
graphs** chosen by `algorithm`. Modulation is same-sample: operators are
evaluated from 4 down to 1, so a modulator's current output reaches its
carrier with no delay. Only operator 4 has feedback, taken from the mean of
its own last two outputs. Changing `algorithm` crossfades the twenty graph
weights over 5 ms rather than switching abruptly.

**`MatrixFm`** has six operators and a **full signed 6x6 matrix**:
`matrix[destination][source]` in radians per unit source output, with every
cell free, including the diagonal and both directions of any pair. Cycles
are therefore legal. To make them deterministic without an implicit solver,
**every matrix route reads the previous sample**, which is the one real
behavioural cost of the full matrix (see limits). Each operator also has its
own `output` send, so the carrier set is a continuous mix rather than a
property of a chosen graph, its own `feedback`, and `key_scaling` in gain
octaves per keyboard octave centred on A4.

**`RingHybrid`** is deliberately not pure FM. Three oscillators with
selectable waveforms feed three FM routes (`fm_21`, `fm_31`, `fm_32`), and
the result then passes two **ring-modulation crossfades** (`ring_12`,
`ring_13`) that blend a signal into its own product with another oscillator,
and then a **resonant state-variable lowpass** with a post-filter amp
envelope. Filter state belongs to each voice. Sum and difference tones from
the ring path and the subtractive lowpass are both audible in the spectrum
test, so the timbre is reachable neither by `FourOp` nor by `MatrixFm`.

## Suggested `InstrumentKind` variant names

For the parent to add, matching the existing `SubtractiveSynth` style:

| Variant | Params | Instrument |
| --- | --- | --- |
| `InstrumentKind::FourOp` | `FourOpParams` | `FourOp` |
| `InstrumentKind::MatrixFm` | `MatrixFmParams` | `MatrixFm` |
| `InstrumentKind::RingHybrid` | `RingHybridParams` | `RingHybrid` |

The camelCase serde tags these imply are `fourOp`, `matrixFm` and
`ringHybrid`.

## Parent wiring deliberately not done

All of this was in reach and was left alone on purpose, because the files
involved are not owned here:

- `instrument.rs`: no `InstrumentKind` variants were added, so
  `InstrumentKind::ALL` is still `[SubtractiveSynth]` and its length is
  still 1. `InstrumentParams`, `AnyInstrument` and every `match` in that
  file are untouched, as are `name`, `descriptors`, `default_params`,
  `sanitized`, `latency_samples`, `get` and `set`.
- `lib.rs`: `pub mod fm;` was already declared and is unchanged. No `pub
  use fm::{...}` re-export was added at the crate root, so callers reach the
  types as `windfall_dsp::fm::FourOp` and so on.
- `docs/parity/parity.json`: no `status` or `windfall_name` change for the
  three ids above.
- `docs/FEATURE-PASS.md`: no entry.
- `crates/windfall-dsp/tests/dsp/`: no integration tests were added, so the
  crate-wide sweeps in `params.rs`, `realtime.rs`, `properties.rs` and
  `examples.rs` do not yet cover these three. The equivalent checks are
  implemented inside `fm/tests.rs` instead, including the descriptor,
  serde-path, sanitizer and extreme-input contracts those sweeps apply.
- TypeScript bindings: `#[ts(export)]` is derived, but no generated binding
  was committed and `scripts/check-bindings.mjs` was not run. Running the
  module's tests writes the bindings into `TS_RS_EXPORT_DIR`.
- Engine, project, and UI: nothing. No channel, preset, or control surface
  knows these instruments exist.

Until the `InstrumentKind` variants land, the only way to reach these
instruments is to construct them directly and drive the `Instrument` trait.

## Realtime behaviour

`process`, `set_params`, `reset`, `note_on`, `note_off` and `all_notes_off`
allocate nothing, take no lock, and do no IO. There is nothing to allocate:
each instrument is one flat struct holding a `Controls<P>` (two copies of a
`Copy` parameter struct), a `Bank<N>` that is a fixed array of
`MAX_POLYPHONY` voices, and a few ramps. No instrument holds a `Vec`, a
`Box`, or any handle, so `prepare` only stores the sample rate, sizes the
glide, and resets. There is no wavetable: operators call `blocks::oscillator`
directly, which computes `sin` per sample.

`eight_voices_run_from_the_preallocated_bank` asserts that the bank is still
a flat array of voices, that eight simultaneous notes render from it, and
that more notes than there are slots take voices over instead of adding
them.

Parameter changes glide. The first `set_params` after `prepare` or `reset`
applies at once; later ones interpolate linearly to the new value over
exactly 5 ms (240 samples at 48 kHz) and land on it exactly, which
`controls_snap_initial_settings_and_finish_a_glide_in_five_ms` measures at
both the halfway point and the end. Choices and the eight `FourOp` graphs
crossfade over the same 5 ms. `all_notes_off` fades over 5 ms instead of
running the releases.

Output is sanitised twice: every parameter goes through `ParamSet::sanitized`
on the way in, so a damaged project file cannot misbehave, and every output
sample goes through a finite guard. `latency_samples` and `tail_samples` are
both 0: there is no delay line anywhere, and a released voice reaches exact
zero rather than an asymptote, after which the instrument emits exact
silence.

Output is mono, written identically to both channels. None of the three has
a pan control; placing an instrument in the stereo field is the channel's
`Balance` job.

## Numerical limits, honestly

These are the places where the measurements are looser than they look, or
where behaviour is a deliberate compromise.

**Matrix routes cost one sample.** Every route in `MatrixFm`, including the
diagonal and including routes that could have been evaluated in order, reads
the previous sample. At 48 kHz that is a 7.5 microsecond delay per hop,
which is a phase shift that grows with frequency: a 10 kHz modulator arrives
75 degrees late. The spectrum is therefore not identical to a same-sample
evaluation of the same matrix, and `FourOp`'s same-sample chain and a
`MatrixFm` matrix set to the same graph will not produce bit-identical
audio. This is the price of allowing cycles without an iterative solver, and
it is why the two instruments are separate rather than one engine.

**Operator self-feedback is averaged, not exact.** `feedback` drives an
operator from the mean of its own last two outputs. The mean is there to
stop the one-sample loop oscillating at Nyquist; it also means the feedback
path is gently lowpassed, so high feedback is darker than an ideal
single-sample loop would be.

**Above Nyquist, operators alias.** Phase increments are clamped at 0.45
cycles per sample in `FourOp` and `MatrixFm` and at 0.24 in `RingHybrid`,
which keeps the phase accumulator sane but does not band-limit anything. A
ratio of 32 on a high key puts operators far above Nyquist, where they fold
down. This matches how hardware FM behaves and is not corrected.
`RingHybrid`'s non-sine waveforms are the band-limited ones from
`blocks::oscillator`, but their ring products and FM sidebands are not.

**Sideband placement is exact; the floor is f32 jitter.**
`four_op_sidebands_sit_on_the_carrier_modulator_grid` drives one operator
pair at whole ratios on key 69 (440 Hz) and measures the first 21 harmonics
of 440 over 4800 samples, which is exactly 44 cycles, so every harmonic sits
on a bin centre and leaks nothing. Energy off the `|carrier +- k *
modulator|` grid is required to be below 1e-5 of the loudest component.
The worst measured value is 1.44e-6, about -117 dB, so the threshold keeps
roughly 7x of headroom. That floor is not a property of FM; it is the f32
phase accumulator's rounding, which is the only thing putting energy there
at all, and tightening the threshold past 1e-6 fails on arithmetic rather
than on anything audible. The three
ratio pairs measured (8:3, 9:4, 6:5) are chosen so that no two sidebands
land on the same harmonic, because where they do they can partially cancel
and a "this sideband is present" check would be reading cancellation rather
than placement. That constraint is `2 * carrier % modulator != 0`, asserted
in the test.

**The spacing test's floor is leakage, not precision.**
`four_op_sideband_spacing_follows_the_modulator` probes a quarter of a
sideband spacing away from the carrier, which is not a bin centre, so what
it measures is the skirt of the real components. Its 10x margin is
deliberately loose for that reason; the grid test above is the tight one.

**Spectral difference tests are differential.** The tests that separate the
eight `FourOp` algorithms, the matrix routes, and `RingHybrid`'s FM, ring
and filter stages compare normalised 16-bin spectra and assert a minimum L1
distance. They prove the stages do something distinct and audible. They do
not pin the spectrum to an authored reference, so they would not catch a
change that altered all of the spectra together.

**Block-size invariance is exact.** `partition_contract` renders 2048
samples whole, and again in a repeating `1, 17, 3, 64, 7` pattern of block
sizes, with parameter changes, a second note and a note-off landing at
sample 197, 701 and 913. The two results are compared with `assert_eq!`, so
they must be bit-identical, not merely close. Everything that moves is
advanced per sample rather than per block, which is what makes that hold.

**Envelopes are linear, and release is linear from wherever it was.** The
module uses its own `Envelope` rather than `blocks::adsr::Adsr`. Segments
are straight lines in amplitude, not the exponential approaches `Adsr`
uses, so these instruments have a different envelope character from
`SubtractiveSynth` by construction. Release scales down from the level the
note was let go at and reaches exact zero, which is what lets
`tail_samples` be 0. Changing a segment time does not restart the segment.

**Voice stealing is abrupt.** `Bank::note_on` takes the oldest active voice,
preferring any free slot first, and resets its phases and envelopes. The
stolen voice is cut rather than faded, so stealing under heavy polyphony can
click. `all_notes_off` fades, and ordinary releases are smooth; only
stealing is abrupt. Nothing in the module's tests asserts click-free
stealing, and fixing it would mean the spare-voice-plus-choke machinery
`SubtractiveSynth` uses.

**`MatrixFm`'s release is per-operator, which is a weak distinction.** Each
of the six operators releases on its own time, so the timbre morphs as a
voice dies. `FourOp` does the same with four operators, so this is a
difference of degree rather than a separate release mechanism. If the parent
wants `MatrixFm` to have a genuinely distinct release, a release-mode
control (per-operator, shortest-wins, or freeze-the-timbre-and-fade) would
be the way, and it is not implemented here.

**Key scaling is bounded.** `MatrixOperatorParams::key_scaling` is gain
octaves per keyboard octave around A4, clamped to 4x so a large setting on a
high key cannot run away.
`matrix_key_scaling_changes_level_by_one_octave_per_octave` checks the
octave-per-octave case to within 1%.

## Running the measurements

```sh
TS_RS_EXPORT_DIR=$(mktemp -d) cargo test -p windfall-dsp --lib fm -- --test-threads=8
```

On Windows, `source scripts/msvc-env.sh` first. `TS_RS_EXPORT_DIR` keeps the
derived TypeScript out of the tree, since no binding is committed yet.
