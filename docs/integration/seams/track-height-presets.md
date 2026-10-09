# Track height presets

Each playlist track header's context menu offers a "Track height" submenu
beside Color: Follow (0), Short (18 CSS pixels), Normal (38 CSS pixels), and
Tall (92 CSS pixels).

The pure `nextTrackHeight(current, preset)` helper returns `null` for an exact
match and otherwise returns the preset without clamping. The matching preset
is checked and disabled, and selecting it sends no command.

Each changed preset dispatches one `updatePlaylistTrack` command with
`patch: { height }` for that header's track. Each preset is one undo step for
that track. Follow stores 0, so the row uses the global height again. The
project subscription copies the saved height into the session map.

Dragging the edge still works and saves one update on pointer-up. Color,
mute, solo, and rename keep their existing behavior.

Focused coverage: `apps/desktop/src/features/playlist/track-height-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/track-height-presets.test.ts
```
