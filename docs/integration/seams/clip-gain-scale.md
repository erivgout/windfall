# Clip gain scale

The audio clip inspector offers Half and Double in a Gain scale group
immediately after Gain presets. Each command changes the gain of the
selected audio clips in one undo step through one `updateAudioClips` command.
Half divides each clip's current linear gain by two. Double multiplies it
by two and stops at the maximum gain (`MAX_GAIN`, currently 2). Neither
operation rounds the result.

Clips that would not change are left out. The pure
`gainScaleUpdates(clips, factor)` helper also omits changes smaller than
0.001, preserves the given order, and does not mutate the input. Each patch
contains only gain. A button is disabled when its update list is empty;
clicking computes the updates again and sends no command for an empty list.

Pan, pitch, normalize, and the fixed Quiet, Unity, and Loud presets stay as
they are.

Focused coverage: `apps/desktop/src/features/playlist/audio/gain-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/gain-scale.test.ts
```
