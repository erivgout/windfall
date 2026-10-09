# Channel pan presets

Right-click a channel rack pan knob and open the Pan submenu to choose
Hard left (-1), Left (-0.5), Center (0), Right (0.5), or Hard right (1), in that
order. The submenu follows the existing pan menu items.

`nextChannelPan(current, preset)` returns `null` when the absolute difference
is less than 0.001, and otherwise returns the preset. The matching preset is
disabled. Each inline action checks the helper again before dispatching; a match
dispatches no command.

Choosing a different preset dispatches one `updateChannel` command with that
channel's `id` and `patch: { pan }`, containing only pan. Each preset is one undo
step for that channel.

The knob's drag behavior is unchanged. Double-click still centers the knob.
Volume is unchanged.

Focused coverage: `apps/desktop/src/features/channel-rack/pan-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/pan-presets.test.ts
```
