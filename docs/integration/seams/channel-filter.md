# Channel filter

The channel rack grid has a **Filter channels** input. It matches channel
names by a case-insensitive substring after trimming the query. A blank
query shows every channel that passes the group filter, in the original
order. When a nonblank filter matches nothing, the rack shows
**No channels match.**

The filter only hides rows. The group filter still applies first. It does
not delete channels, and the selected channel stays selected even when its
row is hidden.

The query is local to `RackGrid` and is not saved. Unmounting the grid clears
it. Arrow keys inside the input stop before parent handlers, so editing the
filter keeps keyboard navigation in the field.

The pure `matchingChannelIds` helper is in
`apps/desktop/src/features/channel-rack/channel-filter.ts`.

Focused verification from `apps/desktop`:

- `pnpm test -- src/features/channel-rack/channel-filter.test.ts`
