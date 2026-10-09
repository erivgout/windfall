# Envelope shape presets

While the sampler's volume envelope is on, the inspector shows Pluck, Keys,
Organ, and Pad buttons below the four envelope knobs. Each preset replaces the
whole shape in one undo step by dispatching one `setSamplerEnvelope` command
with the channel's `id` and the full envelope, outside the drag gesture.

The shapes use attack, decay, and release times in milliseconds:

- Pluck: attack 1, decay 180, sustain 0, release 80.
- Keys: attack 8, decay 400, sustain 0.7, release 250.
- Organ: attack 8, decay 0, sustain 1, release 30.
- Pad: attack 500, decay 300, sustain 0.85, release 800.

The matching preset is disabled. `nextEnvelope(current, preset)` returns `null`
when all three times match exactly and the sustain difference is less than
0.001. Otherwise it returns the preset without mutating the input. The click
handler checks the helper again; a match dispatches no command.

The knobs still change one stage at a time, and the shape editor is unchanged.
Turning the envelope off is unchanged. Presets are hidden while it is off and
do not turn it on.

Focused coverage: `apps/desktop/src/features/channel-rack/inspector/envelope-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/inspector/envelope-presets.test.ts
```
