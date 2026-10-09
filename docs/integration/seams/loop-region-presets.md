# Loop region presets

The sampler inspector's Loop section shows Whole, First half, Second half,
and Last quarter buttons after the crossfade presets. They store loop regions
of 0 to 1, 0 to 0.5, 0.5 to 1, and 0.75 to 1 respectively. Both edges are
fractions of the trimmed sample. A missing start means 0 and a missing end
means 1.

Each preset sets both loop edges in one undo step through one `updateSampler`
command containing only `loopStart` and `loopEnd`. Whole stores 0 and 1, so
the loop covers the trimmed sample. Sample trim and crossfade stay as they
are.

The buttons work while the loop is off, and a preset does not turn the loop
on or change its mode. The matching preset is disabled, including when both
edges differ by less than 0.001. The click handler reads the latest sampler
source and checks both edges before dispatching.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/loop-region-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/loop-region-presets.test.ts
```
