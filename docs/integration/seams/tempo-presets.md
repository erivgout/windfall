# Tempo presets

The tempo readout's context menu offers a "Tempo presets" submenu after the
automation entries and before tap tempo: 80, 100, 120, 128, 140, 160, and
174 BPM, in that order.

The pure `nextTempo(current, preset)` helper returns `null` when the absolute
difference is smaller than 0.001 BPM. Otherwise, it returns the preset without
clamping. Every preset is within the supported 10–522 BPM range. The matching
preset is disabled.

Each preset uses the existing `setTempo` function to dispatch one
`updateSettings` command with `patch: { tempoBpm }`. Each preset is one undo
step. An unchanged value sends no command.

Drag, typing, half, and double stay as they are. Reset and the automation
entries also stay as they are.

Focused coverage: `apps/desktop/src/features/transport/tempo-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/transport/tempo-presets.test.ts
```
