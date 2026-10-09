# Windfall playlist brush meter integration seam

Playlist brush placement reads the same `meterSegments()` used by the ruler
and grid. `PlaylistSession.brushContext(brush, tick)` selects the last segment
whose start is at or before the clip's placement tick and passes that
signature's bar length to `brushClip()` and `brushTicks()`.

## Length and spacing

Before the first meter change, the project settings' time signature supplies
the bar length. Without meter changes, the existing lengths and spacing are
preserved. At a 7/8 placement tick, a bar is seven eighth notes: 3,360 ticks
with Windfall's 960 ticks per quarter note, compared with 3,840 in 4/4.

Draw resolves the context at its snapped placement tick. Paint resolves it
for each clip start. `paintStarts()` accepts a length callback so a forward
stroke advances by each clip's current length; a backward stroke samples the
length just before its preceding start. A stroke crossing an aligned meter
boundary therefore follows the bars of both segments. The existing numeric
length API remains supported.

The bar length is the minimum for an automation brush and the placeholder
length of an unread audio sample. Pattern lengths, known audio durations,
and automation curves longer than a bar still determine their own lengths.
Ghosts carry the resolved clip lengths into the existing placement commands.
Meter editing and existing clips are unaffected.

## Verification

From `apps/desktop`:

```sh
pnpm test src/features/playlist/playback.test.tsx src/features/playlist/session.test.ts src/features/playlist/intents.test.ts src/features/playlist/slip.test.tsx
```

The session tests cover Draw and Paint creating a 3,360-tick automation bar
at the 7/8 change, 4/4 before that change, unchanged no-meter length and
spacing, and leftward/rightward strokes crossing from 4/4 to 7/8. Existing
playlist gestures and Slip regressions pass with these tests.
