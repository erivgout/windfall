# Fine pitch presets

The piano-roll lane header offers Octave down (-1200 cents), Semitone down
(-100 cents), In tune (0 cents), Semitone up (100 cents), and Octave up
(1200 cents) after the release preset group and before Write LFO. The entries
are hidden unless the lane is showing fine pitch. In tune stores 0 cents,
which is in tune with the note's MIDI key.

Each preset sets fine pitch on the selected notes, or on every note when none
are selected, in one undo step through one `updateNotes` command. Each note
patch contains only expression, with finePitchCents replaced. Release and
modulation stay as they are, along with articulation, glide, and any other
expression fields. Notes without expression use `DEFAULT_NOTE_EXPRESSION`
with finePitchCents replaced.

Notes already at the preset are omitted; a difference smaller than 0.001 cents
counts as a match. Each entry is disabled when there is no editor context or
nothing to change. An empty update list or missing editor context sends no
command.

The pure `finePitchPresetUpdates(notes, preset)` helper preserves the given
order and does not mutate the input. Notes without expression count as 0 cents.
The menu subscription includes fine pitch alongside velocity, pan, and release
so the entries stay current as notes change.

Focused coverage: `apps/desktop/src/features/piano-roll/fine-pitch-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/fine-pitch-presets.test.ts
```
