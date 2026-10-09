# Sampler tune presets

The sampler's Sound section shows five presets below the knobs: Octave down
(-12 semitones), Fifth down (-7), Unison (0), Fifth up (7), and Octave up (12).
Each preset changes the semitone and keeps the fine tune already showing,
including Fine at -50 or +50 cents, in one undo step.

Tune and Fine are two views of one stored tune. The preset uses the cents from
`splitTune(source.tune, coarse)` and builds the new tune with
`joinTune(presetSemitones, cents, 48)`. Applying it dispatches one
`updateSampler` command with the channel's `id` and a patch containing only
`tune`. It also sets the displayed coarse semitone to the preset.

The matching preset is disabled. `nextSamplerTune` returns `null` when the
joined tune differs from the current tune by less than 0.001. The click handler
checks again and dispatches no command for a match.

The knobs still move one at a time. Root, Tune, Fine, Gain, and the cut group
retain their existing behavior.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/tune-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/tune-presets.test.ts
```
