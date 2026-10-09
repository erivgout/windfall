# Playlist clip length scaling

The Playlist commands **Halve selected clip lengths** (`playlist.length.half`)
and **Double selected clip lengths** (`playlist.length.double`) change the
stored length of the selected clips in one undo step. They read `selectedClips`
and dispatch one `updateClips` command containing only changed lengths. If no
clips are selected or every selected length stays the same, they dispatch
nothing and create no undo step.

Half uses the whole number of ticks at or below half the current length. A
1-tick clip stays 1 tick. Double stops where the clip would pass the end of the
song (`MAX_SONG_TICKS`); a clip already ending there keeps its length. If fewer
than 1 tick remain after a clip's start, double keeps its current length.

Clip starts stay as they are, and unselected clips are unchanged. The
bar-length commands are unchanged.

The implementation is in
`apps/desktop/src/features/playlist/clip-length-scale.ts`, with actions in
`apps/desktop/src/features/playlist/actions.ts`. Run the focused tests from
`apps/desktop`:

```powershell
pnpm test -- src/features/playlist/clip-length-scale.test.ts
```
