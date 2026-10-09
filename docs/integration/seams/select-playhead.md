# Select clips at the playhead

This only changes the playlist selection. It does not dispatch a project
command, move clips, or add an undo-history entry.

`playlist.selectAtPlayhead` ("Select clips at the playhead") follows Select
matching clips in the playlist panel menu and has no shortcut. It is enabled
in the playlist when at least one clip covers `realtimeFrame().tick`.

The pure `clipsAtTick` helper returns clip ids in their given order using
`start <= tick && tick < start + length`. A clip starting exactly at the
playhead is included. A clip ending exactly there, or one that has already
ended, is left out. Muted clips that still cover the playhead are included.

The action applies those ids with `usePlaylistStore.getState().select`.
When no clips overlap, it does not call `select`, preserving the current
selection.

Focused coverage: `apps/desktop/src/features/playlist/select-playhead.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/select-playhead.test.ts
```
