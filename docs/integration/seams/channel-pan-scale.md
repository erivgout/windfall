# Channel pan scale

Right-click a channel rack pan control and open the Pan submenu to choose
Half or Double after Hard left, Left, Center, Right, and Hard right.

Half divides the channel's current pan by two without rounding, moving it
toward the center. Double multiplies it by two without rounding and stops at
hard left (-1) or hard right (1). Center stays centered for both commands.

`nextChannelPanScale(pan, factor)` returns `null` when the absolute change is
smaller than 0.001. The matching item is disabled. Each action reads the latest
channel with `findChannel` before calculating the new pan. A missing channel
or a change smaller than 0.001 dispatches no command.

Each command sets that channel's pan in one undo step by dispatching one
`updateChannel` command with `patch: { pan: next }` and no other patch fields.
Volume and the fixed pan presets stay as they are.

Focused coverage: `apps/desktop/src/features/channel-rack/pan-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/pan-scale.test.ts
```
