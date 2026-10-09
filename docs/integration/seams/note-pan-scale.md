# Note pan scaling

The piano-roll lane header offers Half and Double after Hard left, Left, Center,
Right, and Hard right. The entries are hidden unless the lane is showing pan.
The fixed pan presets stay as they are.

Each command sets pan on the selected notes, or on every note when none are
selected, in one undo step through one `updateNotes` command. Each note patch
contains only pan. Notes that would not change are left out, including notes
already centered. Changes smaller than 0.001 are omitted.

Half divides the current pan by two, moving a note toward the center. Double
multiplies the current pan by two and stops at hard left (-1) or hard right (1).
Neither command rounds.

The pure `notePanScaleUpdates(notes, factor)` helper keeps the given order,
does not mutate the input, and returns only note IDs and pan values. An entry
is disabled when there is no editor context or nothing to change. Clicking
reads the current notes again; an empty update list or missing editor context
sends no command.

Focused coverage: `apps/desktop/src/features/piano-roll/note-pan-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/note-pan-scale.test.ts
```
