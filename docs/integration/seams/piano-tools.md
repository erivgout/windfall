# Piano-roll pointer tools

All four tools are selectable in the piano-roll toolbar, tool menu, and action
registry. They extend the existing `Tool`, pointer intent, and editor gesture
unions; gesture switches retain exhaustive `never` checks. Existing draw,
paint, select, and erase behavior remains in its original path.

## Existing command wiring

- **win-piano-mute** — `pianoRoll.toolMute` (M). Clicking a note toggles its
  velocity between zero and its remembered previous value via `updateNotes`.
  Each completed gesture dispatches one command/history step; cancel and empty
  clicks dispatch none. Undo/redo use the existing project history. Prior
  velocities are retained by pattern/channel/note while the project is open
  and cleared on project replacement. A zero-velocity note with no remembered
  value unmutes to the current draw velocity (0.8 if that is also zero).
- **win-piano-slice** — `pianoRoll.toolSlice` (C). Clicking inside a note splits
  it at the rounded, snapped pointer tick (Alt bypasses snap). The original id
  stays on the left; `updateNotes` and `addNotes` run in one `batch` labeled
  “Slice note.” Both positive lengths sum exactly to the original length;
  start/end boundary clicks do nothing. Key, velocity, pan, and scalar
  expression are copied. Expression curves are cropped and normalized for
  each half through `setNoteExpressionCurves` and `addNotesWithCurves`, keeping
  held segments and scaling the bend of shortened segments.
- **win-piano-zoom** — `pianoRoll.toolZoom` (Z). The existing marquee overlay
  previews a rectangle. Release sets the viewport to its tick/row bounds,
  subject to existing canvas zoom and scroll limits. Reverse drags work.
  A second click or Escape restores the previous scroll and zoom, retaining
  the canvas's current dimensions. Cancel abandons an unfinished rectangle.
  Zoom does not create project history.
- **win-piano-playback** — `pianoRoll.toolPlayback` (Y). Press and held movement
  update the existing local playhead and call `seek` from the transport store,
  which invokes `backend.transportSeek` / Rust `transport_seek`. The seek is
  rounded and bounded to the edited pattern. It uses the active transport's
  timeline, as the existing seek API does. Scrub auditions the note on the
  pointer's key at that tick through existing note-on/off calls, excluding
  silent and slide notes. Release, pointer cancellation/lost capture, window
  blur, Escape, tool change, and detach stop auditioning. Hover after release
  does not seek. Scrub does not create project history or start global playback.

## Rust seam

No Rust command is needed for slice, viewport zoom, or transport seek.
The current `Note`, `NoteInit`, and `NotePatch` bindings have **no mute flag**.
Mute therefore uses a real zero-velocity edit rather than sending an unknown
command or adding unsupported fields. The zero velocity persists in the
project, but its remembered pre-mute value does not survive closing/reopening
the project. A dedicated persisted per-note mute flag, independent of velocity
and honored by all playback/MIDI paths, still requires a Rust model field and
note-command patch support (plus regenerated bindings). This implementation
does not modify those files or claim that seam is complete.

## Focused verification

`apps/desktop/src/features/piano-roll/pointer-tools.test.tsx` covers tool
selection, mute toggle/history/cancel, split-length conservation and
expression/curve preservation, rectangle zoom/restoration/cancel, and scrub
seek/release on mouseup, blur, Escape, pointer cancellation, and lost capture.
The focused file passes all 15 tests. Expression/curve payloads are verified
at the command boundary because the checked-in WASM simulator predates those
fields/commands; split/history, mute, zoom, and scrub run through the existing
test app. A no-emit TypeScript check reports no piano-roll diagnostics; the
desktop check still has errors in other features outside this task's ownership.
