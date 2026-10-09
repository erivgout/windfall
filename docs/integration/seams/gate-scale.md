# Gate scaling

The channel inspector's Note timing section shows Half and Double buttons
after the fixed gate presets and before the Shift row. Each command sets the
gate in one undo step through one `updateChannel` command.

Half uses the whole number of ticks at or below half the current gate. A
written-length gate of 0 stays 0. Double stops at the maximum gate,
`MAX_PATTERN_STEPS * TICKS_PER_STEP`.

Buttons are disabled when scaling would leave the gate unchanged. Each click
checks the latest gate again before dispatching. Swing mix and shift stay as
they are at click time because the command reads the latest channel timing.
The fixed gate lengths stay as they are.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/gate-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/gate-scale.test.ts
```
