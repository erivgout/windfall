# Clip gain presets

The audio clip inspector offers Quiet (0.5), Unity (1), and Loud (1.5)
buttons immediately after the Gain knob and before Normalize. Each preset
sets gain on the selected audio clips that are not already there, in one
undo step through one `updateAudioClips` command.

The pure `gainPresetUpdates(clips, preset)` helper omits clips whose gain
differs from the preset by less than 0.001. It preserves the given order,
does not mutate the input, and returns patches containing only gain.
The matching button is disabled when every selected clip is already there.
An empty update list sends no command.

Double-clicking the Gain knob still sets 0 dB (gain 1). Normalize is unchanged.

Focused coverage: `apps/desktop/src/features/playlist/audio/gain-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/gain-presets.test.ts
```
