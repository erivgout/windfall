# Sampler loop crossfade

The channel sampler's `loopCrossfade` setting is a playback blend, not a written
sample. It does not bake a new sample file or alter the source audio. The default
of 0 matches the old loop, including one-frame loops and interpolation.

The Loop section exposes a **Loop crossfade** knob from 0 to 100 percent. It is
disabled while the loop mode is Off. Turning looping off preserves the saved
amount; sampler patches may also store an amount while looping is off.

`SamplerSettings.loop_crossfade` defaults to 0 and is omitted from saved projects
at 0. Patches clamp finite amounts to 0 through 1; non-finite amounts become 0.
The optional TypeScript field is `loopCrossfade` on settings and patches.

Playback rounds `amount * loop frames` to obtain N, capped at half the source
loop length. Forward loops blend the last N frames into the first N frames with
equal-power sine/cosine gains. Ping-pong loops use the same blend at the tail of
their reflected period, reading the reflected start. Frames outside the tail
stay dry, and Off ignores the amount. The blend does not change loop timing.
The reader uses only source-frame reads and arithmetic, without allocations,
locks, or IO.

`windfall_engine::loop_crossfade_mix` exposes the pure frame blend used by the
voice reader. The `sampler_loop_crossfade` integration target covers the dry
path, stereo equal-power mixing, loop-length cap, rounding, reflected reads,
one-frame loops, saved defaults, round trips, and patch sanitization. The
channel-rack inspector tests verify one patch per knob movement and the inert
Off state.
