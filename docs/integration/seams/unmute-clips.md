# Unmute playlist clips

`tools.unmutePlaylistClips` ("Unmute playlist clips") appears after the existing
Tools actions and has no shortcut.

The action includes every playlist clip whose `muted` flag is true in a single
`updateClips` command. This is one undo step for every muted clip together.
Clips that are already sounding are left out. Each update patches only
`{ muted: false }`, preserving track, start, length, offset, and content.
If no clips are muted, the action dispatches nothing and adds no undo step.

Focused coverage: `apps/desktop/src/features/tools/actions.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/tools/actions.test.ts
```
