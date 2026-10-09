# Windfall playlist Slip integration seam

The playlist's Slip tool slides content inside a clip's fixed window. Its
action is `playlist.toolSlip`, registered with the other playlist tools and
shown in the playlist toolbar and View > Playlist menu. `Y` selects it in
both the Windfall and FL keymaps while the playlist has keyboard focus.

## Gesture and command

Left-drag any pattern, audio, or automation clip, including its edges or
content handles. Only the pressed clip slips, even within a larger selection.
The gesture uses the signed horizontal travel from the press in song ticks:

`offset = clamp(originalOffset - snappedTravel, 0, MAX_SONG_TICKS)`

Travel snaps to the current playlist grid. Alt or Snap None uses whole ticks.
Rightward travel decreases the offset; leftward travel increases it. Start,
length, track, mute, and content remain fixed. The tool stores the full pattern
offset without wrapping; playback already applies pattern-length modulo.

`PlaylistSession` holds the draft until release. `PlaylistScene.slipDraft`
passes the offset to `ClipPainter`, which previews notes, waveforms, or curves
inside the existing span without moving the canvas batch's geometry. Without
a slip draft, the painter continues to use the document's clip offset.

A completed drag that changes the offset dispatches exactly one existing
command, with no other clip fields in its patch:

```ts
{ type: "updateClips", updates: [{ id, patch: { offset } }] }
```

This is one undo step. A click, vertical-only drag, or unchanged clamped offset
does not dispatch. Escape uses the existing active-session cancel action;
pointer cancellation also calls `session.cancel()`. Both clear the preview
without editing the document. Ctrl and Shift on a clip keep the Slip intent;
Ctrl on empty grid retains marquee selection. Middle-button pan and
right-button erase follow the other playlist tools.

## Verification

From `apps/desktop`:

```sh
pnpm test src/features/playlist/slip.test.tsx src/features/playlist/session.test.ts src/features/playlist/intents.test.ts
```

Coverage includes 480 → 240 on a 240-tick right drag and 480 → 720 on a left
drag for all three content types, unchanged clip fields, one command and undo,
draft-only preview, direct and Escape cancellation, both offset bounds,
unwrapped pattern offsets, snap and Alt behavior, single-clip edits in a
selection, toolbar selection, and both shortcut presets.
