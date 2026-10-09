# Select arrangement clips

The playlist arrangements bar has a **Select arrangement clips** button beside
**Arrangements**. It only changes the playlist selection through
`usePlaylistStore.getState().select(ids)`; it does not dispatch a project command
or switch the active arrangement.

`activeArrangementClipIds(book, liveClipIds)` returns the active arrangement's
live clip IDs in arrangement order, dropping stale IDs and keeping only the
first occurrence of each repeated ID. The helper does not mutate its inputs.

An empty arrangement selects nothing, clearing the playlist selection. No active
arrangement does not change the selection: the helper returns `null` and the
button is disabled. A missing active arrangement also returns `null` and disables
the button.

Focused verification from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/arrangement/select-clips.test.ts
```

The five tests cover live clip order and stale IDs, duplicate IDs, an empty active
arrangement, no active arrangement, and a missing active ID.
