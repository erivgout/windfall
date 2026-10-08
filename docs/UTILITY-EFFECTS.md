# Seven utility effects

Windfall adds seven independent built-in processors through `Effect`,
`AnyEffect`, `EffectParams`, and the generated parameter descriptors. They
can be inserted at any mixer chain position, including the master. They do
not depend on a downloaded plugin. Their editors use `GenericParamEditor`
and the existing audio controls, checked document commands, gesture history,
automation menus, and slot bypass/mix.

The reference equivalents are Fruity Balance, Center, Mute 2, Phase
Inverter, Stereo Shaper, Soft Clipper, and Fast Dist respectively. This
documents Windfall's own transfers; it does not claim identical proprietary
sound. Root parity accounting is left to integration.

## Transfers and controls

- **Balance** (`balance`): linear gain `g` from 0 to 4 and balance `p` from
  -1 to 1. `Lout = g (1-max(p,0)) Lin`,
  `Rout = g (1+min(p,0)) Rin`. Centered gain 1 is exact unity;
  pan attenuates the opposite channel without amplifying the favored one.
  Gain zero produces exact silence.
- **DC blocker** (`dcBlock`): cutoff 1–40 Hz, default 5 Hz.
  `r = exp(-2π cutoff / Fs)` and
  `y[n] = (1+r)/2 (x[n]-x[n-1]) + r y[n-1]` on each channel.
  This normalizes Nyquist gain, rejects DC, and has an approximately -3 dB
  cutoff. State is double precision and flushed to zero. The default is a
  filter, with a decaying step response; it is not bit-unity.
- **Channel mute** (`channelMute`): independent left/right switches multiply
  each channel by 0 or 1. Either, both, or neither can be muted.
- **Polarity** (`polarity`): independent left/right switches multiply each
  channel by -1 or 1. A live switch crossfades through zero; this is polarity
  inversion, not an arbitrary frequency-dependent phase rotation.
- **Stereo matrix** (`stereoMatrix`): coefficients `ll, lr, rl, rr`, each
  -2 to 2. In stereo mode,
  `Lout = ll Lin + lr Rin`, `Rout = rl Lin + rr Rin`.
  The identity defaults are `1,0,0,1`.
  Coefficients display and accept typed signed linear numbers, including
  negative multipliers, rather than dB gain readouts.
  Mid/side mode encodes `M=(L+R)/2, S=(L-R)/2`, applies the same matrix, then decodes
  `L=M+S, R=M-S`. Encode-only and decode-only routing views allow a tested
  round trip between two processors. With the diagonal matrix `1,width`,
  mid/side width 0 removes side content, 1 is unity, 2 doubles it, and
  negative width exchanges its polarity. Identical mono remains unchanged.
  No extra processor aliases are registered for these views.
- **Soft clipper** (`softClipper`): ceiling -24–0 dBFS and knee fraction
  `k` from 0.05 to 1. Let `C` be the linear ceiling, `A=C(1-k)`, and
  `W=Ck`. Magnitudes through `A` are unchanged. Between `A` and `A+2W`,
  `t=(abs(x)-A)/(2W)` and `abs(y)=A+W(2t-t²)`. Beyond the knee,
  `abs(y)=C`. Restore the input sign. The transfer is odd, monotone,
  continuous with continuous first derivative, and bounded by the current
  ceiling. It is a memoryless peak rounder with zero look-ahead, not the
  existing gain-envelope limiter. It does not oversample; high-frequency
  clipping can alias.
- **Drive distortion** (`distortion`): drive 0–36 dB, shape `s` from 0 to 1,
  and output trim -24–0 dB. The driven signal `u` becomes
  `(1-s) u/(1+abs(u)) + s clamp(u,-1,1)`. The rational saturation and hard
  clipping endpoints are distinct from the polynomial soft clipper. This
  nonlinearity runs at four times the base rate, between two 129-tap
  Blackman-windowed sinc filters with cutoff at 90% of base Nyquist.
  Polyphase interpolation has individually normalized phase DC gains;
  decimation has normalized DC gain. FIR ringing can exceed the static
  nonlinearity's unit peak; this is not a brick-wall output limiter.

Gain, coefficients, cutoff pole, soft-knee parameters, drive/shape/trim,
mute gains, and polarity gains ramp in signal space over exactly 5 ms.
Changes before the first processed sample, and reset, take their target
settings immediately. Delay changes crossfade between whole-sample taps
over the existing 5 ms latency-transition interval, without pitching the
signal. Rapid retargeting preserves the currently audible tap mixture and
starts another 5 ms fade to the latest target. The final target is fully
applied 5 ms after the last edit; intermediate requested taps are retained
at their audible weights, without queuing settings. Prepared storage holds
at most one contribution per whole-sample delay in the 0–50 ms range.
The slot's dry signal uses the same transition. Engine compensation mirrors
eligible reference routes as ordered shared-delay stages, so a downstream
delay carries upstream fade weights as well as audio. Zero-delay matrix
stages retain history for the first delayed edit. This covers overlapping
edits in a serial matrix route without flattening their transfer to one fade.
Processing does not
depend on block divisions, including irregular one-sample blocks.

## Matrix delay and host accounting

Each matrix output has its own 0–50 ms delay, rounded to whole samples.
Both lines are allocated during `prepare` for the entire bound. The host
reserves that bound in its compensation buffers. Every setting is checked
and clamped before use; there is no buffer growth on parameter changes.

**PDC reports the shared delay: `min(leftDelay, rightDelay)`.** The remaining
channel difference is deliberate stereo processing. For example, 2 ms
left/5 ms right at 48 kHz reports 96 samples latency and retains 144 samples
of additional right delay. Other tracks align with the first channel, not
with the later intentional stereo echo. Tail and gap report the maximum
channel delay, including the old taps while a transition is active. The
slot's bypass and dry mix use the shared delay, preserving host alignment.
Zero-delay slots keep their dry history primed for the first delayed mix.
After bypass or zero mix makes the processor dormant, waking runs unheard
until **both** matrix outputs are primed, using the maximum channel delay.
Increasing a delay during priming extends that wait. This wait is separate
from the shared minimum used for PDC and the dry signal.
Live insertion uses this maximum output readiness for the engine's outer
splice too. A longer delay requested while insertion is still unheard
extends its wait using the samples already collected. During a bypass or
mix fade to zero, tail/gap accounting keeps
both wet memory and aligned dry taps; only a settled zero-wet slot switches
to dry-only accounting.

Routing compensation retains the existing one-second bound. Serial
mirroring applies when the shorter input is a prefix of the reference
route, or its fixed leading delay can be factored out, and the sum of
prepared stage maxima fits that bound. Distinct varying branches and
changes of reference route retain scalar compensation: settled alignment
is preserved, but exact transient phase cancellation is not guaranteed.
The same limit applies to topology changes during an active tap fade.
[UTILITY-REPAIRS.md](UTILITY-REPAIRS.md) specifies these boundaries and the
prepared storage/CPU costs. Matrix tap targets settle after 5 ms; their
effect at a route's output also includes downstream delay.

Drive distortion reports **32 samples of linear-phase group delay** and
64 samples of finite FIR tail/gap at every rate. Its impulse can have
pre-ringing before the reported peak, like any causal linear-phase FIR
with group delay. PDC and offline rendering account for that group delay.
The other utilities report zero latency. DC blocking reports a conservative
decay tail at its minimum allowed cutoff.

The existing engine does **not** rebuild routing compensation from audio
automation. It refuses automation changes that alter latency. Matrix delay
automation menus therefore show **Changes the latency**, just as limiter
look-ahead does; delay changes remain available through project commands,
undo/redo and live control-side plan replacement. Matrix coefficients and
routing, all other utility controls, and slot mix use existing automation.
No plugin runtime or new host contract is introduced here. A future host
contract for latency-changing automation remains separate work. Browser
latency display simulation now includes shared matrix delay and distortion
latency; it still does not implement audio DSP or send-path simulation.
The mixer inspector also totals limiter, shared matrix and distortion
latency, including bypassed and dry slots. A 2/5 ms matrix followed by
distortion at 48 kHz displays 128 compensated samples. Its hint distinguishes
shared latency from intentional stereo delay. Hosted plugin placeholders
are excluded from this built-in total; runtime plugin latency is not stored
in the project binding and is not part of this badge.

## Data and realtime ownership

New kinds are appended after the existing five. Existing descriptor indices
are unchanged. The new persisted index orders are:

```text
balance:       gain, pan
dcBlock:       cutoffHz
channelMute:   left, right
polarity:      left, right
stereoMatrix:  mode, ll, lr, rl, rr, leftDelayMs, rightDelayMs
softClipper:   ceilingDb, knee
distortion:    driveDb, shape, outputDb
```

Parameter structs are sanitized `Copy` values with serde defaults. Empty
settings objects load their defaults, and project format remains v1.
Adding, editing, moving and removing processors uses existing checked
commands and patches. Browser editing runs the same Rust document via WASM.

Direct processor calls replace NaN/Inf input with zero, clamp finite audio
to ±1000, and flush inaudible values below `1e-20`. The bounded transfers
cannot overflow from that range. `prepare` sanitizes nonfinite rates and
caps utility storage/ramp calculations at 384 kHz; supported verification
rates are 44.1/48/96 kHz. Matrix output can exceed unity by its specified
linear gains; utilities are not all output limiters.

Only construction and prepare may allocate. Process, parameter/tempo
changes, reset, bypass/mix and rejection paths perform no allocation,
deallocation, locks, waits or IO. Existing immutable plans and control-side
retirement own destruction. Fixed FIR arrays contain no callback-side heap
replacement. The allocator suites exercise every new kind, rapid edits,
block lengths, bypass, mix, reset and wrong-kind rejection.

## Measured quality and verification

The distortion alias measurement uses a coherent 0.8-amplitude tone at
773/4096 of Fs (8322.583 Hz at 44.1 kHz, 9058.594 Hz at 48 kHz, and
18117.188 Hz at 96 kHz). A 64-times-rate reference applies the same
nonlinearity and an ideal spectral low-pass. All baseband spectral magnitudes
except the wanted fundamental are compared against that reference; the
base-rate nonlinearity is the unfiltered baseline. Startup is discarded.
Drive settings 6/12/24/36 dB and shapes 0/0.5/1 are tested at all three rates.

Observed error reduction is **14.82–38.98 dB**. At 12 dB drive it is
33.48–35.03 dB; at maximum 36 dB drive it is 14.82–17.77 dB. The regression
bound is 12 dB reduction and under 0.3% fundamental magnitude error for
these signals. This is a reproducible synthetic bound, not a universal
alias floor. Hard clipping at extreme drive still has audible alias
potential, and the FIR deliberately rolls off the top of the band.

One release microbenchmark on Windows / Intel Core i9-14900F processed
1500 stereo blocks of 512 frames at 48 kHz (16 seconds of audio), at default
settings. Construction and prepare are outside the timer; resetting the
input buffers is inside. The observed realtime factors were:

| Processor        | Audio duration / elapsed time |
| ---------------- | ----------------------------: |
| Balance          |                     13731.55× |
| DC blocker       |                      5793.53× |
| Channel mute     |                     13518.08× |
| Polarity         |                     13528.37× |
| Stereo matrix    |                      6274.02× |
| Soft clipper     |                      5619.36× |
| Drive distortion |                        82.36× |

These are single-run throughput measurements of default settings, including
the matrix's zero-delay path. They are not worst-case deadline guarantees.
The timing test is deliberately ignored by ordinary suites; reproduce with
`cargo test -p windfall-dsp --release --test dsp utility_effects_realtime_factors -- --ignored --nocapture`.

Musical listening, target-device deadline/soak testing, and native
WebView/OS walkthroughs remain external boundaries. No hardware audio,
listening review, or additional-platform claim follows from document/UI
simulations, throughput measurements, or synthetic signal tests.

## Checks and integration

Verified on 2026-10-07 in `gpt/t3-utility-effects`, based on
`3e188b678b9d571e5bdb0530b6c04c4577aa40c1`:

- `cargo test -p windfall-dsp --test dsp`: **142 passed**, three timing/demo
  tests ignored. This includes signal measurements at 44.1/48/96 kHz,
  descriptor/index/serde checks, irregular block/reset/bypass/mix regression
  cases, bad-input bounds, and the allocator suite for all twelve kinds.
- `cargo test -p windfall-project --test commands`: **135 passed**. New
  utilities add, edit through a checked gesture, undo/redo, save/reopen in v1,
  load empty settings defaults, and remove. The original five remain covered.
- `cargo test -p windfall-engine --test engine effects`: **35 passed**,
  183 unrelated tests filtered out. Includes all seven before/after a
  limiter, automatic PDC, changing common matrix delay with opposite tracks,
  move/remove state and tails, actual automation/audio changes, export/block
  parity, and callback allocation/free guards.
- Release `utility_effects_realtime_factors --ignored --nocapture`:
  **one timing test passed**, with results above.
- `cargo clippy -p windfall-dsp -p windfall-project -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed**.
- Desktop Vitest: **302 passed in 14 files**, using the requested
  `src/features/effects`, `src/features/params`, and
  `src/features/mixer/effects.test.tsx` filters plus the new
  `src/lib/ipc/sim/utility-effects.test.ts`. The run used `--maxWorkers=1`
  for machine memory coordination. Real WASM commands cover every utility
  setting, save/reopen, automation/removal undo, descriptor-driven menus,
  one-step gestures, and signed matrix coefficient entry.
- App and Vite TypeScript projects, ESLint, and Prettier on the six owned
  TypeScript files: **passed**. TypeScript incremental metadata is local to
  `target/utility_effects-ui`, separate from shared dependency installation.
- Fresh `scripts/gen-bindings.sh target/utility_effects-bindings` and
  `scripts/build-sim.sh`, followed by both generation comparison scripts:
  **passed**. This base produces **154 binding/fixture files** and a
  **1,667,064-byte** document WASM module.

All native Cargo verification used Git Bash's `scripts/msvc-env.sh`,
`CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR` under this worktree at
`target/utility_effects-verification`, and temporary native
`TS_RS_EXPORT_DIR` output. The WASM script uses this worktree's own
`target/sim`. Build processes ran sequentially.

**Generated artifacts are intentionally excluded from this commit.** After
combining accepted workers, the parent must run `scripts/gen-bindings.sh`
and `scripts/build-sim.sh`, then commit their generated outputs. New exports
are the seven parameter structs and `MatrixMode`; `EffectKind`,
`EffectParams`, the barrel, descriptors, and automation fixtures also change.

Narrow integration seams are the DSP unions/descriptors and existing
exhaustive test helpers, `lib/automation/targets.ts` for the matrix delay
boundary, and `lib/ipc/sim/effects.ts` for displayed latency. Existing
all-kind test chains now respect the ten-slot limit. No project format
change, plugin binding/runtime change, or new dependency is required.
Root README/parity accounting and shared generated outputs remain with
the parent. `v0.1.0-alpha.1` is unchanged.

## Review repairs

[UTILITY-REPAIRS.md](UTILITY-REPAIRS.md) records both review rounds,
the bounded retargeting policy, reproduction results and focused checks on
the repair branch. The measurements and original verification above retain
their original provenance; they are not new repair-branch benchmarks.
