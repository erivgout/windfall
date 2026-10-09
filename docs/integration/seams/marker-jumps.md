# Jump to playlist markers

The playlist ruler context menu lists every marker from
`project.playlist.timeline.markers` after "Jump to the end of the song" and
before the existing separator. An empty marker list adds no entries or
separators.

The pure `markerJumps` helper orders entries by tick, then marker id. Names
with non-whitespace text produce "Jump to {name}"; blank names produce
"Jump to marker". Named, loop, skip, and pause markers are all included.
Loop and skip destinations use the marker's start tick, never its end tick.

Choosing an entry calls the same `seekSong` used by the start and end items.
This seeks the song and does not edit markers, dispatch a project command,
or add an undo-history entry.

Focused coverage: `apps/desktop/src/features/playlist/marker-jumps.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/marker-jumps.test.ts
```
