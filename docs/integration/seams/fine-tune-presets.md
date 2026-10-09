# Sampler fine-tune presets

The sampler's Sound section shows five fine-tune presets after the tune presets
and before the gain presets: −50, −25, In tune, +25, and +50 cents. Each preset
sets the fine tune in one undo step and keeps the semitone on the Tune knob.
In tune stores no cents. Sample gain and the root key stay as they are.

Tune and Fine share one stored tune. `nextSamplerFine` splits the latest tune
around the current coarse semitone, joins that semitone with the preset cents,
and checks that the result still shows the same semitone and the requested
cents. A preset that would pass the tune limit does not apply; for example,
+50 cents cannot be stored at +48 semitones.

The matching preset is disabled. Presets that cannot be stored are also
disabled. The click handler reads the latest tune with `storedTune(id)` and
checks again before dispatching one `updateSampler` command with a patch
containing only `tune`. It does not change the coarse state.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/fine-tune-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/fine-tune-presets.test.ts
```
