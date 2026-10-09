# Windfall piano-roll chord progression writer

Parity id: `core-chord-generator`.

`pianoRoll.generateProgression` / **Generate progression…** opens a dialog
from both piano-roll context menus and the command palette. Its enabled
rule matches Generate riff: the piano roll must have an open pattern channel
and an idle editor. There is no toolbar button. The workspace mounts the
dialog beside `RiffGeneratorDialog` and closes it on unmount.

These are diatonic triads from a fixed mood cycle, not a melody and not a
detected chord. The existing seeded Generate riff command is unchanged;
chord detection and readout do not participate in writing this progression.

## Settings and pure generation

`generateProgressionNotes` in `progression.ts` takes the channel's source
notes, root, mode, mood, bars, seed, bar duration, and draw velocity. It
returns note initializers without mutating its inputs or dispatching commands.

- Root: all twelve pitch classes C–B, including sharp/flat labels.
- Mode: major (`0,2,4,5,7,9,11`) or natural minor (`0,2,3,5,7,8,10`).
- Mood: bright, calm, or tense.
- Bars: an integer from 2 through 8, defaulting to 4.
- Seed: a non-negative safe integer, defaulting to 0.

Degrees are zero-based. The fixed cycles are:

- Major bright: `0,4,5,3`; calm: `0,5,3,4`; tense: `6,4,0,3`.
- Minor bright: `0,5,2,6`; calm: `0,5,3,4`; tense: `0,6,5,4`.

The seed rotates the cycle's start by `seed % 4`; generation walks the
requested number of bars and repeats the cycle as needed. Degree `d`
creates scale steps `d`, `d+2`, and `d+4`, wrapping the seven scale steps and
raising wrapped tones an octave to retain root-position triads. Each bar
contains exactly three simultaneous notes, each lasting the full bar.

Chord roots occupy the twelve-semitone window centered around the rounded
midpoint of the source notes' minimum and maximum pitches. Empty channels
use MIDI 60 (C4) as the center. The window shifts near the limits to reserve
room for the fifth, keeping every generated pitch in MIDI 24–96 without
clipping the triad's intervals. Identical source notes and settings produce
identical pitches and starts; seeds separated by four select the same cycle.

## Timing and command seam

The append tick is the maximum `start + length` across every existing note
in the open channel, or tick 0 when empty. The bar duration is captured when
the dialog opens, using the same `patternMeterAt` and `ticksPerBar` helpers
as Generate riff. Pattern timeline meter changes at the append tick take
precedence over the pattern's base meter; an absent base meter falls back to
4/4. That captured duration applies to the whole generated span. Snap does
not affect triads. Draw velocity is captured on open, and pan is centered.

Apply dispatches exactly one existing `addNotes` command for all the triads,
providing one undo step. It appends without deleting, moving, or changing
existing notes, including notes from previous progression runs. It does not
change the explicit pattern length. Notes beyond that length remain stored;
the existing ruler can extend the pattern for playback. Generation rejects
a span beyond `MAX_PATTERN_TICKS`.

Opening or changing dialog fields only computes the note preview count.
Cancel, Escape, dismissal, and invalid fields dispatch nothing. A captured
request cannot apply after its channel, session, revision, or document
changes. Project replacement and history navigation close the dialog.
Pending submission prevents duplicate Apply and cancellation until dispatch
completes. This seam uses existing frontend commands and needs no engine,
binding, or project-model changes.

## Focused verification

From `apps/desktop`:

```powershell
pnpm test -- src/features/piano-roll/progression.test.tsx src/features/piano-roll/riff-generator.test.tsx
```

The progression suite covers seed repetition and rotation, all six mood
cycles in every root, diatonic triads and MIDI bounds, simultaneous bar
timing, append placement, menus and action eligibility, dialog controls,
one `addNotes`, undo/redo, unchanged pattern length, repeated-run preservation,
unconfirmed and cancelled dialogs, invalid and stale requests, project
replacement, and pattern meter/fallback behavior. The two files pass
27 tests: 16 progression tests and 11 unchanged riff tests.
