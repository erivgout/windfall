# Piano-roll chord readout seam

The piano-roll toolbar shows a persistent readout next to the **Chords** button,
in an element with `aria-label="Chord"`. It reuses the existing `detectChord`
without changing its algorithm or labels. Recognition remains triads only;
unmatched pitch sets display the detector's `Unknown chord (...)` label.

The pure `chordReadout` helper accepts only the open channel's notes. Selection
wins: if any of those notes are selected, their keys determine the label,
regardless of their timing. Otherwise, a numeric session playhead is floored,
and notes contribute exactly when `start <= floor(playhead) < start + length`.
Ended notes and notes on other channels are excluded. With no contributing
keys, the readout displays an em dash and does not call `detectChord`.

`ChordReadout` subscribes through `editor.subscribe` and `session.onPlayhead`,
using the label as a stable string snapshot. Both subscriptions are removed on
unmount. The readout does not insert notes, dispatch commands, or change the
Chord tools dialog.

From `apps/desktop`, run:

```powershell
pnpm test -- src/features/piano-roll/chords
```

The focused tests cover selection precedence, playhead updates and flooring,
exclusive note ends, channel isolation, empty and unknown results, and the
absence of dispatches. Existing chord tool tests also run in this suite.
