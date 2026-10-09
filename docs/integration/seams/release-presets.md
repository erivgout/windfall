# Release presets

The piano-roll lane header offers Short (0), Natural (0.5), and Long (1)
after the pan preset group and before Write LFO. The entries are hidden unless
the lane is showing release. Natural is the instrument's own release.

Each preset sets release on the selected notes, or on every note when none are
selected, in one undo step through one `updateNotes` command. Each note patch
contains only expression, with release replaced. Fine pitch and modulation stay
as they are, along with articulation, glide, and any other expression fields.
Notes without expression use `DEFAULT_NOTE_EXPRESSION` with release replaced.

Notes already at the preset are omitted; a difference smaller than 0.001 counts
as a match. Each entry is disabled when there is no editor context or nothing to
change. An empty update list or missing editor context sends no command.

The pure `releasePresetUpdates(notes, preset)` helper preserves the given order
and does not mutate the input. Notes without expression count as release 0.5.
The menu subscription includes release alongside velocity and pan so the entries
stay current as notes change.

Focused coverage: `apps/desktop/src/features/piano-roll/release-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/release-presets.test.ts
```
