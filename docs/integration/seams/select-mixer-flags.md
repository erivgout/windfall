# Select mixer tracks by mute or solo

"Select muted tracks" (`mixer.selectMutedTracks`) and "Select solo tracks"
(`mixer.selectSoloTracks`) appear in the mixer panel menu immediately after
"Select tracks routed here", muted first and solo second. They have no shortcuts
and are enabled when at least one project mixer track has the matching flag.

These commands only change which strips are selected. Mute and solo stay as
they are, as do volume and routing. Neither command dispatches a project command.
If none match, the selection stays.

Matching tracks are selected in visual mixer order. The first match becomes
the selection anchor and the selected track in the UI store. Per-strip menus
stay as they are.

The pure `mutedTrackIds(tracks)` and `soloTrackIds(tracks)` helpers in
`apps/desktop/src/features/mixer/select-flags.ts` return matching ids in the given
order. A track that is both muted and solo appears in both lists; a track with
neither flag appears in neither.

Run the four helper tests from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/select-flags.test.ts
```
