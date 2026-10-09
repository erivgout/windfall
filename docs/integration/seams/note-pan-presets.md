# Note pan presets

The piano-roll lane header offers Hard left (-1), Left (-0.5), Center (0),
Right (0.5), and Hard right (1) after the velocity preset group and before
Write LFO. The entries are hidden unless the lane is showing pan. Velocity
presets are unchanged.

Each preset sets pan on the selected notes, or on every note when none are
selected, in one undo step through one `updateNotes` command. Each note patch
contains only pan. Notes already there are left out. A difference smaller than
0.001 counts as a match. Each entry is disabled when there is no editor context
or nothing to change. An empty update list or missing editor context sends no
command.

The pure `notePanPresetUpdates(notes, preset)` helper preserves the given order,
does not mutate the input, and returns only note IDs and pan values.

Focused coverage: `apps/desktop/src/features/piano-roll/note-pan-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/note-pan-presets.test.ts
```
