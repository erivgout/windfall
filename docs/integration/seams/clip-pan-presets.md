# Clip pan presets

The audio clip inspector offers Hard left (-1), Left (-0.5), Center (0),
Right (0.5), and Hard right (1) buttons immediately after the Pan control
and before the Pitch field. Each preset sets pan on the selected audio clips
that are not already there, in one undo step through one `updateAudioClips`
command.

The pure `panPresetUpdates(clips, preset)` helper omits clips whose pan
differs from the preset by less than 0.001. It preserves the given order,
does not mutate the input, and returns patches containing only pan.
The matching button is disabled when every selected clip is already there.
An empty update list sends no command.

Double-click still centers the knob (pan 0). Gain is unchanged.

Focused coverage: `apps/desktop/src/features/playlist/audio/pan-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/pan-presets.test.ts
```
