# Velocity scaling

The piano-roll lane header offers Half and Double after Soft, Medium, Strong,
and Full. The entries are hidden unless the lane is showing velocity.

Each command sets velocity on the selected notes, or on every note on the open
channel when none are selected, in one undo step through one `updateNotes`
command. Each note patch contains only velocity.

Half divides the current velocity by two. Half of silence stays silent. Double
multiplies the current velocity by two and stops at full velocity (1). Neither
command rounds. Soft, Medium, Strong, and Full stay as they are.

The pure `velocityScaleUpdates(notes, factor)` helper keeps the given order,
does not mutate the input, and returns only note IDs and velocities. Changes
smaller than 0.001 are omitted. An entry is disabled when there is no editor
context or nothing to change. Clicking reads the current notes again; an empty
update list sends no command.

Focused coverage: `apps/desktop/src/features/piano-roll/velocity-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/velocity-scale.test.ts
```
