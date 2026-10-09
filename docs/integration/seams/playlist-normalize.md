# Playlist clip normalize

Windfall stores an optional `normalize` flag on each audio clip. Missing or false
means off; false is omitted from project JSON. New audio clips start off.
`AudioClipPatch.normalize` enables or clears it with the undo labels “Normalize
audio clip” and “Clear audio clip normalize”. The stored gain knob is unchanged.

The clip inspector has a Normalize switch beside Gain. It sends one
`updateAudioClips` command containing a patch for each selected audio clip.
Missing flags display as off; mixed selections turn on together.

During `compile_audio_clips`, the engine scans all interleaved samples of the
prepared playback buffer (including a prepared spectral variant) for the maximum
absolute peak. Enabled clips use `gain / peak`, capped at `MAX_GAIN`. Silence,
peaks below `1.0e-8`, and non-finite peaks keep the knob gain. Disabled clips keep
the existing gain behavior. This work happens during planning, never in the audio
callback; playback consumes the planned scalar gain.

This does not create a file, replace a sample file, or modify a decoded buffer.
The baked audio-editor normalize remains available and unchanged: that operation
can still produce a new normalized WAV. It is separate from this playback flag.

Regression coverage checks legacy loading, serialization, patch history, peak
scaling, silence, disabled normalization, tiny/non-finite peaks, the gain ceiling,
buffer identity, and the inspector's single-command dispatch.

## Verification

Passed in Git Bash from the repository root:

```bash
source scripts/msvc-env.sh
export TS_RS_EXPORT_DIR=target/ts-rs-discard
cargo check -p windfall-project -p windfall-engine -p windfall-flp
cargo test -p windfall-project --test playlist_normalize
cargo test -p windfall-engine --test playlist_normalize
```

Passed from `apps/desktop` (three switch dispatch cases):

```bash
npx vitest run src/features/playlist/audio/audio-ui.test.tsx -t normalize
```

The direct planned-gain unit regression is also present in `plan.rs`. Running
`cargo test -p windfall-engine --lib audio_clip_normalize` is blocked by existing
missing fields in plugin/channel test fixtures and `StemError`/`RenderError`
mismatches in render tests. The renderer integration test runs independently,
including prepared spectral audio, tiny peaks, and non-finite samples.

Broader checks encounter unrelated workspace failures: duplicate plugin-host
constants/functions block `cargo check --workspace`; desktop `npm run typecheck`
reports existing backend/type inconsistencies; the full audio UI test file has
one stale mixer-route label expectation (`Mixer track:` versus `Playback route:`).

## Changed files

- `apps/desktop/src-tauri/src/session/analysis_jobs.rs`
- `apps/desktop/src-tauri/src/session/audio_editor.rs`
- `apps/desktop/src-tauri/src/session/library.rs`
- `apps/desktop/src-tauri/src/session/recording_multitrack.rs`
- `apps/desktop/src-tauri/src/session/recording_takes.rs`
- `apps/desktop/src-tauri/src/session/tests/song.rs`
- `apps/desktop/src/bindings/AudioClipPatch.ts`
- `apps/desktop/src/bindings/ClipContent.ts`
- `apps/desktop/src/features/playlist/audio/audio-ui.test.tsx`
- `apps/desktop/src/features/playlist/audio/clip-inspector.tsx`
- `crates/windfall-engine/src/audio_edit.rs`
- `crates/windfall-engine/src/bin/windfall-soak.rs`
- `crates/windfall-engine/src/plan.rs`
- `crates/windfall-engine/tests/engine/support.rs`
- `crates/windfall-engine/tests/playlist_crossfade.rs`
- `crates/windfall-engine/tests/playlist_normalize.rs`
- `crates/windfall-engine/tests/playlist_track_solo.rs`
- `crates/windfall-flp/src/convert/playlist.rs`
- `crates/windfall-project/src/check.rs`
- `crates/windfall-project/src/command.rs`
- `crates/windfall-project/src/lower.rs`
- `crates/windfall-project/src/model.rs`
- `crates/windfall-project/tests/commands.rs`
- `crates/windfall-project/tests/file.rs`
- `crates/windfall-project/tests/playlist_normalize.rs`
- `crates/windfall-project/tests/properties.rs`
- `crates/windfall-project/tests/slicer.rs`
- `docs/integration/seams/playlist-normalize.md`
