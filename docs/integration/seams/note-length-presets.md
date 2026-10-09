# Note length presets

The piano-roll actions offer 16th (240 ticks), 8th (480 ticks), Quarter
(960 ticks), Half (1920 ticks), and Whole (3840 ticks) after the glide actions
and before Note properties. Each command sets the stored length of the selected
notes in one undo step through one `updateCapturedNotes` command. A quarter
stores 960 ticks.

Note starts and expression stay as they are. Notes that are not selected stay
as they are. This does not quantize note ends.

`lengthPresetUpdates(notes, preset)` omits notes already at the preset, preserves
the given order, does not mutate the input, and returns only `id` and `length`.
`setSelectedLength(preset)` requires an editor context, selected notes, and an
editor that is not busy. An empty update list sends no command.

Focused coverage: `apps/desktop/src/features/piano-roll/length-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/length-presets.test.ts
```
