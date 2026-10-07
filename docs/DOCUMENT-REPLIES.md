# Document replies and recovery

`project:loaded` is the authority for New/Open replacement snapshots and the
existing UI generation signal. New/Open flows await their command for errors
and recent-file bookkeeping but do not load its returned snapshot a second
time. The loaded event can arrive before the reply; reloading the reply would
erase edits made in between or replace a later document with reused revisions.

Edit and history replies check the generation captured when they began.
Snapshot recovery rejects an old generation or a revision older than the
mirror, then fetches again when events overtook that snapshot. A successful
edit dispatch now waits for recovery if its patch exposed a revision gap.
Callers receive created IDs only once the mirror can resolve them. Replacement
during that wait or failed recovery returns null; it cannot attach old IDs to
the new document. A failed recovery does not undo an edit already committed by
the backend; the existing snapshot error reports that synchronization failed.

Independent review reproduced late New/Open rollback and lost Chop selection.
Before these repairs, the focused command
`pnpm test src/lib/flows/project-generation.test.ts src/features/piano-roll/note-tools-recovery.test.ts --maxWorkers=2`
failed all seven initial cases: six New/Open paths rolled revision/history/
dirty state back, and Chop retained only the original ID despite creating two
notes. Removing reply-side snapshot loading made the six replacement cases
pass. Waiting for mirror recovery repaired Chop without changing editor math.
An eighth regression replaces the project during recovery and verifies that
no result selection leaks across documents.

After repair, 66 tests across the new flow/recovery cases, existing project
stores and note-tool integration passed. No persisted field or IPC change was
needed. Backend loaded events remain required; this does not introduce a
second, inferred document-identity mechanism.
