# Mixer name filter

The mixer toolbar's "Filter tracks" input only hides insert strips, across all
three docks. The master and the current utility stay visible. The selection
stays put, even when its insert strip is hidden. Arrow keys inside the filter
stop propagating before reaching mixer selection handlers.

The filter is local component state and is not saved. A blank trimmed query
shows every insert. Otherwise, names are matched by case-insensitive substring,
in their original order. When no inserts match, the mixer shows "No tracks
match."

The pure `matchingTrackIds(tracks, query)` helper lives in
`apps/desktop/src/features/mixer/track-filter.ts` and accepts items with an id
and a name.

Run the four helper tests from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/track-filter.test.ts
```
