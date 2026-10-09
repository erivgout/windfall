# Gate presets

The channel inspector's Note timing section shows Written, 16th, 8th, and
Quarter presets between the Gate and Shift rows. They store 0, 240, 480, and
960 whole ticks respectively, using the shared `TICKS_PER_STEP` and `PPQ`
constants for note lengths.

Each preset sets the gate in one undo step through one `updateChannel` command.
Written stores 0, so notes keep their written length. Swing mix and shift stay
as they are at click time because the command reads the latest channel timing.

The matching preset is disabled. `nextGate` returns `null` for an exact match,
and the click handler checks the latest gate again before dispatching. The
knobs, Shift field, and Reset retain their existing behavior.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/gate-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/gate-presets.test.ts
```
