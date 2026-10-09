# Playlist track group solo

Group solo writes the existing member playlist track `solo` flags. `TrackGroup`
stays `{ id, name }`; no group solo state is saved. The engine plan is unchanged:
playback silences non-solo tracks when any track is solo, and muted tracks stay
silent even when solo.

`toggleGroupSolo` in `apps/desktop/src/features/playlist/track-group-ops.ts`
resolves all descendants with `buildTrackRows(playlist(), new Set())`, including
nested tracks and members hidden by collapse. If any member is not solo, it solos
every member; if all members are solo, it unsolos every member. Empty groups
dispatch nothing.

Each press dispatches one `batch` labeled `Solo playlist track group` or
`Unsolo playlist track group`. Its `updatePlaylistTrack` commands patch only
`{ solo }`, preserving mute flags and producing one undo step. The group header
solo control is pressed when every member is solo and disabled for empty groups.

Focused verification from `apps/desktop`:

```powershell
pnpm test -- src/features/playlist/track-group-view.test.tsx src/features/playlist/track-solo.test.tsx
```
