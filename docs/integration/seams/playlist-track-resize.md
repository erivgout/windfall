# Playlist track resize

Windfall playlist track headers have a drag handle along their bottom edge. Dragging sets that track's height between `MIN_ROW_HEIGHT` (18 CSS pixels) and `MAX_ROW_HEIGHT` (160 CSS pixels). Other tracks retain their own heights. Tracks without an override follow the global row height and its existing tall toggle. Group headers follow the global height and have no resize handle.

Heights are saved in `PlaylistTrack.height`: `0` means follow the global row height, and non-zero values are CSS pixels. During a drag, `usePlaylistStore.trackHeights` holds the live session preview without sending commands. Release sends one `updatePlaylistTrack` command when the clamped height differs from the saved height; cancel restores the saved height without a command. Project changes, including undo, redo, and replacement, copy saved heights into the session map, skipping tracks with an active resize drag. The map is excluded from persisted UI preferences; the document holds the saved copy.

`row-geometry.ts` projects the visible track/group rows into cumulative pixel spans and converts pointer coordinates back to fractional row indices. Headers, clip boxes, clip content painting, hit testing, reorder gaps, reveal scrolling, and scroll extents share those spans. Spare rows retain the global height. `TrackGridSurface` translates row-index batches to integer pixel spans for the existing canvas renderer; the shared canvas row model remains unchanged for other editors. Clip editing continues to use the existing track indices.

Focused regression coverage lives in `apps/desktop/src/features/playlist/track-resize.test.tsx`. From `apps/desktop`, run:

```powershell
pnpm test -- src/features/playlist/track-resize.test.tsx
```
