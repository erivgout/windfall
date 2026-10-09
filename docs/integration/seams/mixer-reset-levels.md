# Reset mixer levels

`mixer.resetLevels` ("Reset levels") is available next to Unmute all in the
mixer menu and in both insert and master strip header menus. It has no shortcut
and is enabled when any mixer track has volume other than 1 or pan other than 0.

`resetLevels()` in `apps/desktop/src/features/mixer/operations.ts` includes every
mixer track, including the master. It dispatches one batch labeled "Reset mixer
levels", making the reset one undo step. Each changed track receives one
`updateMixerTrack` with only the fields that differ: `volume: 1`, `pan: 0`, or
both. Tracks already at unity are omitted; when all tracks are at unity, nothing
is dispatched.

The reset does not touch mute, solo, effects, routing, sends, or recording.

Focused coverage: `apps/desktop/src/features/mixer/reset-levels.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/reset-levels.test.ts
```
