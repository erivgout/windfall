# Windfall playlist clip group gesture seam

The desktop playlist reads saved groups from
`playlist.arrangementBook.clipGroups`. `clip-groups.ts` expands a selection
like Rust `ArrangementBook::expand_clip_selection`: retain the original IDs
first, then visit touched groups in saved order and append their members in
saved clip order, without duplicates. A clip belongs to at most one group.
Missing arrangement metadata or an empty group list needs no migration.

## Selection and movement

`PlaylistSession` expands clip-body presses, context-menu selection, and
marquee selection. An additive press keeps every previously selected member,
including when the pressed grouped clip was already selected. Releasing a
plain click retains the pressed clip's entire group. Move updates retain the
expanded selection order.

A drag that touches a clip group snaps the anchor's target start using the
current playlist snap, or whole ticks with Alt/Snap None. Every selected clip
receives that same signed tick delta. The pointer's destination is projected
to a document track index; every member receives the same delta in the saved
playlist track list. This includes group members outside the visible canvas.

The frontend validates every destination before release: start must be at
least zero, end must be at most `MAX_SONG_TICKS`, and the destination track
must already exist. Invalid grouped drags clear the pending update and
preview; release dispatches nothing, including after an earlier valid preview.
A valid move dispatches one existing `updateClips` command inside one labeled
batch, with `{ start, track }` patches and one undo step. Group moves never
create spare tracks or independently clamp members.

Clips outside groups retain the existing gesture path, including its clamping
and creation of spare tracks. Track-group collapse, Slip, and Playback retain
their existing implementations.

## Erase and document integrity

Erase marks the entire group on the initial press and whenever the stroke
crosses another member. Release passes the expanded, deduplicated IDs to the
existing `deleteClips` operation. It dispatches one labeled batch containing
exactly one `removeClips` command for the entire stroke.

The existing document validator rejects dangling clip references. The same
batch therefore first removes fully deleted clip groups with existing
`removeClipGroup` commands and filters deleted IDs from saved arrangements
with existing `setArrangementReferences` commands. Undo restores the clips,
groups, and arrangement references together. No Rust command or generated
binding changes are needed.

## Verification

From `apps/desktop`, run only the added playlist tests:

```sh
pnpm test src/features/playlist/clip-groups.test.ts
```

Coverage checks press/release expansion order, additive selection, shared
snapped tick and track deltas in one update, bounds refusal at both song and
track edges, invalid final destinations, atomic group erasure and undo,
saved-reference cleanup, stroke expansion, ungrouped selection/movement/erase,
context-menu and marquee selection, and existing no-group drag behavior.
