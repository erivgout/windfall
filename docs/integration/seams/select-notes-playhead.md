# Select notes at the playhead

`pianoRoll.selectNotesAtPlayhead`, titled **Select notes at the playhead**, appears
immediately after Restore muted notes in both the note and panel menus, with no
shortcut. It only changes the selection: it does not change notes or dispatch a
project command.

The playhead is the one the piano roll is already drawing, read from the current
session as a pattern-local tick. The action floors that tick and selects the open
editor's notes covering it. A note starting on that tick is included; a note that
has already ended, including one ending exactly on that tick, is left out.

`notesAtTick(notes, tick)` preserves the given order and does not floor the tick.
The action is enabled only with a numeric playhead and at least one covering note.
With no playhead or no covering note, the selection stays.

Focused verification from `apps/desktop`:

`pnpm test -- src/features/piano-roll/select-playhead.test.ts`
