# Windfall Delay time modulation

The existing Delay effect now supports two optional controls:

| Parameter | Range | Scale | Default | Automation index |
| --- | --- | --- | --- | --- |
| `modRateHz` | 0–8 Hz | Linear | 0 | 10 |
| `modDepthMs` | 0–20 ms | Linear | 0 | 11 |

The new knobs are appended after `mix` in the parameter table and the desktop
`DelayParams` binding. Older automation indices 0–9 are unchanged. Both fields
default to zero through the existing serde defaults, so saved Delay settings
that omit modulation retain their original sound. No new effect kind is needed.

A shared sine LFO offsets both stereo read positions around the existing delay
times, including tempo sync and stereo offset. Rate zero holds the current LFO
phase; reset restarts it at zero. Depth is the peak excursion in milliseconds
and smooths on parameter changes. At settled depth zero the original integer
tap path is used, with exactly the original delay time and no interpolation.

With movement enabled, fractional reads use linear interpolation to keep the
feedback path's interpolation gain at or below unity. This can soften high
frequencies in repeated echoes. Both taps in a delay-time crossfade receive the
same modulation offset. Each read is clamped between one sample and the line's
maximum valid delay; near the shortest time, negative excursions are limited.

`prepare` reserves room for the maximum delay, stereo offset, and modulation
depth. Tail and gap estimates include the current and target modulation depth.
`process`, `set_params`, `reset`, and `set_tempo` use only existing storage and
do not allocate.

Validation (Git Bash, with `source scripts/msvc-env.sh` for the MSVC linker):

```bash
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-dsp --lib delay::
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-dsp --test dsp delay::
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-dsp --test dsp realtime::effects_never_allocate_after_prepare
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo run --quiet -p windfall-dsp --example descriptors > apps/desktop/src/bindings/descriptors.json
```

Module tests cover omitted fields, stable parameter indices, zero-depth impulse
timing, finite moving echoes, clamped reads during crossfades, and held/reset LFO
phase. The TypeScript binding is edited by hand; only the existing descriptors
example refreshes `descriptors.json`.

The existing exhaustive Delay fixture in `tests/dsp/properties.rs` now fills
the appended fields from `DelayParams::default()`. Its previous settings remain
unchanged. Validation passed: 13 delay module tests (including three ts-rs
exports to the discard directory), 13 existing Delay integration tests, and the
existing allocation guard covering processing, parameter/tempo writes and reset
with randomized settings, including modulation.
