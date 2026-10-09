# Windfall piano-roll riff generator

`pianoRoll.generateRiff` / **Generate riff…** opens an append-only dialog from
the piano-roll toolbar, context menus, or action registry. It is enabled when
the piano roll has an open pattern channel and its editor is idle. An empty
channel is supported; selection is not required.

This is one seeded melody in a chosen scale. It is not a multi-stage
chord-progression machine. It does not infer harmony, create chord sections,
or replace the selected-note random generator.

## Controls and generation

- Root: all twelve pitch classes C–B, with sharp/flat labels.
- Scale: major, natural minor, or pentatonic major.
- Length: 1–4 bars. Density: low, medium, or high.
- Seed: an integer from 0 to 4294967295, defaulting to 0.

The append tick is the maximum `start + length` over all notes in the open
channel, including unselected notes; an empty channel starts at tick 0.
Pitches belong to the chosen scale in a twelve-semitone window centered on
the rounded midpoint of the existing minimum and maximum pitches, or MIDI
60 (C4) when empty. Near MIDI limits the window shifts to stay within 0–127.

The time signature at the append tick defines a bar. The current snap at
that tick defines each note's length and spacing; snap off uses one 240-tick
step. These values and the current draw velocity are captured when opening.
All generated notes retain that velocity, with centered pan. Meter changes
inside the generated span do not recalculate its captured bar or step size.
Only complete cells inside the requested span are used. The first cell
always contains a note; subsequent cells have 25%, 50%, or 80% hit probability.
Rests can leave the last portion of the span silent.

A fixed 32-bit LCG (`1664525 * state + 1013904223`, modulo 2³²) consumes two
values per cell: hit probability and scale pitch choice. The same source
pitch range, settings, seed, snap, meter, and draw velocity produce the same
relative melody. Repeated Apply after reopening appends another melody later
in time; a different seed can produce a different melody.

## Existing command seam

Apply dispatches exactly one existing `addNotes` command containing every
generated note. It is one undo step; existing notes and their properties
are preserved. Cancel, Escape, dialog dismissal, and invalid settings send
no command. A stale request after a channel, revision, session, or document
change cannot apply; project replacement and history navigation close it.
The panel closes the request on unmount. Pending submission blocks repeat
Apply and dialog cancellation until the command completes.

No Rust, engine, binding, or project-model change is needed. This command
does not update the explicit pattern length: appended notes past that length
remain stored, and the user can extend the pattern with the existing ruler
to include them in playback. The riff is rejected if its requested span
would pass the existing maximum pattern tick limit. Drum mode and pointer
tools keep their existing behavior.

## Focused verification

From `apps/desktop`:

```powershell
pnpm test src/features/piano-roll/riff-generator.test.tsx
```

The focused file pins a stable seed, checks every root in all three scales,
octave/MIDI bounds, unchanged source notes, append after the latest note end,
dialog settings, snap/draw velocity, the single `addNotes` dispatch and
undo/redo, snap-off fallback, empty-channel repeatability, cancel, invalid
inputs, and stale-channel refusal. All 11 tests pass. Focused ESLint passes;
the desktop TypeScript check has no riff-generator diagnostics, but still
fails on existing errors elsewhere in the workspace.
