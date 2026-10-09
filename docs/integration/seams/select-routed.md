# Select tracks routed here

`mixer.selectRoutedHere` ("Select tracks routed here") appears immediately after
Reset levels in the mixer menu and both insert and master strip header menus.
It has no shortcut. It is enabled when a mixer track is selected and at least
one other track has that track as its main output.

`routedTrackIds(tracks, target)` in
`apps/desktop/src/features/mixer/select-routed.ts` returns matching track ids
in the given order. It omits the target itself and tracks with null outputs.
A send alone does not select a track.

This only changes which mixer tracks are selected. The action replaces the
mixer selection with those ids, sets the selection anchor to the first id,
and selects that first track in the UI store. It does not dispatch a command
or change routing, outputs, or sends.

Focused coverage: `apps/desktop/src/features/mixer/select-routed.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/select-routed.test.ts
```
