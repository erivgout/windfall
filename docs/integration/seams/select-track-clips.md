# Select clips on this track

This only changes which clips are selected. It does not move them, and a clip
on another track stays unselected. The action replaces the playlist selection
without dispatching a project command or adding an undo-history entry.

`playlist.selectTrackClips` ("Select clips on this track") appears immediately
after Rename in the playlist track header menu. It has no shortcut and is
enabled when `ui().targetTrack` names an existing track with at least one clip.

The pure `clipIdsOnTrack(clips, trackId)` helper returns the ids of clips whose
track equals `trackId`, in the given order. The action applies those ids with
`ui().select`. Select matching clips is unchanged.

Focused coverage: `apps/desktop/src/features/playlist/select-track-clips.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/select-track-clips.test.ts
```
