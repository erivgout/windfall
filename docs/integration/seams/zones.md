# Zone instrument integration seam

Implemented only in `crates/windfall-dsp/src/zones/`. The integration owner
must add `pub mod zones;` to `lib.rs` and wire the instrument unions/registry.
Neither those files nor parity accounting were edited here. This is the
bounded playback portion of roadmap section 6, family I5.

Public instrument / parameter types and intended `InstrumentKind`,
`InstrumentParams` and `AnyInstrument` variant names:

- `ZoneSampler` / `ZoneSamplerParams`, variant `ZoneSampler`, **Zone Sampler**.
  Editable project-resident zones; partial parity `inst-directwave-player`
  as a player of zones already in the project. A full sample editor and disk
  streaming are not included.
- `ZonePlayer` / `ZonePlayerParams`, variant `ZonePlayer`, **Zone Player**.
  Same playback engine with `zones_locked` forced true. `new(&params)` installs
  the bank; subsequent `set_params` calls preserve it, even if the incoming
  flag is false. Construct a new player off audio to install another bank.
  The editable instrument is Zone Sampler; Zone Player is its locked mode.
  **No `inst-directwave-full` parity is claimed.**
- `PadSampler` / `PadSamplerParams`, variant `PadSampler`, **Pad Sampler**.
  The first sixteen zone slots correspond exactly to keys **36..51**, one
  zone per pad. Sanitization forces each slot's key range and root to its pad
  key; source sample rate, velocity range, gain, pan and tuning still apply.
  Slots after sixteen do not play. There is no modulo wrapping, and empty
  pads are silent. Partial parity `inst-fruity-pad-controller-fpc` covers the
  pad layout only. This is not a hardware controller. Layered pads, choke
  groups and round robin are not included.
- `KeyBed` / `KeyBedParams`, variant `KeyBed`, **Key Bed**.
  Only zone zero plays, with its key range forced to **0..127**. It transposes
  relative to that zone's root and enters release on note-off. Partial parity
  `inst-fl-keys` covers a single-sample keyboard only. A multi-velocity piano
  library is not included. A source can have a loop for sustained notes;
  an unlooped source naturally ends even while its key is held.

All four implement the actual `Instrument` trait, support note instances,
and provide `Default`, `new(&Params)`, and `params() -> &Params`. Their
`ParamSet::NAME` values are the display names above. Registration, bindings,
editor UI, project import and host asset lifecycle integration remain with
the integration owner. No parity row is closed by this seam.

## Fixed storage and parameters

Public data types: `Sample`, `Zone`, `ZoneTable`, `ZoneTableError`,
`CrossfadeAxis`, and `LoopMode`. All parameter/data types are `Copy` and
derive serde and ts-rs traits. Serde names are camelCase with defaults for
missing fields. No new crate dependency or FluidSynth linkage was added.

- `ZoneTable`: `zones: [Zone; 32]`, `len: u8`; at most **32 zones**.
- `Sample`: `data: [f32; 1024]`, `len: u16`, `sample_rate: f32`;
  at most **1024 mono frames per zone**. Only frames below `len` play.
- `Zone`: sample, inclusive `key_low`/`key_high`, inclusive normalized
  `velocity_low`/`velocity_high`, `root_key`, `gain`, `pan`, `tune_cents`,
  `loop_mode`, `loop_start` and exclusive `loop_end`.
- Parameters: `zones`, `level`, `release_ms`, `crossfade`, `zones_locked`.
  Descriptor order is **level, releaseMs, crossfade**. Level is 0..2,
  default 0.5; release is 1..10000 ms, default 100; crossfade is
  velocity (0, default) or key (1). Zone payloads and the locking flag are
  discrete project edits, excluded from automation descriptors.

`Sample::from_slice` and `ZoneTable::from_slice` reject oversize input with
`ZoneTableError`; they never silently truncate. Sanitization of manually
constructed parameters caps declared lengths, cleans all fixed sample
frames to [-1, 1] (nonfinite becomes zero), clamps source/output sample
rates to 1000..384000 Hz (nonfinite becomes 48000), keys to 0..127,
velocities to 0..1, gain to 0..4, pan to -1..1, and tuning to
-12000..12000 cents. Reversed ranges remain empty. Invalid loop endpoints
disable looping. Default tables are empty and produce exact silence.

Sample JSON writes all 1024 frames. Short sample arrays deserialize with
zero padding; oversized ones error. A supplied `ZoneTable.zones` JSON array
must contain exactly 32 entries, including empty unused slots; omit the
field for its default. ts-rs represents sample frames as `number[]`.
Deriving these types does not automatically write binding files.

Zone edits are applied to future notes. Sounding layer voices retain fixed
copies of their source and zone settings, so replacing/removing a zone
does not invalidate playback or free assets in a callback. Global level
and release controls glide with a 10 ms per-sample smoother; changes before
the first rendered frame after prepare/reset apply immediately. Release
duration is captured on note-off. `ParamSet::approach` interpolates global
controls and copies discrete control flags, leaving the table alone.

`zones_locked` defaults false for sampler/pads/keyboard and true for player.
The current flag prevents replacement in `set_params`. A sampler can clear
the flag, then replace its zones on the following call. Player sanitization
always forces it true; `ZonePlayer::default()` is consequently an empty
locked player. Use `ZonePlayer::new` with populated parameters to load it.

## Selection, crossfades and playback

Only zones matching both key and velocity and containing frames participate.
Missing keys, empty samples and velocities outside every range are silent.
Keys above 127 are ignored. Velocity clamps at 1; nonfinite or nonpositive
velocity triggers release rather than a new voice.

For the chosen crossfade axis, each matching zone contributes a triangular
weight `max(0, 1 - abs(2 * (x - low) / (high - low) - 1))`.
An axis range of zero width contributes weight 1. Weights are normalized
over matching zones; if all are zero at coincident edges, they share equal
weights. One matching zone plays at full weight. Coincident ranges blend
equally, and differently centered overlapping ranges crossfade as the key
or velocity moves. Final gain also includes note velocity, zone gain and
global level. This is normalized crossfading, rather than SF2's unrestricted
additive layering; SoundFont parity remains partial.

Playback uses linear interpolation at
`source_rate / output_rate * 2^((pitch - root + tuning_cents/100)/12)`.
Combined musical pitch is clamped to +/-96 semitones before rate conversion.
This resampler does not promise alias-free extreme transposition. The last
frame is held for interpolation to the exclusive sample end, then the voice
retires. Loop interpolation wraps its next frame to the loop start.
`LoopMode::Continuous` loops through release; `UntilRelease` exits to the
sample remainder on note-off; `Off` is one-shot. Note-off applies a linear
fade. Repeated note-offs never extend a release; `all_notes_off` applies a
5 ms fade or retains a faster release already in progress.

Pan uses constant-power stereo gains. Per-note pan adds to zone pan and
clamps to -1..1. Instance expression supports fine pitch and the common
release multiplier; modulation/filter and articulation/glide behavior are
not synthesized. Instance pitch/pan changes apply immediately. Instance
release affects all that instance's layers, while legacy key release affects
all matching voices. Retriggering the same instance replaces its layers.
Legacy generated IDs count down from `u64::MAX`; hosts should use their own
disjoint identity range when mixing legacy and explicit instance calls.

## Realtime bounds

There are **32 layer voices**, not 32 independent layered notes. Each has a
fixed sample snapshot. An overlapping note can consume all 32 slots.
Overflow steals the oldest started layer, breaking ties by the first slot;
stealing can be abrupt. Selection uses at most 32 fixed weights, with no
dynamic staging or deferred notes. Source retirement is clearing voice state.

`prepare` allocates the bounded voice pool. Construction, prepare, parser
calls, serialization and destruction belong off audio. Process, parameter,
reset, tempo, expression, pitch and note callbacks allocate/free nothing,
perform no IO and acquire no locks. Playback before prepare is silent.
Block size does not change playback/smoothing; every process call overwrites
output, including silence. Latency and post-voice tail are zero.

The Copy table is approximately 132 KiB. Avoid putting many parameter
copies on a small thread stack. Off-audio serde round trips and test
utilities need extra stack space (the focused harness used **16 MiB**
test-thread stacks). Callback allocation tests pass on the normal test
stack; the voice pool itself lives on the heap after prepare.

## Minimal SoundFont 2 import

Public pure functions:

```rust
parse_soundfont(bytes: &[u8]) -> Result<ZoneTable, SoundFontError>
parse_soundfont_preset(bytes: &[u8], bank: u16, preset: u16)
    -> Result<ZoneTable, SoundFontError>
```

The first imports the first preset in file order; the second selects a
bank/program pair. They parse RIFF `sfbk` structure and mandatory INFO,
sample and hydra tables. Chunk lengths, padding, record widths, monotonic
bag indices, terminal indices, references and sample bounds are checked.
Truncated input fails, including files cut anywhere before their declared end.
Structural validation includes unselected presets. The format reference is
the [SoundFont technical specification](https://www.synthfont.com/sfspec24.pdf).

The supported subset is mono 16-bit PCM, key/velocity ranges, root override,
header pitch correction, pan, attenuation, coarse/fine tuning and forward
loop modes. Preset/instrument links are resolved; global values are
overridden by local values, preset offsets add to instrument values, and
key/velocity ranges intersect. Default 100-cent key scaling is accepted.
There is no bank browser or full SF2 synthesis model.

Unsupported generators, explicit modulators, stereo/ROM sample types,
24-bit sample chunks, rates outside storage bounds and unrepresentable
tuning return typed errors. Supported generator numbers are 17, 43, 44,
48, 51, 52, plus preset link 41, or instrument sample link 53, mode 54,
default scale tuning 56 and root override 58. Filters, envelopes beyond
the module's release, LFOs, address-offset generators, effects and exclusive
classes are not imported. Unsupported content is not presented as faithful
playback.

`TooManyZones` rejects a selected preset expanding beyond **32 zones**.
`TooManyFrames { sample, frames }` rejects a referenced sample longer than
**1024 frames**. Neither error returns a partial/truncated table. These
caps intentionally prevent loading most full-size commercial/GM banks.
Parity `inst-soundfont-player` stays **partial**.

## Verification

`tests.rs` covers range boundaries, velocity silence, independent pads,
keyboard release, sample-rate/pitch interpolation, both crossfade axes,
coincident edges, locked/editable behavior, source retirement, bounded
polyphony, instance release/pan, loops, block invariance, hostile floats,
Copy/serde/descriptors, SF2 playback, bank selection, every truncated fixture
prefix, bad indices/sizes, unsupported data, global/local generators and caps.
`verify.rs` is a standalone harness against the compiled library's actual
traits. Its counting allocator tests all four instruments and validates that
the counter detects both allocations and frees. No copied trait or stub
engine is used.

Before registry wiring, use an existing built `windfall-dsp` rlib and its
dependency directory (or build it with `cargo build -p windfall-dsp`). In a
configured MSVC/Windows SDK development shell:

```powershell
$deps = Join-Path (Get-Location) 'target/debug/deps'
$dsp = (Get-ChildItem "$deps/libwindfall_dsp-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$serde = (Get-ChildItem "$deps/libserde-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$ts = (Get-ChildItem "$deps/libts_rs-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
$json = (Get-ChildItem "$deps/libserde_json-*.rlib" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
rustc --test --edition 2024 -C opt-level=1 -D warnings -D unsafe_op_in_unsafe_fn crates/windfall-dsp/src/zones/verify.rs --extern "windfall_dsp=$dsp" --extern "serde=$serde" --extern "ts_rs=$ts" --extern "serde_json=$json" -L "dependency=$deps" -o "$env:TEMP/windfall-zones-verify.exe"
$env:RUST_MIN_STACK = '16777216'
& "$env:TEMP/windfall-zones-verify.exe" --test-threads=1
```

Observed: **24 tests passed**, including zero callback allocations/frees,
with arithmetic overflow checks enabled. Owned Rust files pass rustfmt and
the standalone harness passes Clippy with all warnings denied. No workspace
suite, registry edit, new dependency, commit, push, GitHub action or PR was
performed. After integration, run `cargo test -p windfall-dsp --lib zones`
with the same test stack setting; keep the standalone allocator check too.
