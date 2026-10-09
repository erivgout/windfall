# Utility commands

Windfall registers three global command palette actions under **Tools** through
`apps/desktop/src/features/layout/register-actions.ts`. Their implementation is
in `apps/desktop/src/features/tools/actions.ts` and reads the current project at
execution time, regardless of selection.

- **Set all audio clips to tape** (`tools.setAllAudioClipsToTape`) sends one
  `updateAudioClips` command with `stretch: { mode: "tape" }` for each changed
  audio clip. Explicit tape and missing stretch (the legacy tape default) are
  omitted.
- **Set all audio clips to spectral** (`tools.setAllAudioClipsToSpectral`) sends
  one `updateAudioClips` command with
  `stretch: { mode: "spectral", ratio: 1, quality: "standard", formants: false }`.
  Clips already matching all four fields are omitted.
- **Mute empty playlist tracks** (`tools.muteEmptyPlaylistTracks`) sends one
  labeled `batch` of `updatePlaylistTrack` commands with `muted: true`. Any clip
  makes its track occupied, including muted, pattern, audio, and automation
  clips. Occupied tracks and tracks already muted are omitted.

Each command dispatches once, producing one undo step. If no changes are needed,
it dispatches nothing and creates no undo step. The commands patch only stretch
or track mute state; none delete channels, clips, or samples. Channel deletion
and live MIDI are not included.

The focused desktop test checks Tools registration and cleanup, exact batch
payloads, unchanged items, all spectral fields, all clip types for occupancy,
and no-op behavior, including an empty playlist. Run from `apps/desktop`:

```powershell
pnpm test src/features/tools/actions.test.ts
```
