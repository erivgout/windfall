# Project name

The project-info panel starts with a Name field initialized from `settings.name`.
`projectNameChange(current, draft)` trims surrounding whitespace, omits an
unchanged name, refuses a blank name, and rejects names longer than 256 UTF-8
bytes after trimming.

Save project info includes a changed name with the other changed project-info
fields in one `updateSettings` patch, so they are saved in one undo step.
Unchanged name, author, genre, and comments are omitted. An invalid name shows a
field error and blocks the entire save, even if other fields have changed.
Project replacement and settings changes, including undo/redo, reset the draft
from the document mirror.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/project-info/project-name.test.ts`
- `pnpm test -- src/features/project-info/panel.test.tsx`
