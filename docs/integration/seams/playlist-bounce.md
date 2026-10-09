# Playlist bounce

Windfall's **Bounce selected clips** playlist action is enabled when clips are
selected and appears in the playlist and clip context menus. Pattern and audio
clips may be selected together. The earliest selected start and latest selected
end define a linear song region; the first selected track in playlist track order
receives the result.

`Backend.bounceSelectedClips` calls the worker-backed Tauri
`bounce_selected_clips` command and `Session::bounce_selected_clips`. The session
copies the project and sample pool, solos and unmutes the selected playlist
tracks in that copy, and clears solo on every other playlist track. **The live
solo state is not used as the render switch.** Live playlist track solo and mute
flags remain unchanged. The existing `render_streaming_checked` offline renderer
plays that copy's region with no extra tail. Unselected clips are muted in the
copy, including overlapping clips on a selected track. Selected clips
contribute according to ordinary playback, including clip mute,
patterns, audio, routing and automation.

The worker collects the stereo buffer, writes a persistent Float32 WAV under the
app's recordings directory, and decodes it through the existing sample cache.
The same `audio_clip_commands` builder used by `add_audio_clip_from_sample`
creates the clip. Its tick length is explicitly the selected span, including
when tempo automation changes the render's duration. The new clip is unmuted
and uses Direct output because the buffer already includes Master processing.
If an arrangement is active, the new clip is enrolled in that arrangement in
the same transaction as its creation.

After checking document identity, edits, source handles, pending sample loads and
plugin revision, the session prepares and publishes one sample-edit batch:
add the sample, add its audio clip, and mute the selected source clips.
**Source clips are muted rather than deleted.** Undo restores their previous mute
states and removes the new sample and clip together; the render file remains
available for redo and saved projects. Empty or stale selections, renderer
errors, incomplete renders, dropped audio clips and preparation failures cause
no document changes. Failed transactions remove their derived WAV.

Rendering, file I/O and preparation happen on the worker, outside the document
lock. The audio callback gains no allocation, lock or I/O work. Playlist gestures,
normalize, solo playback rules and delay modulation are unchanged.

The desktop mock exposes the same method. Browser previews refuse it because
they have no offline renderer; tests may inject a render-file fixture to exercise
the real document batch and failure path without another renderer.

Coverage: `session::tests::playlist_bounce` checks nonzero-region pattern/audio
rendering, track isolation, exact geometry, one-step undo/redo and stale admission.
`windfall-engine --test playlist_bounce` independently checks the streaming
renderer's nonzero span and pattern/audio track isolation.
`features/playlist/bounce.test.ts` checks one successful batch, source mute,
destination selection, unchanged track flags, empty selection and failed render.
