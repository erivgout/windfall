# Mixer send level presets

Right-click a mixer send gain knob and open the Send level submenu to choose
Off (0), Quiet (0.5), Unity (1), or Loud (1.5), in that order. The submenu follows
the automation entries and precedes the separator leading to Remove send.
All presets are within the range of 0 to `MAX_GAIN` (2).

`nextSendGain(current, preset)` returns `null` when the absolute difference is
less than 0.001, and otherwise returns the preset. The matching preset is
disabled. Each inline action checks the helper again before dispatching; a
match dispatches no command.

Choosing a different preset dispatches one `setSend` command with `from`,
`to: send.target`, and `gain`. Each preset is one undo step for that send.
Off leaves the send in place at silence.

Knob dragging is unchanged. Double-click still sets 0 dB (gain 1). Removing
the send is unchanged, and sidechain knobs are unchanged.

Focused coverage: `apps/desktop/src/features/mixer/send-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/send-presets.test.ts
```
