# Piano-roll Drum mode

Windfall's piano toolbar exposes **Drum**, a session-only toggle registered as
`pianoRoll.drum`. Its shortcut is **G**, which was free in both Windfall and FL
piano-roll keymaps. The toggle does not select Paint automatically. It survives
switching editor tabs and channels, is excluded from persisted preferences, and
resets to off through `onProjectReplaced` after New/Open.

## Pointer and cell behavior

With Drum on, left presses in Paint route to a dedicated `drum` gesture before
note-body/edge move and resize hit testing. Draw and the other tools retain their
existing paths; with Drum off, Paint retains its remembered note length, spacing,
overlap checks, and note move/resize behavior. Right-click retains normal erase.

Drum cells use `paintSpacing(snap, snap)` for a positive snap interval, anchored
at tick zero or the local meter segment's start. Each new note starts at the cell
boundary, uses that cell's length, and stays on the pointer's key. A meter-change
boundary shortens a crossing cell. With snap set to None, spacing falls back to
`paintSpacing(0, lastLength)`. Alt does not turn a Drum stroke into free painting.
Scale highlighting is unchanged; Drum addresses the actual key row.

`drum-paint.ts` owns the stroke decision and handled-cell set. A filled cell has
one or more existing notes on that key whose starts lie in its half-open tick
range; a preceding note's tail does not fill it. The first cell fixes the stroke:

- Empty: add one note to each empty visited cell, leaving filled cells alone.
- Filled: remove every note starting in each visited cell, leaving empty cells
  alone.

The handled-cell identity includes both start tick and key, so revisiting a cell
does nothing and changing keys can address another cell at the same tick.
Pointer segments cover intervening rows and steps even when movement events
skip cells. Adds and deletes stay in the gesture's preview until release.

## Existing command seam

Release dispatches one `batch` labeled `Paint drum steps`, containing existing
`addNotes` and/or `removeNotes` commands as needed. The existing `withExtension`
path includes pattern growth in the same undo step when additions reach beyond
the current pattern. A stroke with no changes dispatches nothing. Escape,
pointer cancel/lost capture, blur, detach, tool/mode changes, and project
replacement abandon the preview without dispatching it.

No Rust, generated binding, playlist, or channel-rack change is required.

## Focused verification

Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/drum-paint.test.tsx
```

The focused file passes 12 tests covering empty-cell insertion, filled-cell
removal (including multiple and off-grid starts), both first-cell decisions,
revisits and key changes, note-tail occupancy, one batch/history step and
undo/redo, cancellation, normal Paint with Drum off, normal Draw with Drum on,
toolbar/G in both keymaps, and the project-replacement reset.
