# Buffered performance DSP seam

`crates/windfall-dsp/src/performance/mod.rs` is complete within this seam. The
parent registers `pub mod performance;` in the DSP crate and wires the effect
unions, latency bounds, persistence, bindings and editors. This implementation
does not modify those registries or `parity.json`.

## Finished public types and integration names

- `VolumeGate`, `VolumeGateParams`: EffectKind **VolumeGate**, display name
  **Volume Gate**, parity reference `fx-gross-beat` (volume/retrigger subset).
- `TimeTransport`, `TimeTransportParams`, `TransportMode`: EffectKind
  **TimeTransport**, display name **Time Transport**, parity reference
  `fx-transporter` (explicit trigger/captured playback subset).
- `Scratch`, `ScratchParams`: EffectKind **Scratch**, display name **Scratch**,
  parity reference `fx-fruity-scratcher` (live-ring position playback subset).
- `PerformanceRack`, `PerformanceRackParams`, `PerformanceModel`: EffectKind
  **PerformanceRack**, display name **Performance Rack**, parity reference
  `fx-effector-12-fx` (twelve distinct audio treatments).

These EffectKind names are requested integration additions, not enum variants
already installed by this module. Each processor implements the existing
`Effect` trait, and has `new()` and `Default`. All parameter structs are `Copy`,
implement `ParamSet`, sanitize every numeric control and derive serde and
`ts_rs::TS`. JSON and enum values use camelCase, with default filling for missing
fields. TS derives deliberately do not use automatic export tests: binding
generation belongs to the registry owner. `PerformanceModel::ALL` lists twelve
models in stable descriptor order.

## Storage and callback contract

Each prepared processor owns exactly one stereo ring containing
`floor(2 * prepared_sample_rate)` frames, eight bytes per frame, plus constant
scalar/filter/control state. That is 768,000 ring bytes at 48 kHz and 6,144,000
at the maximum prepared rate of 384 kHz. Rates are finite and clamped to
1..=384,000 Hz, with 48 kHz fallback. No block-sized scratch storage exists;
`max_block` does not change memory use. Construction has an empty ring;
`prepare` allocates/replaces the ring. Unprepared processing is safe.

`process`, `set_params`, `set_tempo`, and `reset` contain no allocations,
deallocations, locks, IO or waits. Reset clears existing storage in place. Ring
addresses, feedback writes and both linear-interpolation neighbors stay inside
the ring; fractional neighbors also wrap inside the selected captured slice.
Inputs/history/output are finite, with invalid audio replaced by zero and a
numerical guard at +/-1,000,000. Feedback is clamped to 0..=0.95.

Controls glide per frame with a 5 ms one-pole approach and snap at its endpoint.
Controls set before the first frame after prepare/reset apply immediately.
Trigger, freeze and mode are discrete. Timing uses target loop beats immediately.
Deliberate read-head jumps, trigger/release, mode switches, transport wraps and
rack replay wraps use a 2 ms transition from the last wet sample. The programmed
gate steps remain literal gains; a zero step at fully wet mix silences the entire
step. Designed gating edges are not softened into nonzero gain.

## Timing, triggering and latency

Tempo is finite/clamped to 20..=400 BPM, default/fallback 120 BPM. Loops use
quarter-note beats and rounded integer frames, capped by available ring space.
Clocks advance per sample, never per block. Phase starts at zero after reset;
the interface supplies tempo, not song PPQ or bar position.

- Volume Gate divides its loop into sixteen steps. Each visited step boundary
  takes one deterministic seeded probability draw. A successful retrigger
  jumps back `retriggerSteps * loop_frames / 16` frames, clamped to recorded
  history, then advances with the incoming recording until the next step.
  Chance zero passes current input through the volume curve. The PRNG resets
  to a fixed seed for reproducible offline/live rendering. Feedback writes the
  preceding wet frame into the incoming recording, bounded by the ring.
- Time Transport records/passes live audio while trigger is false. The next
  rising edge captures the preceding loop (or all available history during
  startup), then stops writing so the slice remains immutable. Repeat starts
  at its oldest sample; Reverse and Hold start at its newest. Hold sustains
  that sample; Reverse traverses backward; Repeat traverses forward. Rate
  controls signed traversal speed, and repeated edges require an intervening
  false frame. Releasing trigger resumes recording. Tempo/length changes
  affect the next capture, preserving the current captured slice.
- Scratch records live input and addresses the most recent loop: position 0
  selects its oldest sample, 1 its newest, and intermediate values interpolate.
  Freeze captures the preceding available loop and stops writes; subsequent
  position automation traverses this fixed record in either direction.
  Tempo/length changes apply to live playback and the next freeze capture;
  a frozen record retains its bounds.
- Performance Rack captures the preceding loop at each loop boundary and
  records new input into a disjoint region of the same ring. Slice length is
  capped at **one second**, leaving room for simultaneous capture/playback
  within the two-second ring. The internal dry mix reads the same captured
  history at its normal forward rate. Feedback recirculates the preceding wet
  frame into the recording region, so both history paths include that feedback.
  Tempo/loop edits restart its local slice clock and recapture within bounds.

`latency_samples()` is zero for Volume Gate, Time Transport and Scratch:
their variable history lookbacks are intentional playback effects rather than
a fixed transport delay to compensate. Time Transport and Scratch report one
loop of warmup. Performance Rack reports **one loop of latency** (maximum one
second); its internal dry mix is aligned with that latency. The parent should
reserve `floor(sanitized_sample_rate)` frames for its maximum latency and
re-read latency after tempo/loop edits.

Frozen Scratch and triggered Time Transport can sustain forever, reporting
`usize::MAX` tail/gap. Rack feedback may sustain indefinitely through nonlinear
treatments and also reports `usize::MAX` tail/gap when feedback is nonzero.
Otherwise tails/gaps include buffered playback and filter decay; Volume Gate
reports a conservative bounded feedback-decay tail.

## Twelve rack transfers

1. Stutter repeats the first short subdivision of the captured loop;
   amount selects 2..=16 subdivisions.
2. Reverse reads the entire captured loop backward.
3. Tape Stop integrates a decreasing playback velocity, from normal speed
   toward `1 - amount` by the end of the loop.
4. Half Speed reads at 0.5x.
5. Double Speed reads at 2x, wrapping inside the capture.
6. Beat Repeat repeats the last quarter-note subdivision (0.25 beats),
   distinct from the stutter's initial short slice.
7. Telephone Filter applies one-pole highpass at 300 Hz then lowpass at
   3000 Hz, with cutoff caps for low sample rates and amount as filtered blend.
8. Distortion applies normalized tanh saturation with 1..=20 drive and
   amount as the saturation blend.
9. Pan Spin applies a sinusoidal left/right gain orbit per loop, with amount
   setting its depth and square-root gains.
10. Fade reduces gain linearly across the loop, with amount setting depth.
11. Silence Gate attenuates the second half of the loop, reaching exact
    silence at amount 1.
12. Pitch Jump resamples at `2^(pitchSemitones * amount / 12)`; it changes
    duration along with pitch and is not a duration-preserving pitch shifter.

The fixed-speed treatments are selected directly; amount is meaningful only
where described. Mix is the wet blend for every treatment.

## Tests and explicit skips

Module tests cover a zero step silencing its sine slice, exact backward reverse
capture and immutable replay, hold/repeat/release/reset, scratch sample selection
and fractional interpolation, three differing models on identical input,
latency-aligned rack dry playback through ring wraps, deterministic retrigger,
finite hostile inputs/parameters across all twelve models, tempo/memory bounds,
JSON/default/descriptor/TypeScript contracts and partition invariance.

Checks use an isolated temporary copy of the DSP crate with the module declaration
added there, and run only performance tests. No workspace suite or source
registry changes are needed. The parent remains responsible for actual registry,
automation/project migration, bindings and host latency integration tests.

Validation completed: **13 module tests passed**. Two additional tests in the
temporary harness passed: the allocator guard detects both allocation and free,
and 1,000 rounds of all four processors' parameter/tempo/process/reset callbacks
produce **zero allocations and zero deallocations**, including every rack model.
`rustfmt --check` passed for the four owned Rust files. The temporary crate also
compiled the complete DSP source copy with `pub mod performance;` added, proving
the module's compatibility with the actual Effect/ParamSet contracts without
editing the workspace's module declaration.

The **16 steps are the Volume Gate curve**; a drawn curve editor and arbitrary
time curves are not included. Time Transport uses the trigger parameter, with
no transient detector or MIDI trigger adapter. Scratch operates on live ring
audio; loaded-source decoding and **vinyl decoding are not included**. Rack
controls are parameter-based; an XY pad UI is not included. No proprietary
preset/content decoding, independent-duration pitch shifting, oversampling,
song-position synchronization, UI/example content or full branded acoustic
equivalence is claimed. These are DSP subsets of the four parity references,
not complete parity-row closures.
