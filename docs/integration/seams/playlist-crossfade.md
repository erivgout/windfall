# Playlist audio crossfades

Windfall computes automatic crossfades while compiling the engine playback plan
in `crates/windfall-engine/src/plan.rs`. Document fade values are unchanged;
only the planned fades are extended.

Audible audio clips are grouped by playlist track and sorted by start tick, then
clip ID. Only adjacent same-track overlaps crossfade. When the next clip starts
before the previous clip ends, the overlap is the previous end minus the next
start. The previous clip's planned fade-out and the next clip's planned fade-in
are extended to at least that overlap, each clamped to its own clip length.
Longer user fades are preserved within that limit.

Clips on different playlist tracks do not crossfade, even when routed to the
same mixer track. Non-adjacent overlaps are left alone. Touching or separated
clips do not extend fades; a clip without a following overlap keeps its existing
fade-out. Muted clips, muted playlist tracks, and clips without playable audio
remain excluded from the audio plan.

The existing clip renderer applies equal-power curves to the planned fades.
All overlap ordering and arithmetic happen during plan compilation on the
control side. The audio callback gains no allocation, lock, or I/O.

Verification from Git Bash at the repository root:

```bash
source scripts/msvc-env.sh
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-engine --test playlist_crossfade
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-engine --lib adjacent_audio_overlaps
```

The planner tests cover fade extension, longer user fades, unchanged document
values, playlist-track isolation, touching clips, deterministic ID ordering,
clip-length clamping, muted clips and tracks, and non-adjacent overlaps.
The standalone renderer test passed and verifies the equal-power output,
preservation of longer user fades, unchanged audio on another playlist track
sharing the mixer destination, and unchanged document values. Running the
planner unit tests is currently blocked by unrelated compilation errors in
existing test fixtures in `project_preparation.rs`, `device.rs`, and `render.rs`.
