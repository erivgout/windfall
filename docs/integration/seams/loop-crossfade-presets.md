# Loop crossfade presets

The sampler inspector's Loop section shows None, Short, Medium, and Long
buttons after the knob row. They store loop crossfade values of 0, 0.25,
0.5, and 1 respectively. A missing crossfade value means 0.

Each preset sets the loop crossfade in one undo step through one
`updateSampler` command containing only `loopCrossfade`. None stores 0,
so playback keeps the original loop. Loop mode, loop start, and loop end
stay as they are.

The buttons stay disabled while the loop is off, and a preset does not
turn the loop on. The matching preset is also disabled, including when
the difference is smaller than 0.001. The click handler reads the latest
sampler source and checks its loop mode and crossfade before dispatching.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/loop-crossfade-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/loop-crossfade-presets.test.ts
```
