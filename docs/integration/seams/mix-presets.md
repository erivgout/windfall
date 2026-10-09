# Effect mix presets

Right-click an effect's dry/wet mix knob and open the Mix submenu after the
automation entries. Choose Dry (0), 25% (0.25), Half (0.5), 75% (0.75), or
Wet (1), in that order.

`nextMix(current, preset)` returns `null` when the absolute difference is less
than 0.001, and otherwise returns the preset. The matching preset is disabled.
Each inline action checks the helper again before dispatching; a match sends
no command.

Choosing a different preset dispatches one `updateEffect` command for that
track and effect, with a patch containing only `mix`. Each preset is one undo
step for that effect.

Double-click still sets 100%. The enable lamp is unchanged, as are the effect
menu and knob drag behavior.

Focused coverage: `apps/desktop/src/features/mixer/mix-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/mix-presets.test.ts
```
