# Select matching pitches

`Editor.selectMatchingPitches()` collects the keys of the currently selected
notes and selects every note on those keys in the open channel scene. The
scene already contains only that channel's notes. An empty selection does
nothing.

This only changes the editor selection. It dispatches no project command,
changes no project notes, and creates no undo step. The existing selection
update path refreshes the canvas and the piano-roll selection count.

`pianoRoll.selectMatchingPitches`, titled **Select matching pitches**, appears
beside Select all notes in both the note and panel menus. It is enabled when
the piano roll has a selection and has no shortcut. The existing `selectKey`
method and key gutter menus are unchanged.

Focused verification from `apps/desktop`:

`pnpm test -- src/features/piano-roll/select-pitch.test.ts`

The tests cover expanding selected C and E notes while leaving D unselected,
an empty selection doing nothing, no project dispatch or undo step, selection
count updates, and action availability, menu placement, and shortcut absence.
