# Velocity presets

The piano-roll lane header offers Soft (0.25), Medium (0.5), Strong (0.8),
and Full (1) after the separator and before Write LFO. The entries are hidden
unless the lane is showing velocity.

Each preset sets velocity on the selected notes, or on every note on the open
channel when none are selected, in one undo step through one `updateNotes`
command. Notes already there are left out. A difference smaller than 0.001
counts as a match. The matching entry is disabled when there is nothing to
change. An empty update list or missing editor context sends no command.

The pure `velocityPresetUpdates(notes, preset)` helper preserves the given
order, does not mutate the input, and returns only note IDs and velocities.

Mute stays on the mute tool. There is no silent preset: velocity 0 mutes a note.

Focused coverage: `apps/desktop/src/features/piano-roll/velocity-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/velocity-presets.test.ts
```
