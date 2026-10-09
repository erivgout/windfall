# Clip pan scale

The audio clip inspector offers Half and Double in a Pan scale group after
Pan presets and before Pitch. Each command changes the pan of the selected
audio clips in one undo step through one `updateAudioClips` command.

Half divides each clip's current pan by two, moving it toward the center.
Double multiplies pan by two and stops at hard left (-1) or hard right (1).
Neither operation rounds the result.

Clips that would not change are left out, including clips already centered.
The pure `clipPanScaleUpdates(clips, factor)` helper also omits changes smaller
than 0.001, preserves the given order, and does not mutate the input. Each
patch contains only pan. A button is disabled when its update list is empty;
clicking computes the updates again and sends no command for an empty list.

Gain, pitch, and the fixed Hard left, Left, Center, Right, and Hard right pan
presets stay as they are.

Focused coverage: `apps/desktop/src/features/playlist/audio/clip-pan-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/clip-pan-scale.test.ts
```
