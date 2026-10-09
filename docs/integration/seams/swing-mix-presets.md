# Swing mix presets

The channel inspector's Note timing section shows Straight, Light, Half, and
Full presets after the Swing mix row and before the Gate row. They store 0,
0.25, 0.5, and 1 respectively in the channel's `timing.swingMix` field.

Each preset sets the channel swing mix in one undo step through one
`updateChannel` command. Straight stores 0, so the channel ignores project
swing. Full stores 1, so the channel follows project swing. Gate and shift
stay as they are at click time because the command reads the latest channel
timing. This does not change the project swing amount.

The matching preset is disabled. `nextSwing` returns `null` when the current
mix differs from the preset by less than 0.001, and the click handler checks
the latest swing mix again before dispatching.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/swing-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/swing-presets.test.ts
```
