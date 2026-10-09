# Project info

This adds partial project metadata coverage for roadmap T5. The playlist header
mounts `ProjectInfoControls` from `features/project-info`, which opens a popover
with Author, Genre, and Comments. It dispatches one `updateSettings` command per
Save project info action, containing only changed fields. Controls are disabled
while that dispatch is pending; failed saves retain the draft for retry. Project
replacement or a metadata update (including undo/redo) resets the draft from the
document mirror.

`ProjectSettings` stores `author`, `genre`, and `comments` as strings. Serde uses
camelCase, defaults missing fields to empty, and omits empty metadata on save.
`SettingsPatch` accepts an optional string for each field. Missing values leave
the corresponding setting unchanged; an empty or whitespace-only string clears
it. `UpdateSettings` trims leading and trailing whitespace, preserves internal
line breaks, and validates UTF-8 byte lengths after trimming:

- Author: 256 bytes.
- Genre: 128 bytes.
- Comments: 16,384 bytes.

An oversized field returns the existing `CommandError::Invalid` style without
applying any part of the patch. All metadata in one patch is one undo step, and
an empty patch leaves settings, dirty state, history, and redo unchanged. Desktop
validation uses UTF-8 bytes too; the Rust command remains authoritative.

The two TypeScript bindings were updated by hand with optional string fields so
older snapshots and existing fixtures remain valid. The binding generation command
was not run; the requested Rust test filter also runs existing ts-rs export tests
for settings types.
The panel adds Windfall labels only. This stores project metadata, not notebook
pages, HTML, or tags embedded in exported audio files.

Focused verification:

- Git Bash, repository root: `source scripts/msvc-env.sh && cargo test -p windfall-project --lib settings`.
- `apps/desktop`: `pnpm test src/features/project-info/panel.test.tsx`.

Rust tests cover omission/defaults, serialization, exact byte limits with
multibyte text, atomic rejection, trimming, clearing, comments undo/redo, one
history step, and empty-patch redo preservation. Desktop tests cover the connected
panel dispatching one patch, omitted values, changed fields only, byte validation,
pending-save controls, duplicate submits, clearing, and project replacement.
