# Project backups browser

Windfall's desktop browser offers Library, Project, and Backups tabs. The new
`src/features/browser/backups/BackupsTab` reads `DocumentState.path` through
`useProjectStore`. A null path displays that the project has no backups because
it has not been saved; it makes no `browserList` call.

For `C:/songs/Demo.windfall`, the view calls the existing
`backend.browserList("C:/songs/Backup")`. Windows backslashes are normalized to
forward slashes. The file stem is taken from the saved filename, independently
of the project's display name. Files must match the literal stem, one space,
the exact 19-character `YYYY-MM-DD HH-mm-ss` digit/separator shape, and
`.windfall`. Directories, other projects, and malformed names are excluded.
Matching filenames are sorted descending, which gives newest-first ordering.
The timestamp shape is checked without calendar validation.

An empty result or the backend's exact NotFound message,
`The folder "<Backup path>" does not exist.`, displays No backups. Other list
failures display an inline error without changing the project store or opening
a file. This missing-folder distinction depends on the existing `browser_list`
error text; no backend command or error format is added.

Each row's Open button passes the listed entry's path to the existing
`openProjectPath` in `src/lib/flows/project.ts`. That shared flow retains its
unsaved-change confirmation and backup-as-copy behavior. The view explains
that opening a backup does not replace the original file. It does not write,
delete, or prune backups.

Listings load when the Backups view mounts or the saved path changes. A keyed
saved-project view removes old rows immediately on path changes, and effect
cleanup ignores late replies after switching projects or tabs. Re-entering the
tab reloads the folder; there is no polling for backups created while it stays
open. Library and Project keep their existing behavior and stores.

Focused verification from `apps/desktop`:

`pnpm test src/features/browser/backups/backups.test.tsx`

Tests mock `browserList` and cover the unsaved state, project-only filtering and
newest-first ordering, Open delegation, empty/missing folders, visible errors
without project changes, Windows paths and literal stems, and stale replies.

Awaiting QA. The `win-browser-backups` parity status remains unchanged.
