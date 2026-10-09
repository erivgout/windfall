# Mixer fader Half and Double

Right-click a mixer fader and open the Volume submenu. Half and Double follow
Quiet, Unity, and Loud. Each command sets that one strip's fader in one undo
step. Other selected strips stay as they are. Quiet, Unity, and Loud stay as
they are.

Half divides the current linear volume by two. Half of silence stays silent.
Double multiplies the current linear volume by two and stops at the maximum
volume, `MAX_GAIN` (2). Neither operation rounds the result.

`scaledFaderVolume(volume, factor)` calculates the new volume.
`nextFaderVolumeScale(volume, factor)` returns `null` when the absolute change
is smaller than 0.001. Such menu items are disabled. Each action reads the
latest track volume from the project store and checks again before dispatching
one `updateMixerTrack` command with only `volume` in its patch. A missing track
or a change smaller than 0.001 dispatches no command and creates no undo step.

Fader dragging and the pan control keep their existing behavior.

Focused coverage: `apps/desktop/src/features/mixer/fader-scale.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/fader-scale.test.ts
```
