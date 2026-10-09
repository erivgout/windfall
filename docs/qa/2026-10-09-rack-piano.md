# Piano QA, 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Scope: this resumed worker owns `apps/desktop/src/features/piano-roll/**`.
Channel-rack QA belongs to the rack worker; this report makes no rack completion claim.

## Verified checks

`bun run test src/features/piano-roll/editable-ghosts.test.ts src/features/piano-roll/pointer-tools.test.tsx src/features/piano-roll/drum-paint.test.tsx --maxWorkers=1`

Result: **3 files, 31 tests passed**, 17.63 seconds. `bunx eslint src/features/piano-roll` exits 0.
The existing `2026-10-09-frontend-typecheck-recheck.log` records a clean `tsc -b`;
the full frontend worker owns final combined typecheck and test verification.

The broader piano run was canceled to avoid duplicating the full frontend run.
These targeted checks are not a claim that the entire application passes.

## Repairs and acceptance

- Editable ghost input now resolves a source lane for either ordinary left edits or right-button erase. The former left-only guard made right-click erase miss ghost notes. Its ownership guards still protect stamp choices and active gestures.
- Four real-grid-input tests verify source-lane erase and move, unchanged current lane, no duplicated notes, one undo entry and undo restoration, unchanged viewport, read-only ghost protection, and stamp cancellation over a ghost without lane switching or document changes.
- Pointer tool tests verify toolbar selection, mute restoration and undo/redo, canceled/no-op gestures, slice length conservation, expression preservation, curve cropping at the command boundary, zoom fitting/restoration/cancellation, and held scrub seek/audition release on pointerup, blur, Escape, pointercancel and lost capture.
- Drum Paint tests verify add/delete strokes, off-grid starts, revisits, row identity, preceding tails, cancel, normal Draw/Paint behavior, toolbar/keymap access and project replacement.
- Piano test fixtures now explicitly select step snap. Their arithmetic expects 240-tick cells, whereas the shared session default is now bar snap. The original focused run exposed seven fixture failures from this mismatch. The production default is unchanged.
- The ghost canvas fixture now supplies a grid theme; otherwise its ghost-hit path throws before reaching the intended regression assertions.
- Previous resumed edits repair nullable bindings/action signatures and `prefer-const` in note curves; preserved without reverting others' work.

## Parity recommendation

Promote `win-piano-paint`, `win-piano-mute`, `win-piano-slice`,
`win-piano-zoom`, `win-piano-playback` and `win-piano-ghost-notes` once the
root accepts the combined frontend/native checks. The focused tests exercise
the complete row descriptions through the shared WASM-backed document harness.
The slice curve subcase checks its payload rather than claiming hosted DSP acceptance.

Retain existing done rows for draw, delete, select, stamp, keyboard, scale
highlighting, snap and the established selected-note tools, subject to the full
suite results. This worker did not independently rerun all those existing tests.

Do not promote `win-piano-event-editor`, `win-piano-slide-porta`,
`win-piano-note-colors` or `win-piano-lfo` merely because their UI exists:
their tracked hosted/live expression, plugin/MIDI pitch, live MIDI output,
continuous note expression and recorded-event limitations remain. Randomizer,
pattern markers/meters and waveform helper need their own regenerated native/WASM
and end-to-end evidence. Waveform control arithmetic alone does not verify audio
loading/rendering, and command-payload checks alone do not verify native execution.

No parity file was edited by this worker; the root owns the consolidated ledger.

## Canvas layer scaling acceptance

The full-suite inventory exposed eight failures in `layer-scaling.test.tsx`.
The first was a stale label (`Note velocities` versus the current
`Note velocity values`). Since fixture construction threw before returning its
cleanup handle, subsequent cases leaked the action registry and failed with
`Action "file.importMidi" is registered twice`. The fixture now installs its
cleanup immediately after app/session creation, expands it once the canvas
view exists, and invokes it during `afterEach` even if setup fails.

The painter/input assertions were preserved: six UI-scale/DPR/viewport
combinations check rendered marker/cap alignment, actual grid hit testing,
backing-size caps and visible velocity edits; further tests exercise real end
marker commands and observer-only logical resizing under unchanged backing caps.
No product source or render/input expectations were changed.

`bun run test src/features/piano-roll/layer-scaling.test.tsx --maxWorkers=1`:
**8 tests passed**, 7.09 seconds, default timeout. Evidence:
`2026-10-09-piano-layer-recheck.log`; original reproduction retained in
`2026-10-09-piano-layer.log`. ESLint on the test exits 0.

The additional note-tools dialog failure was a stale assumption that quantize
always shows `Grid (ticks)`. Its default now uses musical units; custom ticks
requires choosing that unit. The test now verifies both zero musical divisions
and zero custom ticks disable Apply and report invalid fields, valid divisions
reenable Apply, and cancellation still preserves the exact project/history
state. Product behavior and the original cancellation/invalid-input assertions
are preserved.

Final combined command:
`bun run test src/features/piano-roll/note-tools.test.tsx src/features/piano-roll/layer-scaling.test.tsx --maxWorkers=1`
**23 tests passed**, 12.40 seconds, default timeout. Log:
`2026-10-09-piano-repairs-final.log`. ESLint on both repaired test files exits 0.
