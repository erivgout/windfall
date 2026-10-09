# Shift presets

The channel inspector's Note timing section shows 16th early, On grid,
16th late, and 8th late presets after the Shift row. They store -240, 0,
240, and 480 whole ticks respectively, using the shared `TICKS_PER_STEP`
constant. Each preset is within the -960 to 960 shift range.

Each preset sets the shift in one undo step through one `updateChannel`
command. On grid stores 0, so notes start where they were written. Swing mix
and gate stay as they are at click time because the command reads the latest
channel timing.

The matching preset is disabled. `nextShift` returns `null` for an exact
match, and the click handler checks the latest shift again before dispatching.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/shift-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/shift-presets.test.ts
```
