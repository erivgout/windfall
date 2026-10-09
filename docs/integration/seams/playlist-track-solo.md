# Playlist track solo

Windfall saves `PlaylistTrack.solo` as a boolean. Missing fields in old projects load as `false`, false is omitted when serializing, and newly added tracks start unsoloed. `PlaylistTrackPatch.solo` is optional. A solo-only patch uses the undo label **Solo playlist track** or **Unsolo playlist track** and follows the existing track edit, playlist patch, save/load, and undo/redo paths. The two playlist track TypeScript bindings were updated by hand with `solo?: boolean`.

The track header has an **S** control beside the existing mute control. Each toggle sends one `updatePlaylistTrack` with `{ solo }`. The returned project patch updates the pressed state, the visible **Solo** label, and the header hint. Legacy desktop tracks with no solo field show an unpressed control.

Pattern, audio, and automation clips all use the same playlist track silence rule: a muted track is silent; when any playlist track is solo, every non-solo track is also silent. **Mute still wins**, including on a solo track. When no track is solo, only muted tracks are silenced. Compilation does not change stored mute flags, and silent clips still contribute to song length. Multiple tracks can be solo at once.

**Group solo is absent.** Groups retain their existing mute behavior. Playlist track solo is independent of channel and mixer solo. Crossfade overlap calculations, delay modulation, row resizing, clip groups, and link-label behavior are unchanged. The link-label test's exact header-text expectations account for the added S control.

The project command test covers new-track defaults, legacy missing-field loading, patches in both directions, labels, save/load, undo/redo, and preserving mute. Three engine integration tests use the public renderer to compare pattern, audio, and automation playback against explicitly clip-muted references. They cover one solo silencing another track, a muted solo staying silent, no solos, and multiple solos. Tempo automation also checks rendered duration, so automation suppression is observable even when all sound is silent. Desktop tests cover one command per toggle, mirror updates, legacy defaults, muted solo display, and the absence of group solo.

Passed from the repository root in Git Bash:

```bash
source scripts/msvc-env.sh
export TS_RS_EXPORT_DIR="$PWD/target/ts-rs-discard"
cargo test -p windfall-project --test playlist_track_solo --test playlist_track_color --test slicer
cargo test -p windfall-engine --test playlist_track_solo --test playlist_crossfade
cargo check -p windfall-project -p windfall-engine -p windfall-flp
```

Passed from `apps/desktop`:

```bash
pnpm test -- src/features/playlist/track-solo.test.tsx src/features/playlist/track-link.test.tsx src/features/playlist/track-color.test.tsx src/features/playlist/track-resize.test.tsx src/features/playlist/track-group-view.test.tsx
pnpm exec prettier --check src/features/playlist/track-headers.tsx src/features/playlist/ops.ts src/features/playlist/track-solo.test.tsx
```

Broader checks remain blocked by unrelated working-tree errors: the engine unit-test target has incomplete `PluginBinding`/`Channel` fixtures and render-error type mismatches; the project `commands` test target has an incomplete `ChannelPatch` fixture; desktop `pnpm typecheck` reports existing analysis backend, command fixture, meter-change, and instrument-type errors. The solo integration tests compile and pass without changing those areas.
