# Mixer sidechain level presets

Right-click a mixer sidechain gain knob and open the Sidechain level submenu
after the automation entries to choose Off (0), Quiet (0.5), Unity (1), or
Loud (1.5), in that order. All presets are within the range of 0 to `MAX_GAIN` (2).

`nextSidechainGain(current, preset)` returns `null` when the absolute difference
is less than 0.001, and otherwise returns the preset. The matching preset is
disabled. Each inline action checks the helper again before dispatching; a
match dispatches no command.

Choosing a different preset dispatches one `setSidechain` command with `from`,
`to: send.target`, and `gain`. Each preset is one undo step for that sidechain.
Off leaves the sidechain in place at silence.

Knob dragging is unchanged. Double-click still sets 0 dB (gain 1). Removing
the sidechain is unchanged.

Focused coverage: `apps/desktop/src/features/mixer/sidechain-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/sidechain-presets.test.ts
```
