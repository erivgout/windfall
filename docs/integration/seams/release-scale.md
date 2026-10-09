# Release scaling

The piano-roll lane header offers Half and Double after Short, Natural, and Long.
The entries are hidden unless the lane is showing release.

Each command sets release on the selected notes, or on every note when none are
selected, in one undo step through one `updateNotes` command. Half divides each
note's current release by two. Half of a short release stays short. Double
multiplies release by two and stops at a long release (1). Values are not rounded.
Notes without expression count as release 0.5, the instrument's own release.

Each note patch contains only expression, with only release replaced. Fine pitch
and modulation stay as they are, along with articulation, glide, and other
expression fields. Notes without expression use `DEFAULT_NOTE_EXPRESSION` with
release replaced. Short, Natural, and Long stay as they are.

Changes smaller than 0.001 are omitted. Each entry is disabled when there is no
editor context or nothing to change. Clicking reads the current notes again;
an empty update list or missing editor context sends no command.

The pure `releaseScaleUpdates(notes, factor)` helper keeps the given order and
does not mutate the input. `scaledRelease(release, factor)` accepts `"half"` or
`"double"`.

Focused coverage: `apps/desktop/src/features/piano-roll/release-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/release-scale.test.ts
```
