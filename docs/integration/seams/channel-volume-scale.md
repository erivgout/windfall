# Channel volume scale

Right-click a channel rack volume knob and open the Volume submenu to choose
Half or Double after the fixed Quiet, Default, Unity, and Loud presets.

Half divides the channel's current linear volume by two without rounding.
Double multiplies it by two without rounding and stops at the maximum volume,
`MAX_GAIN` (2). Half of silence stays silent.

`nextChannelVolumeScale(volume, factor)` returns `null` when the absolute change
is smaller than 0.001. The matching command is disabled. Each action reads the
latest channel with `findChannel` before calculating the new volume. A missing
channel or a change smaller than 0.001 dispatches no command.

Each command sets that channel's volume in one undo step by dispatching one
`updateChannel` command with `patch: { volume: next }` and no other patch fields.
Pan and the fixed Quiet, Default, Unity, and Loud presets stay as they are.
The volume knob's behavior stays as it is.

Focused coverage: `apps/desktop/src/features/channel-rack/volume-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/volume-scale.test.ts
```
