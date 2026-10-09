# Modulation presets

The piano-roll lane header offers Low (0), Center (0.5), and High (1) after
the fine-pitch preset group and before Write LFO. The entries are hidden
unless the lane is showing modulation X or modulation Y. Center stores 0.5,
which applies no offset.

Each preset sets that lane's modulation on the selected notes, or on every
note when none are selected, in one undo step through one `updateNotes`
command. Each note patch contains only expression, with that lane's axis
replaced. The other modulation axis, fine pitch, and release stay as they
are, along with articulation, glide, and any other expression fields.
Notes without expression use `DEFAULT_NOTE_EXPRESSION` with only that axis
replaced, and count as 0.5 on either axis.

Notes already at the preset are omitted; a difference smaller than 0.001
counts as a match. Each entry is disabled when there is no editor context or
nothing to change. An empty update list or missing editor context sends no
command.

The pure `modulationPresetUpdates(notes, axis, preset)` helper preserves the
given order and does not mutate the input. The menu subscription includes
both modulation axes alongside velocity, pan, release, and fine pitch so
the entries stay current as notes change.

Focused coverage: `apps/desktop/src/features/piano-roll/modulation-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/modulation-presets.test.ts
```
