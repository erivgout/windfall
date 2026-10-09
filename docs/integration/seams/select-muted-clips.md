# Select muted playlist clips

These commands only change the playlist selection. They do not dispatch a
project command or change clip mute, track, start, or length. If none match,
the selection stays as it is.

A muted track is not the same as a muted clip. `playlist.selectMutedClips`
("Select muted clips") selects clips whose own `muted` flag is true.
`playlist.selectClipsOnMutedTracks` ("Select clips on muted tracks") selects
every clip on a track whose `muted` flag is true, regardless of the clip's
own flag. An individually muted clip on an unmuted track is left out of
that second command.

Both actions have no shortcut and are enabled in the playlist when their
matching list is nonempty. They appear immediately after Select clips at
the playhead in the playlist panel menu, muted clips first, then clips on
muted tracks. Matching ids replace the selection through
`usePlaylistStore.getState().select`.

The pure helpers `mutedClipIds(clips)` and `clipsOnMutedTracks(clips, tracks)`
return matching clip ids in the given clip order without mutating inputs.

Focused coverage: `apps/desktop/src/features/playlist/select-muted.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/select-muted.test.ts
```
