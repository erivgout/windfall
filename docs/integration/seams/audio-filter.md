# Audio picker filter

The Audio section shows a local, unsaved "Filter sounds" field when the project
has samples. It matches sample names without regard to case, preserves their
order, and shows every sound for a blank query. If no names match, it shows
"No sounds match." Projects without samples keep the existing empty message and
do not show the filter.

The filter only hides sound rows. It does not remove samples or change the brush.
The pattern and automation lists are not filtered by it. Arrow keys inside the
field stop propagating so parent handlers cannot move focus or act on them.

Matching reuses `matchingPatternIds` from
`apps/desktop/src/features/playlist/pattern-filter.ts`. The sample-shaped matching
tests are in `apps/desktop/src/features/playlist/audio/picker-filter.test.ts`.
