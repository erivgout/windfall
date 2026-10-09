# Channel volume presets

Right-click a channel rack volume knob and open the Volume submenu to choose
Quiet (0.5), Default (0.8), Unity (1), or Loud (1.5), in that order. The submenu
follows the existing automation and show-in-mixer items. All presets are inside
the knob's range of 0 to `MAX_GAIN` (2).

`nextChannelVolume(current, preset)` returns `null` when the absolute difference
is less than 0.001, and otherwise returns the preset. The matching preset is
disabled. Each inline action checks the helper again before dispatching; a match
dispatches no command.

Choosing a different preset dispatches one `updateChannel` command with that
channel's `id` and `patch: { volume }`. Each preset is one undo step for that
channel.

The knob's drag behavior is unchanged. Double-click still resets to 0.8.
Pan is unchanged and has no Volume submenu. Mute and solo are unchanged.

Focused coverage: `apps/desktop/src/features/channel-rack/volume-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/volume-presets.test.ts
```
