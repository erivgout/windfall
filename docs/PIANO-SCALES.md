# Piano-roll scales and stamps

The piano toolbar's **Scale** menu chooses a root and a musical scale. **Highlight scale** and **Snap pitches to scale** are independent, opt-in settings. Both start off. With highlighting off, the existing black-key shading and C octave boundaries stay unchanged. With it on, notes outside the scale have darker rows in either theme; a stronger boundary sits just below every chosen root. The toolbar names that root and scale.

Definitions in `scales.ts` are authored musical intervals: major, natural/harmonic/melodic minor, the seven diatonic modes, major/minor pentatonic, blues, whole tone and chromatic. Melodic minor uses its ascending intervals in both directions. No proprietary chord, scale or groove files are read or bundled.

## Pitch snap

Drawing and painting choose the nearest allowed MIDI key, inside 0–127. Equal distances choose the lower key. Hold **Alt** while drawing, painting, dragging or placing a stamp to bypass both pitch and time snap. Shift continues to add to selection. For keyboard edits and paste, turn off **Snap pitches to scale** to bypass it.

Groups move rigidly: the grabbed note anchors a pointer drag; the lowest note anchors a keyboard nudge, transpose or paste. The anchor snaps and every voice receives the same semitone offset. Other voices may remain outside the scale. This keeps chord intervals, unisons, note order and time spacing intact instead of rounding each voice into a smaller chord. The bounds of the whole group constrain its anchor. Keyboard pitch nudges advance to an allowed anchor in the requested direction; at the boundary they stop. A group with no permitted translation stays put, and a paste with no permitted anchor is refused in full.

Enabling a scale, changing the root, highlighting, time-only moves, resizing, duplication to the right and changing velocity/pan do not retune existing notes. Pitch changes occur only through an intentional pitch edit or new-note placement. The separate Rust note-tools operations retain their explicit transforms.

## Stamp placement

Open **Stamp**, choose a chord or an ascending/descending scale, and move into the note grid. The complete pattern previews before any edit. One left click places the pattern and returns to the previous tool. Placement works over occupied notes as well as empty space. **Escape**, **Cancel**, right-click, a tool change or losing window focus cancels without an edit. Moving out of the grid hides the preview; moving back resumes it.

Chords are simultaneous, in root position. Available chords include major/minor/diminished/augmented triads, sus2/sus4, power chords, sixths, seventh variants and major/minor ninths. Scale patterns include an octave endpoint. Ascending patterns begin on the clicked root and end an octave above it; descending patterns start an octave above the clicked root and end on it.

Every note uses the current drawing length and velocity, with pan 0, matching ordinary drawing. Scale notes are separated by exactly that length. The time grid aligns the start unless Alt is held. Pitch snap, when enabled, aligns the stamp's root and preserves the pattern's original intervals; it does not rewrite the selected chord/scale to fit the highlight scale.

Every note must fit MIDI keys 0–127 and the maximum pattern tick bound. If any note fails, the entire stamp is refused, the reason appears beside Cancel, and the stamp remains armed for a corrected click. It never clips, folds, shortens or places a partial pattern. Extending the pattern to the next bar and adding all notes use the existing `addNotes` transaction, producing one undo step. The new notes become the selection. Undo/redo and saving/opening use the existing Rust commands and project format.

Armed previews are tied to the current lane, its notes and document generation. Lane switches, note changes, pattern-length/signature changes, project replacement and editor disposal cancel them. A project replaced between pointer-down and pointer-up receives no stamp command. Late responses cannot apply created-note selection to another project or lane.

## Preferences and rendering

Root, scale and both options are retained in the existing `windfall.pianoRoll` UI preferences. Load accepts only known scale IDs, roots 0–11 and booleans. Only the documented preference fields are merged, so malformed storage cannot replace methods, note defaults, selection or clipboard state. Existing version-1 preferences remain compatible. No project fields, IPC types or Rust commands were added.

`scaleRows` configures the existing `TimeGridView.setRows` API. Canvas 2D, WebGL2 and WebGPU consume the same grid batch and provisional note scene. There are no renderer/shader changes, and playlist/automation row configuration is untouched.

## Validation

[Stamp lifecycle repairs](PIANO-STAMP-REPAIRS.md) records the delayed-choice cancellation and pointer-presence regressions, fixes and focused validation on the integrated base.

Focused tests cover all named scales and roots, deterministic nearest-key ties and MIDI limits, group policy, unchanged default row styles, malformed/retained preferences, stamp rhythms/dynamics and all-or-nothing bounds. Integration tests drive the real piano session and shared Rust WASM `addNotes`/`updateNotes`/undo/redo/file commands, including draw/paint bypass, chord drags, keyboard pitch moves, paste, previews, atomic pattern extension, selection, save/open and stale-session cancellation. Toolbar tests exercise accessible menu choices and Cancel.

The pinned base precedes the parent's generated library bindings and simulator exports. Validation uses those parent-generated artifacts from `4b0d509d` locally; the scale/stamp source commit excludes all bindings and WASM artifacts. No Cargo build is needed for these UI changes. Parent integration owns artifact regeneration and parity updates.

Validated in this worktree:

- `pnpm exec vitest run src/features/piano-roll src/lib/canvas/grid.test.ts src/lib/canvas/time-grid-view.test.ts src/lib/canvas/renderer-parity.test.ts src/features/playlist/session.test.ts --maxWorkers=4`: 265 tests in 12 files passed, including Strict Mode session reattachment.
- `pnpm exec eslint src/features/piano-roll` and `pnpm build`: passed. The build includes TypeScript checking and reports the existing large-chunk advisory.
- T3 preview confirmed both menus and scale shading. After its preview host disconnected, a local Chromium check exercised menu selection, real pointer previews/placement, selection, atomic extension, undo/redo, Escape/right-click cancellation, invalid MIDI placement and retained preferences after reload, without page errors. Stamp arming waits until the menu exit completes so keyboard cancellation reaches the grid.
- Real Canvas 2D and WebGL2 views accepted scale row styles, restored default styles and rendered complete provisional chord scenes. The environment supplied no WebGPU adapter; WebGPU remains covered by the shared grid/scene path and existing renderer arithmetic parity tests. Physical WebGPU and the native Tauri window were not exercised. Local browser evidence is in ignored `target/piano-qa/`.

This implementation deliberately provides root-position chords, single-octave scale patterns and the rigid-group snap policy above. Inversions, custom stamp-file import, independent voice revoicing and proprietary preset catalogs are outside these bounded workflows.
