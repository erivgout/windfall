# Mixer pan scale

Right-click a mixer strip's pan knob and open the Pan submenu to choose
Half or Double after Hard left, Left, Center, Right, and Hard right.

Half divides that strip's current pan by two without rounding, moving it
toward the center. Double multiplies it by two without rounding and stops at
hard left (-1) or hard right (1). Center stays centered for both commands.

`nextMixerPanScale(pan, factor)` returns `null` when the absolute change is
smaller than 0.001. The matching item is disabled. Each action reads the latest
pan for that track from `useProjectStore.getState().project.mixer.tracks` before
calculating the new pan. A missing track or a change smaller than 0.001
dispatches no command.

Each command sets that one strip's pan in one undo step by calling
`patchTrack(track.id, { pan: next })` with no other patch fields. Other selected
strips stay as they are. The fixed pan presets stay as they are.

Focused coverage: `apps/desktop/src/features/mixer/mixer-pan-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/mixer-pan-scale.test.ts
```
