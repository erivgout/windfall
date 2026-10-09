# Windfall playlist Slice integration seam

The playlist Slice tool places a vertical cut across all tracks. The action
`playlist.toolSlice` uses **C** in both Windfall and FL keymaps, in the playlist
keyboard scope. C was free in that scope. Registration follows Slip and
Playback through `intents.ts`, `actions.ts`, `toolbar.tsx`, `session.ts`,
`grid.tsx`, and `lib/actions/menus.ts`; the Tool switches remain exhaustive.

## Gesture and preview

A left press on empty grid or any clip part starts a captured Slice gesture.
Dragging positions a vertical preview line across the grid. It snaps to the
nearest playlist grid line using the containing meter segment's signature and
origin. Alt or Snap None uses whole ticks. The overlay painter reads the cut
tick from the session, converts it to device coordinates on each frame, and
is removed when the session is destroyed.

Release uses the final pointer position and modifiers. Escape through the
existing `playlist.deselect` action or pointer cancellation clears the line
and ends the gesture without dispatching. Selection, clip inner editors, and
clip-group expansion are not involved in a cut. Existing middle-button pan,
Ctrl marquee selection, and right-button erase conventions still apply.

## Command boundary

Every document clip with `clip.start < cut < clip.start + clip.length` splits,
including pattern, audio, and automation clips. A cut at either edge leaves
the clip unchanged. An empty result dispatches nothing.

One existing `batch` command contains:

- `updateClips`: keeps each original ID and changes only its length to
  `cut - clip.start`.
- `addClips`: creates each right piece on the same track with the same mute
  state and content, starting at the cut, with the remaining length and
  offset `clip.offset + cut - clip.start`.

The batch is one undo step. No group command is sent: members whose spans do
not cross the cut stay unchanged, original group membership remains, and new
right pieces are ungrouped. The existing audio slicer dialog is unchanged.

## Verification

Run from `apps/desktop`:

```sh
pnpm test src/features/playlist/slice.test.tsx
```

The new test covers pattern/audio/automation content and offsets, strict
edges and empty cuts, two crossing clips in one batch, unchanged clip groups,
one-step undo/redo, the preview painter, grid and whole-tick snapping, meter
origins, release coordinates, Escape and pointer cancel, and toolbar and
keyboard registration in both keymaps.
