# Channel rack levels

`channel.unmuteAll` ("Unmute all"), `channel.unsoloAll` ("Unsolo all"), and
`channel.resetLevels` ("Reset levels") are available in the rack menu after the
pattern entries and in each channel button menu after Mute and Solo. They have
no shortcuts and are enabled when at least one channel needs the corresponding
change. They apply to every project channel, including channels outside the
current group filter.

The commands live in `apps/desktop/src/features/channel-rack/channel-ops.ts`.
Each command dispatches one batch and is one undo step:

- `unmuteAllChannels()` uses "Unmute channels" and patches only muted channels
  with `muted: false`.
- `unsoloAllChannels()` uses "Unsolo channels" and patches only solo channels
  with `solo: false`. It does not turn any channel on or change mute.
- `resetChannelLevels()` uses "Reset channel levels" and patches only channels
  whose volume is not 1 or pan is not 0. Each channel receives one
  `updateChannel` containing only `volume: 1`, `pan: 0`, or both, according to
  which fields differ.

Unchanged channels are omitted. If no channel needs a change, nothing is
dispatched and no undo step is added. Reset levels does not change mute, solo,
or routing; name and color also remain unchanged.

Focused coverage: `apps/desktop/src/features/channel-rack/channel-levels.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/channel-levels.test.ts
```
