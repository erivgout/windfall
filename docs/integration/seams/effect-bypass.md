# Bypass and enable mixer effects

`mixer.bypassEffects` ("Bypass effects") and `mixer.enableEffects` ("Enable
effects") act on the selected mixer tracks. Both appear next to Show effects
in the mixer menu and in insert and master strip header menus. Neither has a
shortcut. Each is enabled only when at least one selected slot needs to change.

`effectEnableUpdates(tracks, enabled)` in
`apps/desktop/src/features/mixer/effect-enable.ts` returns `{ track, effect }`
pairs in track order and then slot order. A slot that is already in the
requested state is left out. Tracks with no slots contribute nothing.

`bypassSelectedEffects()` and `enableSelectedEffects()` in `effect-ops.ts` use
`selectedMixerTracks()` and dispatch one batch of `updateEffect` commands with
only `patch: { enabled }`. Bypass requests `false` with the label "Bypass mixer
effects"; enable requests `true` with the label "Enable mixer effects". Each
command is one undo step. Neither changes the mix. If no slots need changing,
nothing is dispatched and no undo step is created.

Focused coverage: `apps/desktop/src/features/mixer/effect-enable.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/mixer/effect-enable.test.ts
```
