# Clip pitch presets

The audio clip inspector offers Octave down (-12), Fifth down (-7),
Unison (0), Fifth up (7), and Octave up (12) in semitones immediately after
the Pitch control and before `ClipProcessingControls`. Each preset sets pitch
on the selected audio clips that are not already there, in one undo step
through one `updateAudioClips` command. Pitch still changes the tape speed.
The group is hidden for spectral stretch, under the same condition as the
Pitch field.

The pure `pitchPresetUpdates(clips, preset)` helper omits clips whose pitch
differs from the preset by less than 0.001. It preserves the given order,
does not mutate the input, and returns patches containing only pitch.
The matching button is disabled when every selected clip is already there.
An empty update list sends no command.

Focused coverage: `apps/desktop/src/features/playlist/audio/pitch-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/pitch-presets.test.ts
```
