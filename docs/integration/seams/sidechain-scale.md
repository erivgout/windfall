# Sidechain gain scaling

The sidechain knob's Sidechain level submenu offers Half and Double after
Off, Quiet, Unity, and Loud. Each command sets that one sidechain's gain in
one undo step, using its latest gain when clicked. Other sidechains are
unchanged. If the sidechain has disappeared or the change is smaller than
0.001, no command is sent.

Half divides the linear gain by two without rounding. Off stays in place,
and half of silence stays silent. Double stops at the maximum gain of 2.
Half and Double are disabled when their change would be smaller than 0.001.

Off, Quiet, Unity, and Loud are unchanged. Removing a sidechain is unchanged.
The send level is unchanged.

Focused coverage: `apps/desktop/src/features/mixer/sidechain-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/sidechain-scale.test.ts
```
