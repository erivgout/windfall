# Swing presets

Right-click the channel rack's swing percent readout to choose Straight (0),
Light (0.25), Medium (0.5), Heavy (0.75), or Full (1), in that order.

The pure `nextSwing(current, preset)` helper returns `null` when the absolute
difference from the preset value is less than 0.001. Otherwise, it returns
the preset value. A matching item is disabled and dispatches no command.
The current project value is checked again when selecting a preset.

Selecting a different preset dispatches one `updateSettings` command with
`patch: { swing: value }`. One preset is one undo step. Only project swing
changes; each channel's own swing mix is untouched. Zero stays straight,
and one stays the full delay of every second step.

The knob is unchanged, including its double-click reset to zero.

Focused coverage: `apps/desktop/src/features/channel-rack/swing-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/channel-rack/swing-presets.test.ts
```
