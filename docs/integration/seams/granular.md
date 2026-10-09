# Slice and grain instrument integration seam

The parent must add `pub mod granular;` to `windfall-dsp/src/lib.rs`.
Public types then live in `windfall_dsp::granular`:
`SliceMap` / `SliceMapParams`, `SliceDeck` / `SliceDeckParams`,
`GrainCloud` / `GrainCloudParams`, `WaveRide` / `WaveRideParams`,
`SampleTable`, and `SliceSettings`. All four instruments implement
`Instrument` and `Default`. Suggested `InstrumentKind`, `InstrumentParams`,
and `AnyInstrument` variant names are `SliceMap`, `SliceDeck`,
`GrainCloud`, and `WaveRide`. Registry, bindings, editor, and parity
integration belong to the parent. No parity row is closed by this DSP seam.

- **Slice Map**: parity `inst-fruity-slicer`. Sixteen normalized ordered
  `sliceStarts`; MIDI keys select
  `(key - baseKey).rem_euclid(16)`, with default `baseKey = 60`.
  Each slice ends at the next start, and slice 16 ends at the table length.
  Boundaries are floored to frame indices, with an exclusive end. Empty slices
  are silent. A slice plays once at the source sample rate, with velocity,
  level and release. Keys select slices rather than transposing them.
  This is an instrument, separate from the playlist slicer.
- **Slice Deck**: parity `inst-fruity-slicer-2` and the **playable part**
  of `inst-slicex`. Same mapping, with sixteen `SliceSettings` values
  containing `pitch` (-48 to 48 semitones), `gain` (0 to 2), and
  `reverse`. Reverse reads from the last frame of the selected region;
  interpolation never reads another slice. Transient detection, a full editor,
  and a two-deck UI are not included.
- **Grain Cloud**: parity `inst-fruity-granulizer`. Independent note-local
  periodic schedulers launch Hann-windowed grains immediately at note-on and
  then at `density` (1 to 128 events/second). `grainSizeMs` is 1 to 250 ms,
  rounded to output frames with a minimum of three frames. `position` is
  normalized; `spray` (0 to 1) scatters starts uniformly by up to half its
  value in either direction, wrapping within the table. `pitch` is -48 to
  48 semitones and MIDI pitch is relative to `rootKey` (default 60).
  Grain positions come from a wrapping 32-bit LCG initialized with
  `seed XOR (key * 0x9e3779b9)`; identical parameters/key/seed reproduce
  positions on every note and after reset. Seed is an exactly representable
  24-bit integer control. `grains_spawned()` counts accepted events since
  prepare/reset; it does not allocate. Source playback wraps, grain windows
  reach zero at both ends, and gain divides by the square root of nominal
  overlap. Note-off stops scheduling and fades the remaining grains.
- **Wave Ride**: parity `inst-wave-traveller`. Eight normalized `positions`
  at equally spaced envelope times scan the table by piecewise linear
  interpolation. The default envelope rises from 0 to 1; drawn reversals and
  holds work. `durationMs` is 1 to 10,000 ms. MIDI pitch relative to
  `rootKey` and `pitch` scale envelope speed; one octave doubles scan
  speed. Reaching the end automatically retires the voice. This is direct
  curve-driven table playback, not independent time stretching.

## Source and parameter contract

Each parameter struct is `Copy`, with a fixed `SampleTable`:
`data: [f32; 4096]`, `len: u16`, and `sample_rate: f32`.
The **4096-sample mono limit** is intentional; longer samples require a future
immutable-asset contract. `SampleTable::from_slice` rejects longer input.
Unused table frames do not play. Defaults contain an empty, silent source.
Sanitization clamps `len` to 4096, finite samples to [-1, 1], replaces
nonfinite samples with zero, and clamps source/output rates to 1–384 kHz
(nonfinite rates become 48 kHz). Rates are independent for slice/grain
resampling. Wave Ride uses the output rate and envelope duration.

Serde uses camelCase with defaults for missing fields. Source serialization
writes the fixed 4096 entries; deserialization accepts shorter arrays with
zero padding and rejects longer arrays without growing storage.
`len` still declares the playable frame count. ts-rs types describe source
data as `number[]`; this module does not write bindings automatically.
Parent export code can export the derived types explicitly.

All continuous UI controls have stable `ParamSet` descriptors. Descriptor
order is: common controls, then sixteen starts for slice instruments, then
pitch/gain/reverse for each Slice Deck slice, or eight Wave Ride positions.
Common order:

- Slice Map/Deck: `baseKey`, `level`, `releaseMs`.
- Grain Cloud: `rootKey`, `grainSizeMs`, `density`, `position`,
  `spray`, `pitch`, `seed`, `level`, `releaseMs`.
- Wave Ride: `rootKey`, `durationMs`, `pitch`, `level`, `releaseMs`.

Starts sanitize monotonically without reordering slice identities.
Boundaries are captured at note-on, so marker edits affect subsequent notes.
Source payloads are discrete data, excluded from automation descriptors and
`ParamSet::approach`; `set_params` replaces the table immediately using
fixed copies. Active grains wrap safely if the replacement source is shorter.
Continuous controls glide on a persistent 16-output-frame clock with 10 ms
smoothing; toggles and integers jump. Fresh controls after prepare/reset apply
immediately. Slice pitch/gain/reverse and Wave Ride positions affect sounding
voices; grain size, position and spray are captured when a grain launches.

## Realtime and playback bounds

Construction, prepare and destruction belong off the audio thread. All state
is fixed storage: **16 voices**, each with at most **32 grain slots**.
Overflow steals the oldest rendered voice (ties choose the lowest slot).
Excess grain events at the slot limit drop deterministically, never queue.
Every callback, including reset, parameter and note calls, performs no
allocation, freeing, locking, IO or waits. No callback scratch buffers depend
on the host block size.

Source interpolation is linear, with no bandlimited resampler or promise of
alias-free extreme transposition. Effective pitch ratios are bounded to
eight octaves in either direction. A 1 ms attack and natural-end fade reduce
slice/scan edge clicks. `releaseMs` is 1–1000 ms; per-note release expression
multiplies it by 1/8–8. `all_notes_off` uses a fade of at most 4 ms. Retired
voices output exact zero. Latency and post-voice tails are zero.
Stereo uses equal-power note pan and 0.25 fixed headroom before summed level;
output is finite but is not a brickwall limiter.

Instance-specific note-on/off, pan, fine pitch, release expression, and
effective pitch updates are supported. Instance release preserves other
occurrences of the same key. Modulation X/Y, articulation and tempo are not
specialized here; pitch updates take effect directly. Source replacement,
reverse toggles and voice stealing may cause discontinuities. This slice
does not claim transient analysis, editor/persistence integration, factory
content, tempo-synchronized slicing, or full reference-product equivalence.

## Focused verification

The permanent module tests cover distinct ramp slices, boundaries, reverse,
per-slice pitch/gain, real grain event counts, matching/different seeds,
curve reversals, bounded polyphony, finite hostile input/source replacement,
exact release silence, note-instance isolation, parameter defaults/serde/
descriptors, table capacity, and exact block partition invariance including
automation and note events.

Verification on 2026-10-08 used a temporary rustc test harness containing an
exact copy of the current `Instrument` trait, the actual `param.rs`,
`note_expression.rs` and processing blocks, this module, and matching
existing dependency artifacts. **13 focused tests passed**, including an
allocator guard covering all four instruments' callbacks with zero allocations
and frees. The harness, copied trait and build artifacts were removed.
Only owned Rust files were formatted; no workspace suite was run.
After parent wiring, run `cargo test -p windfall-dsp --lib granular`.
