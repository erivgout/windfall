# Windfall playlist Playback integration seam

The playlist Playback tool scrubs the transport by seeking at the pointer.
`playlist.toolPlayback` is registered with the other playlist actions, appears
in the toolbar and tool menus, and uses `Q` in both Windfall and FL keymaps
while the playlist has keyboard focus. Slip remains on `Y`.

## Gesture and transport

`intents.ts` routes a left press to Playback on empty grid or any clip part,
including audio and automation handles. Ctrl and Shift keep that intent.
Middle-button pan remains available; right presses do nothing in Playback.
The Playback cursor and hints describe seeking rather than clip editing.

`PlaylistSession` captures the pointer and calls the existing `seek()` from
`lib/store/transport.ts` on the press and subsequent pointer movements. The
target is the nearest playlist grid tick, using the containing meter segment's
signature and origin. Alt or Snap None uses whole ticks. Targets are bounded
to the song and the containing segment.

Pointer-up, Escape through the existing `playlist.deselect` action, or pointer
cancellation ends the gesture without another seek. Later hover movement does
not seek. The gesture does not dispatch clip commands, alter selection or clip
geometry, or change song/pattern mode. Other tools retain their existing
behavior.

## Verification

From `apps/desktop`:

```sh
pnpm test src/features/playlist/playback.test.tsx src/features/playlist/session.test.ts src/features/playlist/intents.test.ts src/features/playlist/slip.test.tsx
```

The tests cover tool selection through the toolbar and both keymaps, snapped
press/drag seeks over clips and empty grid, no dispatch or transport mode
update, unchanged document/history/geometry, cancellation and release without
another seek, whole-tick seeking, and snapping to a later meter's grid origin.
The same command includes the existing Slip regression tests.
