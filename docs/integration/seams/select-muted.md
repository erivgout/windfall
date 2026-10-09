# Select muted notes

`mutedNoteIds(notes)` returns the ids of notes whose velocity is exactly 0,
in the given order. Velocity 0 is the mute; notes with velocity above 0 are
left out.

`Editor.selectMutedNotes()` replaces the selection with those ids from the
open channel scene. An empty result clears the selection. This only changes
the selection: it dispatches no project command, changes no note, and creates
no undo step. It does not restore a velocity. The existing selection update
path refreshes the canvas and the piano-roll selection count.

`pianoRoll.selectMutedNotes`, titled **Select muted notes**, appears immediately
after Select matching pitches in both the note and panel menus. It has no
shortcut and is enabled when the open piano-roll channel has at least one
note with velocity 0.

Focused verification from `apps/desktop`:

`pnpm test -- src/features/piano-roll/select-muted.test.ts`

The tests cover ordered muted ids, no muted notes, action selection without
dispatch or note changes, clearing the selection, open-channel availability,
menu placement, and shortcut absence.
