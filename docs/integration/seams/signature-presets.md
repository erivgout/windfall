# Time-signature presets

The transport time-signature popover offers presets under the two dropdowns:
4/4, 3/4, 2/4, 6/8, 5/4, 7/8, and 12/8, in that order.

The pure `nextSignature(current, preset)` helper returns `null` when both
numbers already match. Otherwise, it returns only the preset's numerator
and denominator. The matching preset button is disabled and dispatches no
command.

Selecting a different preset sends one `updateSettings` command with
`patch: { timeSignature: next }`. One preset is one undo step because both
numbers change in that single command. The dropdowns still change one number
at a time, with a separate command for each dropdown change.

Focused coverage: `apps/desktop/src/features/transport/signature-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/transport/signature-presets.test.ts
```
