# Metronome click-volume presets

The metronome popover offers Quiet (0.25), Medium (0.5), and Loud (1), in
that order, under the existing click-volume number field.

One preset sets only the click volume through `setMetronome({ gain: value })`.
The accent and the on/off switch stay as they are. The number field retains
its existing behavior. Recording disables the presets.

The pure `nextMetronomeGain(current, preset)` helper returns `null` when the
absolute difference is less than 0.001. Otherwise, it returns the preset
value. A matching preset is disabled and dispatches no command.

Focused coverage: `apps/desktop/src/features/transport/metronome-gain.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/transport/metronome-gain.test.ts
```
