# Playlist track group view

Windfall's playlist headers and clip canvas share the visible row projection in
`apps/desktop/src/features/playlist/track-rows.ts`. It reads saved groups from
`playlist.arrangementBook.trackGroups`, nested parents from `groupParents`, and
track membership from `trackParents`. Groups appear at their first descendant
track's position; empty groups follow the tracks. Ungrouped tracks retain their
relative document order. Group members are indented, including nested groups.
Projects without groups retain the existing flat layout.

Collapse is session state in `usePlaylistStore.collapsedGroups`. It is not saved
to the project or local storage, dispatches no command, and is cleared when a
project is opened or reloaded. Collapsing a group hides descendant headers and
clips from the canvas and its hit-test batch. Expanding restores them. Group
header rows cannot receive clips; visible track and spare-row edit destinations
are translated back into document track positions.
This includes audio brush placement and audio file drops on the clip canvas.

The group mute control considers all descendant tracks, including hidden members.
If any is unmuted, it mutes every member; otherwise it unmutes every member. Each
press dispatches one `batch` of `updatePlaylistTrack` commands with `muted`
patches, producing one undo step. Empty groups dispatch nothing. Group solo
writes the existing member track solo flags; see [playlist track group solo](track-group-solo.md).

Focused verification from `apps/desktop`:

```powershell
pnpm test -- src/features/playlist/track-group-view.test.tsx
```

The suite covers header/canvas collapse and expand, nested membership, both mute
directions and batch history, project replacement, unsaved collapse, empty
groups, clip and audio brush destinations, member reordering, and the flat layout
without groups.
