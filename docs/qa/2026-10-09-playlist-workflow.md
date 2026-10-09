# Playlist bounce workflow QA — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Scope: `win-playlist-consolidate`, the implemented **Bounce selected clips**
action. This is not a verification claim for every playlist feature.

Three defects were repaired: unselected overlapping clips on selected tracks
were included in the print, a complete mixer print was routed through Master
again on playback, and prints were omitted from the active arrangement.
The project copy now mutes unselected clips without breaking saved metadata
references; the installed print uses Direct output; playlist insertion enrolls
the active layout as part of the same undo transaction.

Native regression coverage (`session::tests::playlist_bounce`):

- Pattern/audio spans starting after zero, source-track isolation, destination
  ordering, exact clip geometry and nonzero rendered output.
- One history entry, selected-source mute, unchanged live track flags, exact
  undo and redo, and retention of the audio source for redo.
- Empty/missing selection and an edit racing the completed render produce no
  partial document update.
- Unselected overlapping clips on a selected track are muted only in the copy.
- Active-arrangement enrollment, Direct output, and complete undo/redo.
- Rendering through Master gain 0.25 before and after bounce produces matching
  playback samples (maximum permitted absolute difference 0.00001), preventing
  a second application of Master gain.

Execution evidence is pending the single native validation owner's run. The
targeted command is `cargo test -p windfall-desktop --lib playlist_bounce -j 1`.
The independent renderer target is
`cargo test -p windfall-engine --test playlist_bounce -j 1`.

Eligibility: the implemented selection-bounce subset of `win-playlist-consolidate`
can be recorded as QA verified after these native tests and its frontend
workflow tests pass on the repaired source. The broader parity row remains
in progress: a track header provides **Select clips on this track**, followed
by this action, but no separate consolidate-track command was verified.
No full-feature completion claim is made. Other playlist IDs require their
own evidence. In particular,
`win-playlist-arrangements`, `win-playlist-track-groups`,
`win-playlist-clip-groups`, and `win-playlist-instrument-audio-tracks` retain
their independent layout, gesture, or routing scope.
