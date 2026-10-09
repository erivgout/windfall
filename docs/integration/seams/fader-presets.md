# Mixer fader volume presets

Right-click a mixer fader and open the Volume submenu to choose Quiet (0.5),
Unity (1), or Loud (1.5), in that order. The submenu follows the existing
automation and Reset peak readout entries and is shared by the horizontal and
upright faders. All presets are inside the range of 0 to `MAX_GAIN` (2).

`nextFaderVolume(current, preset)` returns `null` when the absolute difference is
less than 0.001, and otherwise returns the preset. The matching preset is
disabled. Each inline action checks the helper again before dispatching; a
match dispatches no command.

Choosing a different preset dispatches one `updateMixerTrack` command with the
clicked strip's `id` and `patch: { volume }`. Each preset is one undo step for
that strip. Other selected strips stay where they are.

Fader dragging still moves selected strips together. Double-click still sets
0 dB (volume 1). The meter, peak readout, and pan behavior are unchanged.

Focused coverage: `apps/desktop/src/features/mixer/fader-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/fader-presets.test.ts
```
