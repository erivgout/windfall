# Piano-roll chord tools seam

This is a partial stand-in for the chord tools in roadmap section X2. It does
not print notation and does not close a parity row. No bindings, note model,
piano-roll store or Rust changes are needed.

## Entry point and behavior

The piano-roll toolbar mounts `ChordToolsControl` from
`apps/desktop/src/features/piano-roll/chords/control.tsx`. The **Chords** button
opens selection detection, root, triad quality, start tick, length and seed
controls. Empty selection reads **No notes selected**. Unmatched sets read
**Unknown chord** followed by their pitch names.

`detectChord(keys)` is pure and deterministic. It reduces MIDI keys to sorted
unique pitch classes and recognizes major, minor, diminished, augmented,
suspended-second and suspended-fourth triads. Inversions, input ordering and
octave doublings have no effect. Ambiguous augmented and suspended sets use
the lowest pitch-class root, then the declared quality order. It does not infer
harmonic context, spell flats, recognize seventh chords, or analyze notes over
time. Selection detection combines every selected note regardless of timing.

Manual insertion uses the chosen MIDI root and quality. Seeded insertion uses
that field as a major-key tonic and selects from I, ii, iii, IV, V, vi and vii°.
`seededTriad(seed, tonic)` accepts a signed 32-bit integer seed and a MIDI tonic
from 0 to 110. Its fixed permutation uses the seed modulo seven and tonic pitch
class: identical inputs produce identical root-position notes. Consecutive
seeds change the result; the sequence repeats after seven seeds. **Next seed**
wraps at the signed integer limit. This is a small reproducible chooser, not
a general harmonic progression generator. Generated triads ignore the manual
quality control. Previews show exact MIDI pitches before insertion.

## Existing command boundary

`chordInsertCommand({ pattern, channel }, keys, start, length)` validates three
distinct MIDI pitches, whole ticks and the maximum pattern end. It calls
`edit-math.ts`'s existing `noteInsertionCommand` once with three `NoteInit`
values. It uses `withExtension` so pattern growth is included in the same
one-undo command. Notes have velocity 0.8, neutral pan and ordinary default
expression. No selection notes are removed or replaced.

The control dispatches through the existing project store, selects the created
notes on success, blocks repeated submission, and reports dispatch failure.
Requests capture the lane, project generation and revision; stale requests
cannot submit. Project replacement and history navigation close the dialog.
Invalid roots and timing are rejected rather than clipped.

## Validation

From `apps/desktop`:

```powershell
pnpm test src/features/piano-roll/chords --pool=threads --maxWorkers=1
```

The focused tests cover triad detection, empty/unknown selections, inversions,
seed stability and advancement, MIDI/tick bounds, command payloads, the UI,
and insertion plus pattern extension as one undo step with redo. They use the
desktop Vitest runner and existing mock backend, without starting the app.
