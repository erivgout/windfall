# Playlist clip length presets

The Playlist actions `playlist.length.1`, `playlist.length.2`,
`playlist.length.4`, and `playlist.length.8` set selected clips to 1, 2, 4,
and 8 bars. Each command sets the stored length of the selected clips in one
undo step using one `updateClips` command.

The length is that many bars of the project time signature. In 4/4, one bar
is 3840 ticks. A meter change later in the song is not used.

Clip starts stay as they are. Clips that are not selected stay as they are.
Updates contain only clip IDs and length patches. Clips already at the
requested length are omitted; no command is dispatched when all selected
clips already match or when no clips are selected.

Focused coverage: `apps/desktop/src/features/playlist/clip-length-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/clip-length-presets.test.ts
```
