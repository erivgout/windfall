# Mixer latency correction presets

The latency popover offers None (0 ms), 1 ms, 5 ms, and 10 ms next to Apply and
Reset. Each preset sets that track's declared latency correction in one undo
step. None stores 0, meaning no correction.

The buttons stay disabled while recording, while the panel is busy, and when
the current correction matches the preset within an absolute difference of
less than 0.001 ms. Clicking a preset checks the latest correction for that
track before dispatching one `updateMixerTrack` command. A matching correction
dispatches no command. Presets work even when the draft field is empty.

The number field, Apply, and Reset stay available with their existing behavior,
including the range of -1000 to 1000 ms and their recording and busy guards.
Successful presets update the draft and clear the error; failures use the same
error handling as Apply.

This does not move captured takes. The separate recording-offset control is
unchanged.

Focused coverage: `apps/desktop/src/features/mixer/latency-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/latency-presets.test.ts
```
