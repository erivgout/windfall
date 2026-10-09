# Mixer pan presets

Right-click a mixer pan knob and open the Pan submenu after the existing
automation entries. Choose Hard left (-1), Left (-0.5), Center (0), Right (0.5),
or Hard right (1), in that order.

`nextPan(current, preset)` returns `null` when the absolute difference is less
than 0.001, and otherwise returns the preset. The matching preset is disabled.
Each inline action checks the helper again before dispatching; a match sends
no command.

Choosing a different preset calls `patchTrack(track.id, { pan })`, which sends
one `updateMixerTrack` command for the clicked strip. Each preset is one undo
step for that strip. Other selected strips stay where they are.

Dragging still moves selected strips together. Double-click still centers the
knob. Fader, mute, and solo behavior are unchanged.

Focused coverage: `apps/desktop/src/features/mixer/pan-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/pan-presets.test.ts
```
