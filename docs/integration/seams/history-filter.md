# History filter

The history popover has a **Filter history** input above the undo list.
It matches labels by a case-insensitive substring after trimming the query.
A blank query shows every row; no matches shows **No steps match.**

The filter only hides rows. Runs retain their order and original first and
last indexes. Choosing a shown row still jumps to that step; choosing a
folded run jumps to its original last step, and unfolded steps retain their
individual jump targets. Filtering does not change the undo history.

Closing the history clears the filter, including when the shared UI store
closes the popover. The query is local component state and is not saved.

The pure `matchingRuns` helper is in
`apps/desktop/src/features/history/history-filter.ts`. `HistoryList` filters
the grouped runs and the initial **Project opened** row together.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/history/history-filter.test.ts`
- `pnpm test -- src/features/history/history-list.test.tsx`
