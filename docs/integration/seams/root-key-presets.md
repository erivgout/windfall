# Root key presets

The sampler's Sound section shows C3 (36), C4 (48), C5 (60), and C6 (72)
after the knob grid and before the tune presets. These are whole MIDI keys.
The root key is the key that plays the sample at its own pitch.

Each preset sets the root key in one undo step through one `updateSampler`
command with a patch containing only `rootKey`. C5 stores 60, which is the
key that matches a double-click on the root knob.

The matching preset is disabled. On click, the handler reads the latest
channel source with `findChannel(id)` and does nothing if it is no longer a
sampler or its root key already matches the preset. `nextRootKey` returns
`null` for an exact match and otherwise returns the preset.

Tune, fine tune, sample gain, reverse, cut itself, and cut group stay as they
are. The knobs and the tune and sample-gain preset buttons retain their
existing behavior.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/root-key-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/root-key-presets.test.ts
```
