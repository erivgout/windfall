# Jump to pattern edges

The piano-roll ruler context menu offers "Jump to the start" and "Jump to
the end" after "Edit markers / meter here…" and before the existing
separator.

The jumps seek the pattern playhead through `seek` only when the transport
mode is pattern. Song mode leaves the song position alone and does not
call `seek`. Each jump reads the open pattern's current length. It does
not change the pattern length or dispatch a project command.

The pure `patternEdgeTick(lengthSteps, edge)` helper returns 0 for the
start. For the end, it returns `lengthSteps * TICKS_PER_STEP - 1` when the
length is at least 1, and 0 otherwise, so the destination is never negative.

Focused coverage: `apps/desktop/src/features/piano-roll/pattern-jumps.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/pattern-jumps.test.ts
```
