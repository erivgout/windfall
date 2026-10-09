# Pattern filter

The playlist picker's Patterns section has a **Filter patterns** input.
It matches pattern names by a case-insensitive substring after trimming the
query. A blank query shows every pattern in its original order. When no
names match, the section shows **No patterns match.**

The filter only hides pattern rows. It does not rename patterns or change
the selected pattern, even when that pattern is hidden. The audio and
automation lists stay complete.

The query is local to the picker and is not saved. Unmounting the picker
clears it. Arrow keys inside the input stop before the pattern list's arrow
handling, so editing the filter does not move the selected pattern.

The pure `matchingPatternIds` helper is in
`apps/desktop/src/features/playlist/pattern-filter.ts`.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/playlist/pattern-filter.test.ts`
- `pnpm test -- src/features/playlist/playlist.test.tsx`
