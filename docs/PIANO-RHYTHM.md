# Selected-note rhythm tools

Choose **Note tools** or a selected-note context-menu/command-palette action.
Select notes in one lane first. Every Apply uses the same Rust `TransformNotes`
calculation in native and browser WASM. Options, Cancel, close and Escape do not
change the project. There is no live note/audio preview. The dialog explains
the exact policy before Apply; Rust validates the whole result before edits.

All tools preserve unselected notes. One Apply creates one undo entry, including
any pattern extension. A no-op preserves dirty state, redo and the ID allocator.
The captured revision, project generation, lane and selected IDs must still
match. Rust also compares each captured note's properties against the lane.
New hits receive deterministic fresh IDs and join the result selection;
removed IDs are pruned. Removing the whole selection leaves it empty. Undo and
redo use the existing selection policy described in [PIANO-TOOLS](PIANO-TOOLS.md).

## Custom Chop

Uniform **Chop** retains its original absolute grid behavior. **Custom Chop**
uses Origin, Period and 1–64 increasing boundaries within the period. The first
boundary must be 0; each boundary must be below Period. Boundaries repeat in
both directions from Origin. For example Origin 100, Period 480, boundaries
`0, 120, 360` give lines at 100, 220, 460, 580, 700, 940, and also before 100.
Tick zero is the earliest legal output start; no notes are added before the
source start or beyond its original end.

Enter comma-separated `tick`, `tick:gate%`, or `tick:gate%:velocity%` values.
Omitted accents default to 100%. Gate is greater than zero through 100%; velocity
is 0–400%. Gate scales each retained piece's length, including partial first
and last pieces, rounds half ticks upward, and keeps at least one tick. Velocity
multiplies the source's velocity and clamps at 1. Pitch and pan stay unchanged.
The first retained piece keeps the original ID; subsequent pieces get fresh IDs.

With `0, 120:50:150, 360:100:50`, a source starting at 50 with length 600 and
velocity 0.5 becomes `(start, length, velocity)`:
`(50,50,.25), (100,120,.5), (220,120,.75), (460,120,.25), (580,70,.5)`.
This is a nonuniform repeating rhythm with a gated middle piece. No proprietary
preset files or rhythms are consumed.

## Arpeggiate

Group selected notes by exactly equal start tick. Each group becomes its own
repeated arpeggio. Build an ascending pitch ladder from the original voices plus
0–7 higher octave copies (Octave span 1–8). Sort by resulting MIDI key, then
original ID; equal pitches remain separate voices with their own dynamics.
Ascending traverses that ladder; Descending sorts pitch downward with ID ties
still ascending. Alternating traverses upward then downward without duplicating
turnaround endpoints. Three voices therefore play `1,2,3,2,1,2,3,2…`; one voice
repeats itself. Every hit retains its chosen voice's velocity and pan.

Grid is the rate between hits. Gate sets hit length to rounded `rate × gate`,
at least one tick. **Original span** uses the longest source note in each chord,
requiring an exact number of complete rate slots. A 720-tick chord at rate 120
produces six hits; rate 200 is refused. Shorter source voices can sound later
within that longest-voice span. **Fixed repetitions** replaces the original
span with 1–64 complete traversals; it may shorten or extend it. A traversal
contains the ladder's voice count, or `2 × count − 2` for Alternating (one for
a single voice). Every full rate slot must fit the pattern limit, even when
the gate is shorter. No partial final slot or MIDI octave clipping is allowed.

The first emitted occurrence of each original voice keeps that voice's ID,
including when Descending first emits its higher octave. Further occurrences
get fresh IDs. Original voices not reached within Original span are removed.
This rewrites chords into repeated hits; Strum still delays original voices once
and leaves their lengths unchanged. Scale snapping does not retune these explicit
Rust pitches.

## Flam

Interval is 1–960 ticks. Before places one grace start at `original start −
interval`; After places it at `original start + interval`. The grace length is
the smaller of the interval and the source length. Grace velocity is 0–100% of
the original velocity; key and pan are retained. Original notes and IDs remain
exactly unchanged. Before finishes no later than the original start. After can
overlap a sustained original; the original is never shortened to avoid that
overlap. Any negative grace start or end beyond tick 245760 refuses all hits.
Applying again adds another grace hit to every currently selected note.

## Rhythm reshaper

Grid defines cells of integer ticks from Origin. Cell index is
`floor((start − origin) / grid)`; Period is 1–64 cells, and Phase is a zero-based
index below Period. Match cells whose index modulo Period equals Phase, using
Euclidean modulo even before Origin. Cells include their left boundary and
exclude their right; off-grid onsets inside a matched cell are included.

- **Remove** deletes all matched selected notes. Period 1 / Phase 0 removes
  the entire selection and is a valid atomic edit.
- **Add** copies matched notes at the signed Offset, retaining key, length,
  velocity and pan. Offset must be nonzero. Skip a destination hit when its
  start, length, key, velocity and pan already match a selected hit; IDs are
  irrelevant to this comparison. Unselected hits are never consumed or used
  for suppression. Added hits join the selection, so a later Apply may add
  further hits if their cells match.
- **Shift** moves matched originals by Offset with lengths/properties/IDs
  retained. Offset zero, no matched cells, and fully suppressed additions are
  meaningful no-ops. Repeated Shift applies the rule to the current onset cells.

With Origin 0, Grid 240, Period 2 and Phase 0, onsets 0 and 490 match; onset 250
does not. Add at Offset 240 adds hits at 240 and 730. Shift at Offset 30 moves
the matching starts to 30 and 520. Remove leaves only the note at 250.

## Limits and validation

Selections and outputs are capped at 16,384 notes. All starts, lengths and full
slots must fit 0–245760 ticks. Period/rate/grid and note lengths must be positive
integer ticks. Invalid, duplicate or unordered Chop boundaries, oversized
patterns, nonfinite values, invalid velocities/pan, invalid MIDI/octave ranges,
ID exhaustion and any out-of-bounds note refuse the whole command. Inputs
normalize identical duplicate IDs; conflicting snapshots fail. Input order does
not change the result, allocator order or lane ordering.

No project format fields, DSP note identities, sampler settings, scale/stamp
preferences, renderer changes or proprietary imports are introduced. The parent
owns generated bindings/WASM and parity integration. These workflows address
`win-piano-chop`, `win-piano-arpeggiate`, `win-piano-flam`, and `win-piano-claw`;
broader randomizer, LFO-event, riff and slide/portamento contracts remain separate.

## Verification

Windows validation used one Cargo build job, worktree-local
`target/piano-rhythm-native` and `target/piano-rhythm-bindings`; WASM used the
same worktree's `target/sim`. No desktop native build or full workspace/native
suite was run. UI workers were capped at two.

- `cargo test -p windfall-project --test piano_tools`: 21 passed. New cases
  check exact partial/accented notes, tied voice ordering, independent chord
  groups, octave/repetition/gate rules, explicit grace placement, Euclidean
  rhythm cells, suppressed duplicates, empty selected output, deterministic
  IDs, unchanged unselected notes/lane, one undo/redo including pattern
  extension, real file persistence, no-op history/dirty/redo preservation,
  numeric/MIDI/tick refusals, output cap and ID exhaustion.
- `cargo clippy -p windfall-project --lib --test piano_tools -- -D warnings`,
  `cargo fmt --all --check`, and `git diff --check`: passed.
- Generated TypeScript into the local target directory, then copied it into
  the local UI for validation. `scripts/build-sim.sh`,
  `node scripts/check-bindings.mjs target/piano-rhythm-bindings` and
  `node scripts/check-sim.mjs`: passed. All generated files are excluded from
  the source commit; the parent regenerates the combined integration artifacts.
  After final verification, tracked artifacts were restored to the base and
  new generated exports removed from the source tree for a clean handoff.
  Validation copies remain in ignored `target/piano-rhythm-bindings` and
  `target/piano-rhythm-validation-artifacts`.
- The focused UI/WASM run over `piano-rhythm.test.tsx`, `note-tools.test.tsx`,
  `note-tools-recovery.test.ts`, `scale-stamp.test.tsx`, and
  `lib/ipc/sim/document.test.ts` passed 121 tests in the final resumed run,
  including 20 rhythm-control cases. Actual accessible controls drive
  Apply/Cancel/invalid fields, full-output refusal, empty result selection,
  no-ops/redo, and scale
  opt-in without retuning Rust output. Recovery cases use the real backend
  with held snapshot responses to verify all created-hit IDs are selected
  after a revision gap, and discarded on replacement during recovery.
- `pnpm typecheck`, scoped ESLint/Prettier and `pnpm build`: passed. Vite reports
  the existing large-chunk advisory.
- T3 collaborative Chromium preview on an isolated port/tab at 1280×800:
  inspected the Custom Chop and Arpeggiate dialogs; duplicate boundaries
  disabled Apply; Cancel changed no notes/history. Custom Chop changed four
  notes into eight accented/gated pieces, all selected; one Undo restored four
  originals and Redo restored eight pieces with the documented selection
  behavior. Arpeggiate at rate 120, gate 50%, two octaves/two repetitions
  produced 16 real repeated hits from four original onsets; one Undo restored
  all four originals. The longer arpeggio form scrolls inside the dialog.

There is no live transform preview. Exact policy/help and real Rust/WASM result
tests satisfy the bounded review contract without a second UI rhythm algorithm.
Native device playback, packaged Tauri interaction and physical MIDI were not
verified. Parent integration/parity reconciliation and artifact regeneration
remain separate from this source delivery.
