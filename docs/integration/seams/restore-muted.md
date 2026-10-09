# Restore muted notes

`rememberedVelocity(pattern, channel, noteId)` reads the mute tool's existing
velocity memory. It returns `undefined` when that note has no remembered value.
`toggledVelocity` and its fallback behavior are unchanged.

`restoredVelocities(notes, remembered)` returns `{ id, velocity }` updates for
notes whose velocity is exactly 0 and whose remembered velocity is greater
than 0, preserving the given order. Sounding notes are omitted. A note that
was drawn at velocity 0, with nothing remembered, stays silent.

`pianoRoll.restoreMutedNotes`, titled **Restore muted notes**, appears immediately
after Select muted notes in both the note and panel menus, with no shortcut.
It is enabled when the open channel has at least one restorable note. It writes
the remembered velocities back with one `updateNotes` command in one undo step,
patching only velocity and leaving the selection unchanged. If nothing can be
restored, it dispatches nothing.

Closing or replacing the project forgets the remembered velocities, which the
mute tool already does. The memory is not saved in the project.

Focused verification from `apps/desktop`:

`pnpm test -- src/features/piano-roll/restore-muted.test.ts`
