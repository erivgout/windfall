# Audition note logger

Windfall's desktop piano roll keeps a session-only log in
`apps/desktop/src/features/piano-roll/note-log.ts`. The existing `auditionOn`
and `auditionOff` helpers record calls alongside their unchanged backend
calls. MIDI hardware is not logged: only notes that pass through
`auditionOn` and `auditionOff` enter this log.

A matching on/off pair for the same channel and key becomes one held note,
preserving velocity, onset and release time. Repeated on calls while that
key is held keep its original onset; unmatched off calls create nothing.
Entries whose onset is older than 20 seconds expire, including incomplete
holds. Completed notes and pending holds are each capped at 1,024 entries.
An expiry timer updates command availability; reads also check age if the
browser delays timers. New or Open clears both completed pairs and holds
through `onProjectReplaced`, including when the new project reuses channel IDs.

`pianoRoll.dumpPlayedNotes`, titled **Dump played notes**, is registered in
the piano-roll actions and exposed in the command palette and context menus.
It is enabled with a pattern channel open in the piano roll and at least one
completed pair for that channel, while the editor and dump are idle.
`dump-played-notes.ts` converts elapsed milliseconds to ticks using the
current project tempo. It anchors the first onset at the realtime transport
position and rounds starts to the shared snap from `useSnapStore`, respecting
the pattern's local meter and ignoring piano-only snap divisions. Thus an
off-grid transport position is itself snapped. With snap off, starts round
to whole ticks. Each held length is at least one sixteenth-note step (240
ticks), with longer holds rounded to whole ticks.

The dump sends exactly one `addNotes` command to the open pattern/channel.
An empty log dispatches nothing. A successful dispatch consumes only the
captured pairs for that channel; other channels, incomplete holds and pairs
played while the command is pending remain. Failure retains the pairs for
retry, and notes beyond the maximum pattern end are refused before dispatch.
Concurrent dumps are blocked and a project replacement invalidates an
in-flight dump's cleanup.

The log is neither saved in the project nor persisted in UI preferences.
Undo removes the inserted notes through normal document history and does not
restore the consumed log.

Focused verification from `apps/desktop`:

`pnpm test src/features/piano-roll/note-logger.test.ts`

The test covers pair timing and shared snap, one dispatch and one undo step,
channel isolation and later consumption, an empty or incomplete log, New/Open
clearing completed and held notes, command availability, the 20-second window,
the 1,024-note cap and preserving pairs after dispatch failure.
