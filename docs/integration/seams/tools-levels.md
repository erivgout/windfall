# Tools: playlist solo and audio clip levels

Both commands are registered in the Tools section after "Mute empty playlist
tracks" and have no shortcuts. Each command is one undo step when it changes
the project; if nothing differs, it dispatches nothing and creates no undo step.

- `tools.unsoloPlaylistTracks` ("Unsolo playlist tracks") sends one batch labeled
  "Unsolo playlist tracks", containing one `updatePlaylistTrack` per solo track
  with only `{ solo: false }`. Unsolo does not unmute or change track name, color,
  or height. Tracks that are not solo are omitted.
- `tools.resetAudioClipLevels` ("Reset audio clip levels") sends one
  `updateAudioClips` command for audio clips whose gain differs from 1 or pan
  differs from 0. Each patch contains only the differing fields, reset to gain 1
  and/or pan 0. Pattern clips, automation clips, and clips already at unity are
  omitted. Reset levels does not change fades or stretch, nor reverse, pitch,
  normalize, or mixer routing.

The existing frontend command bindings handle both operations; no new backend
command is required. Focused coverage is in
`apps/desktop/src/features/tools/actions.test.ts`, using the existing dispatch
mock and playlist seed helper to verify exact payloads and no-op behavior.
