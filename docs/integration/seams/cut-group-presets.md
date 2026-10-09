# Cut group presets

The sampler's Sound section shows None, 1, 2, and 3 after the cut group row.
Each preset sets the cut group in one undo step through one `updateSampler`
command with a patch containing only `cutGroup`.

None stores 0, so the channel does not cut other channels. Channels that
share 1, 2, or 3 stop each other. Cut itself stays as it is, along with
reverse, root key, tune, and sample gain. The number field, switches, and
other preset buttons keep their existing behavior.

The matching preset is disabled. On click, the handler reads the latest
channel source with `findChannel(id)` and does nothing if it is no longer a
sampler or its cut group already matches the preset. `nextCutGroup` returns
`null` for an exact match and otherwise returns the preset. Cut groups are
whole numbers from 0 to 255.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/cut-group-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/cut-group-presets.test.ts
```
