# Portamento duration presets

The piano-roll actions offer 16th (240 ticks), 8th (480 ticks), Quarter
(960 ticks), and Half (1920 ticks) portamento durations after the articulation
actions. Each command sets the portamento duration on the selected notes in one
undo step through one `updateCapturedNotes` command. A 16th stores 240 explicitly
when changing a note's duration.

Notes that are not selected stay as they are. The other expression fields stay
as they are, including release, fine pitch, modulation, articulation, and color
group. Changed notes merge `DEFAULT_NOTE_EXPRESSION`, their existing expression,
and the chosen `glideTicks` value.

`glidePresetUpdates(notes, preset)` treats a missing duration as 240, omits notes
already at the exact preset, preserves the given order, and does not mutate the
input. `setSelectedGlide(preset)` requires an editor context, selected notes,
and an editor that is not busy. An empty update list sends no command.

The toolbar's Portamento duration for newly drawn notes (`drawGlideTicks`) is a
separate setting.

Focused coverage: `apps/desktop/src/features/piano-roll/glide-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/glide-presets.test.ts
```
