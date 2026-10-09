# Fade presets

The audio clip inspector offers None, Short, Medium, and Long buttons after
the Fade out knob. Each preset sets both fades from that clip's own length:
None uses 0, Short uses 1/16, Medium uses 1/8, and Long uses 1/4. The target
is rounded to ticks, then clamped between zero and the clip's length.

The pure `fadePresetUpdates(clips, fraction)` helper preserves the given order
without mutating the input. A clip that is already there on both fades is
left out. A button is disabled when every selected audio clip already matches.

Each click dispatches one `updateAudioClips` command containing all changed
clips, so both fades on every affected clip change in one undo step. An empty
update list sends no command. The knobs still change one fade at a time.

Focused coverage: `apps/desktop/src/features/playlist/audio/fade-presets.test.ts`.
Run from `apps/desktop`:

```sh
pnpm test -- src/features/playlist/audio/fade-presets.test.ts
```
