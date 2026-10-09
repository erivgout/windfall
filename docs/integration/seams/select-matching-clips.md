# Select matching clips

This only changes the playlist selection. It does not edit the project, dispatch
a project command, or add an undo-history entry.

`playlist.selectMatchingClips` ("Select matching clips") is available beside
Select all in the playlist panel menu and in the clip context menu. It has no
shortcut and is enabled in the playlist when at least one clip is selected.

The action passes `selectedClips()` and `playlist().clips` to the pure
`selectMatchingClips` helper, then applies the returned clip ids with
`usePlaylistStore.select`.

Pattern clips match by pattern id, audio clips by sample id, and automation
clips by automation id. Audio gain, pan, fades, mixer track, and other playback
settings do not affect matching. A mixed selection expands each selected kind
independently, so equal source ids across kinds never match. Existing selected
clips remain selected, duplicates are removed, and an empty selection stays
empty.

Focused coverage: `apps/desktop/src/features/playlist/select-matching.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/select-matching.test.ts
```
