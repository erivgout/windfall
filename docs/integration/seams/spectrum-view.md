# Spectrum view

Partial view for `vis-fruity-spectroman`. Windfall calls the panel **Spectrum**
and the opening action **Show spectrum** (`view.spectrum`). This is not a new
analyzer, not a spectrogram history, and not hardware-calibrated.

## Frame contract

`RealtimeFrame.spectrum` is a `Vec<f32>` on Rust and an optional `number[]` in
the camelCase desktop binding. Missing input defaults to an empty vector;
serialization omits empty vectors. A missing/empty field means no valid
spectrum is available, rather than a measured silent spectrum.

The publication contains at most **64** ascending, linear-frequency
bands from DC through Nyquist. For an analyzer spectrum with `M` FFT bins,
publish `B = min(M, 64)` bands. Band `i` contains original bins in the half-open
range `[floor(i*M/B), floor((i+1)*M/B))`. All original bins, including DC and
Nyquist, contribute once. Each value is the sum of existing analyzer
`SpectrumBin.mean_square` values in that band, averaged over valid stereo
channels. Values are finite, nonnegative **mean-square power**, not dB;
averaging valid channels avoids cancellation of opposing stereo phases.
If neither channel is valid, or a value cannot be represented as finite f32,
the publication is empty. The default 1024-point analyzer produces 513 source
bins summarized into 64 bands. No new FFT is needed.

The view takes the latest frame through `useRealtime`, just like the transport
readout. It updates a fixed pool of 64 SVG rectangles directly without React
state updates. It converts positive power with `10*log10(power)` for display,
clamps heights to −90…0 dB, and does not retain earlier spectra. Zero power
draws a zero-height bar. Empty input clears all bars and shows **No spectrum
available**; a valid silent spectrum shows **Quiet**. The labels use DC and
Nyquist because this frame does not carry the analyzer sample rate.

Closing/unmounting the view drops its realtime subscription. The shared feed
stops only when its last subscriber leaves, preserving other realtime views.

## Native connection and ownership

This checkout initially had the analyzer foundation but no engine-owned reader
or tap. The authorized scope expansion connects it through engine `spectrum.rs`,
processor attachment, and the existing controller `Link` retirement path.
No analyzer implementation or FFT was changed.

Attachment prepares the existing default analyzer outside controller/document
guards. Processor owns its audio tap; Link owns its worker and snapshot reader.
Each attachment receives a checked, never-reused selection ticket for the
stream's mix-output source. The initial project/plan generation labels that
attachment. Audio copies the stereo mix **after metronome and output gain**, in
chunks of at most 256 frames, with engine-frame timestamps, sample rate, and
the engine's actual PDC latency; device latency remains unknown.

`Controller::frame()` pumps the existing bounded worker on its control caller
and polls its borrowed snapshot. No FFT, allocation, lock, or snapshot copy runs
on audio. Analysis is bounded by the analyzer's default 32 input packets per
pump and its fixed snapshot pool; it progresses when frames are requested and
adds no helper thread. A full queue invalidates earlier evidence and resumes
after a fresh contiguous window. Preparation/quota refusal leaves the spectrum
empty and does not prevent playback.

Transport changes, seeks, plan adoption and explicit navigation jumps reset
the tap's epoch. The frame also compares the published plan identity with the
controller's current plan so a pending project replacement hides earlier power
before audio adopts it. Stream replacement creates new endpoints; retired Link
owners drop off audio through the existing off-guard retirement carrier, and
the audio tap lives and retires with its processor. Cancelled/dropped taps and
invalid snapshots publish empty input. PDC outside the analyzer's supported
one-second range is also treated as unavailable. This provides only the current mix
spectrum, with no source selector or analyzer controls.

## Requested validation

From Git Bash at the repository root:

```sh
source scripts/msvc-env.sh
cargo check -p windfall-engine --lib
```

From `apps/desktop`:

```sh
pnpm test -- src/features/spectrum/view.test.tsx src/features/spectrum/panel.test.tsx
```

View tests cover omitted/empty frames, clearing previously drawn bars, a known
64-bin power fixture, no React render per frame, bounded/invalid data, project
replacement, and unsubscription on unmount. Panel tests cover the opening
action and subscription cleanup on close.

The passing native integration tests cover power preservation for opposing stereo
channels, overflow recovery, project/seek/stream
invalidation, and zero audio-side allocator calls:

```sh
source scripts/msvc-env.sh
cargo test -p windfall-engine --test spectrum_view -- --test-threads=1
```

Minimal desktop integration glue registers the action in the command palette,
mounts `SpectrumPanel` in the shared shell overlays, and forwards/clears the new
field in the realtime store. New visible strings use Windfall's own names.

The additional `cargo test -p windfall-engine --lib spectrum::tests --
--test-threads=1` attempt was blocked by existing unit-fixture compile errors
in `project_preparation.rs`, `device.rs`, and `render.rs` (missing newly added
model fields and mismatched render error types). The library check and separate
native integration target pass without changing those fixtures.

A broader startup-registration test attempt also encountered an existing
duplicate `NotePreviewLane` import in the prohibited channel-rack
`rack-store.ts`. The feature-specific desktop suite passes; that unrelated
file was left untouched.
