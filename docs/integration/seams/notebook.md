# Song notebook

This is a partial paged notebook implemented as saved plain text. The playlist
header mounts `NotebookControls` beside Project info. The editor selects a page,
edits its title and body, adds pages up to eight, and removes the selected page.
Save notebook dispatches one `replaceNotebook` command containing the whole book,
so edits across multiple pages are one undo step. No author, genre, or comments
are changed.

`Project.notebook` contains `Notebook { pages: Vec<NotebookPage> }`; each page has
`title` and `body` strings. Serde uses camelCase, defaults a missing notebook to
an empty book, and omits an empty book from project files. Existing projects open
without creating pages or becoming dirty. Titles are trimmed before command
validation and stored with a limit of 128 UTF-8 bytes. Bodies preserve all
whitespace and line breaks and have a limit of 16,384 UTF-8 bytes. A ninth page
or oversized text returns `CommandError::Invalid` without changing the document
or history. `Project::check` also validates saved books on load.

The editor displays one temporary blank page for an empty book. Opening the
panel and selecting pages do not dispatch commands. Editing that blank page
materializes it in the draft; adding from it keeps it and creates a second page.
Removing the last page leaves an empty draft, which Save notebook can persist.
All changes wait for the explicit save action. The panel validates UTF-8 byte
limits across all pages, including pages that are not selected. Controls are
disabled during a save and repeated submissions are ignored. Failed saves keep
the draft available for retry. Document updates, undo/redo, and project
replacement reset the draft from the mirror.

The command lowers to one reversible notebook edit. A dedicated touched flag
and optional notebook patch field carry updates to the desktop mirror, including
an explicit empty book when undo or removal clears it. The project, command,
patch, and notebook TypeScript bindings and barrel exports were updated by hand;
`gen-bindings` was not run. The requested Rust filter also runs the two ts-rs
notebook export tests.

There is no HTML rendering, script host, playback follow, or engine integration.
Text resembling markup is ordinary text in the textarea. New UI labels use
Windfall's notebook terminology.

Focused verification:

- Git Bash, repository root: `source scripts/msvc-env.sh && cargo test -p windfall-project --lib notebook`.
- `apps/desktop`: `pnpm test src/features/notebook/panel.test.tsx`.
- `apps/desktop`: `pnpm test src/features/notebook/panel.test.tsx src/lib/store/patch.test.ts src/features/playlist/playlist.test.tsx` (48 tests passed).
- `apps/desktop`: `pnpm exec eslint src/features/notebook src/lib/store/patch.ts`.

The full desktop `pnpm typecheck` remains blocked by errors in other features
and existing bindings (including analysis backend methods, instrument parameter
unions, and stage/piano-roll fixtures). It reports no notebook or patch-store
diagnostics.

Rust tests cover default omission, round trips, exact multibyte byte caps,
title trimming and body preservation, atomic page/text rejection, validation on
load, whole-book undo/redo, clear patches, metadata isolation, and no-op redo
preservation. Desktop tests cover the empty editor, one command across pages,
selection, add/remove limits, clearing the last page, visible and hidden page
validation, pending saves, patch merging, undo clearing, and project replacement.
