# Utility effect review repairs

Repair branch: `gpt/t3-utility-repairs`, based on
`055365868ef59a2c5b58833737b5142bc58971f2`. The three P2 findings were pinned
to `5254a40ef7a35aed202f17df7092c10d184c7bb8`. Tests below reproduced the
findings on the repair base before their corresponding fixes.
The first repair is `99792c909c82e69c49dea60f223759aa99f1a72c`. The second
round integrates parent `bde3fd77dacdd9c1d4c92cf3a77e5c4d1d1eab55`, which
already contains that repair and the other accepted parent changes.

## Reproductions and behavior

1. **Asymmetric wake-up.** At 48 kHz, an identity matrix with delays 0/50 ms
   was bypassed until dormant and then enabled while receiving constant 1.
   `asymmetric_matrix_wake_primes_both_outputs_before_bypass_or_mix_fade`
   failed at the first re-enabled right sample: `0.99791664` instead of 1.
   The slot now uses a separate processor warm-up duration: the maximum
   matrix delay, while PDC and dry mix retain the minimum. The wait also
   covers simultaneous dry/PDC transitions. Increasing a delay during the
   wait extends it using elapsed priming samples; it does not clear the
   audio already collected. A 5-to-50 ms edit during wake-up reproduced an
   additional hole before that extension was added. Tests cover irregular
   blocks, bypass, zero mix, half mix, reset, and eventual wet gain.
2. **Rapid delay edits.** The 48 kHz, 1 kHz sine reproduction changes right
   delay from zero to 0.5 ms after 4092 samples, processes 120 samples, then
   requests 1 ms. The initial half-mix test measured a `0.5001465` boundary
   step (`-0.49985346` to `-1`). The regression limits adjacent steps to
   0.15: the tone's natural step is at most 0.131, and a 240-sample tap fade
   contributes at most 2/240. It also checks repeated retargets, final
   settings, shared and asymmetric delay, and dry/half/full mixes.
   An engine cancellation regression initially measured residual peak 1
   during rapid common-delay edits at dry mix. Matrix wet taps, slot dry
   taps and routing compensation now preserve the audible mixture through
   the same `TapCrossfade` primitive. Opposite sampler tracks cancel during
   rapid edits at all three mixes, including one-sample blocks.
3. **Inspector latency.** A bypassed 2/5 ms matrix plus distortion returned
   zero from `chainLatencyFrames`, where the expected shared total is
   96 + 32 = 128 samples. The actual inspector regression found no badge.
   The helper now totals limiter look-ahead, rounded shared matrix delay
   and distortion's 32 samples regardless of slot enable/mix. Matrix
   rounding uses Rust's f32 multiplication order. The inspector shows
   `2.7 ms (128 samples) compensated`, updates with delay edits and undo,
   and explains shared latency and intentional stereo delay. A sampler
   channel and hosted effect coexist in the test chain. Plugin-bound
   placeholders are excluded from the built-in total.

## Second review: compiled reproductions

All four R2 tests failed against the integrated parent's source before any
corresponding production fix. These are compiled regressions, rather than
the review's source-equation traces. The sine fixture has amplitude 0.5,
twice that of the review traces; cancellation still uses the original
`1e-6` absolute peak threshold.

1. **Live asymmetric insertion:** constant 1 through a newly inserted
   identity 0/50 ms matrix failed at frame 1 with right output `0.99583334`.
   `EffectUnit::fade_in` now waits for maximum output readiness before its
   240-frame splice. Shared PDC remains zero. The test checks every output
   sample through priming and completion with blocks 1, 137 and 511.
   A later regression increased the right delay from 5 to 50 ms after 100
   priming samples and failed before elapsed priming was tracked. The outer
   wait now extends while insertion is still unheard. A shared-delay edit
   during that extension also retains its remaining wait in compensation:
   the left path cancels while the right stereo difference becomes audible.
2. **First zero-to-delay edit:** opposite 173 Hz sampler tracks, with one
   identity matrix initially 0/0 ms, failed with residual `0.3540905` after
   requesting 2/2 ms at frame 4092. Compensation now retains prepared
   zero-delay matrix stages, including their input history. Wet/dry and
   routing taps start their fades on the same frame. Full, half, dry and
   bypassed policies cancel throughout the edit, without a filling delay.
3. **Downstream timing:** two serial 1/1 ms matrices, editing the first to
   2/2 ms, failed with residual `0.20000002`. Compensation now processes
   the reference's shared stages in order. The second stage delays the
   first stage's fade weights naturally. Regressions also overlap edits
   in both stages, pass through zero, split the chain over a track and bus,
   and use irregular blocks including single frames. They cover all four
   slot policies, audible solo output, and exact settled restart/replay.
4. **Bypass tail/gap:** after one impulse through a 0/5 ms matrix, disabling
   the slot reported tail and gap zero. The queries now include wet memory
   while its gain is positive or moving, together with dry-tap history.
   Tests observe the right impulse 239 subsequent frames later at gain
   0.5, verify no output beyond the reported bounds, and verify dry-only
   bounds after settling. Variants include 2/5 ms, half/full mix, zero-mix
   fades, and irregular blocks.

An additional regression exposed removal timing before fixed downstream
limiters: the 173 Hz fixture measured residual `0.35409224` when removing
a settled matrix at the front of the chain. A leaving matrix now retains
a zero-target compensation stage through its outer splice. Tests insert
and remove a matrix before, between and after two fixed limiters, preserving
their already primed history. Insertion promotes settled scalar fixed-delay
history into the new stages with the correct preceding delay for each
stage's input.

A prepared hosted-effect fixture supplies a real 32-sample delay through
the public engine host interface. Sampler tracks, half-mix matrix edits
and enabled/bypassed hosted slots cancel throughout repeated changes; the
factory creates one instance, and guarded callbacks make zero allocator
calls. This is engine hosting evidence, not native plugin/OS verification.

## Transition policy and bounds

Each retarget freezes the current tap mixture at its current weights and
starts a new 5 ms linear fade to the latest whole-sample destination. An
unchanged target does not restart a fade. There is no queue or discarded
partially audible destination. Five milliseconds after the **last** edit,
the output uses exactly its requested tap; reset immediately takes the
latest settings and clears all historical tap contributions.

The prepared vector reserves one entry for every legal whole-sample tap.
Entries with the same delay merge, so edits cannot grow storage beyond
that bound. Matrix lines have at most 2401 entries at 48 kHz and 19201 at
the utility's 384 kHz cap. Dry and compensation lines reserve their host
latency bounds on the control side. Reads cost one tap when settled, two
for an ordinary edit, and a bounded number of retained taps under repeated
edits. Pathological uninterrupted edits can reach the prepared bound; this
is not a constant two-tap CPU guarantee. Old taps remain in tail/gap
accounting until the fade finishes. Clearing vectors retains their storage.
The allocator regression traverses every 0..=2400 tap twice with one edit
per processed sample, including slot mix/reset/bypass, with zero allocations
or frees after prepare. A separate limiter dry/wet regression preserves
its existing destination-joining policy; the matrix policy does not change
the limiter processor's rapid look-ahead behavior. Bypassed slot tails also
retain old dry taps through their transition, and reset snaps the final
configured delay before an impulse is processed.

PDC continues to report the latest configured minimum channel delay, and
the deliberate stereo difference is preserved. On eligible reference
routes its audible compensation follows the causal serial transfer of
the dry/wet transitions, including their downstream timing.
The existing project/automation/parameter unions, persisted indices,
seven transfers, control history and format remain unchanged. Matrix delay
automation still reports **Changes the latency**; latency automation is not
enabled by these repairs.

## Causal routing contract and remaining limits

The control side records the selected longest shared-latency route as
ordered stages keyed by effect/instrument identity. Equal current delays
prefer the route with greater prepared reach, retaining a zero-delay
matrix reference. Compensation factors out an identical incoming prefix,
or a fixed instrument/plugin prefix that can be removed from the reference's
fixed leading stages. Remaining matrix, limiter and fixed group-delay
stages process in the same order as the reference. Limiter stages retain
the limiter's destination-joining policy; matrix stages preserve their
audible tap mixtures. A fixed distortion stage represents its group delay,
not its nonlinear/FIR transfer.

Stage identities, bounds and order let unchanged prepared paths move
between plans. Changed paths are prepared on the control side; matching
stage histories are copied into prepared storage during handover. Removed
storage stays in the retired plan until control-side collection. An
aggregate raw-input ring remains primed for scalar fallback and route-shape
changes. No callback creates or destroys a stage, ring or tap vector.

Each stage reserves a power-of-two ring of `P = next_power_of_two(maximum+1)`
stereo frames and a tap vector with capacity P. The aggregate ring/vector
uses the host's prepared reach, at most one second before power-of-two
rounding. Serial mirroring is selected only if the sum of stage maxima
fits that one-second bound. Storage is additive across stages, not a
flattened product of tap mixtures. Per-frame reads cost the sum of active
stage taps, plus one raw history write; retarget/history-copy work is also
bounded by prepared storage. There is no new worst-case device deadline
measurement. A zero-delay reference consumes prepared history and work
even before its first delayed edit; a project without delayed processors
still constructs no compensation lines.

There is no settings queue. Each matrix stage reaches its exact latest tap
5 ms after its last edit. At the route output, completion additionally
waits for downstream shared delay to carry the last upstream fade samples.
Overlapping edits in an unchanged eligible serial route retain all currently
audible contributions and synchronize wet, dry and compensation clocks.

**Exact transient cancellation has a defined boundary:** two independently
varying branches generally cannot be factored into one another by a causal
delay; their tap filters need not commute or have an inverse. They use the
existing scalar compensation policy. A change of dominant reference route,
effect reordering/moving, or insertion/removal during an unfinished tap fade
also does not promise exact phase cancellation. Settled latency alignment
and deliberate channel offset remain correct, but these edits can leave
an audible transient phase difference. Routes whose _prepared maxima_
exceed one second also use scalar compensation, even if their current
settings are shorter. The repaired serial edit regressions and settled
splice regressions do not imply a universal graph-transition guarantee.

## First-repair verification (99792c90)

All native commands use Git Bash's `scripts/msvc-env.sh`,
`CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=target/utility-repairs-native`, and
`TS_RS_EXPORT_DIR=target/utility-repairs-bindings`, resolved under this
worktree. Cargo runs are sequential. Desktop dependencies were installed
with the checked-in pnpm lockfile in this worktree. TypeScript metadata is
under `target/utility-repairs-ui`.

- `cargo test -p windfall-dsp --test dsp`: **149 passed**, three existing
  timing/demo tests ignored. Includes all utility transfer measurements,
  allocator guards, existing partition/reset tests and new wake/retarget
  regressions.
- `cargo test -p windfall-engine --test engine effects`: **37 passed**,
  183 unrelated tests filtered out. Includes the new rapid PDC cancellation
  and enabled/bypassed matrix-plus-distortion 128-sample chain checks,
  existing routing/render/automation cases and the callback allocator guard.
- `cargo test -p windfall-engine --lib rack::tests`: **three passed**,
  covering compensation taps, delayed fades and state history transfer.
- Desktop Vitest for `features/effects/helpers.test.ts`,
  `features/mixer/effects.test.tsx`, and `features/plugins/plugins.test.tsx`
  with `--maxWorkers=1`: **73 passed**.
- App and Vite TypeScript projects: **passed**.
- ESLint and Prettier checks on all four affected TypeScript files:
  **passed**.

- `cargo test -p windfall-dsp --test dsp utility_repairs`: **six passed**
  after the final tail-accounting refinement.
- `cargo clippy -p windfall-dsp -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed**.
- `git diff --check`: **passed**.

No full workspace suite was duplicated.

## Second-repair verification (integrated bde3fd77 source)

The same worktree-local native target and export directory are reused,
with `CARGO_BUILD_JOBS=1` and one sequential Cargo process. The four red
reproductions completed before the server restart; post-restart checks
are new completed commands, not claims about cancelled handles.

Completed scoped checks:

- `cargo test -p windfall-dsp --test dsp`: **150 passed**, three existing
  timing/demo tests ignored. Includes the new tail/gap impulse bounds and
  all original transfers, partition/reset properties and allocator guards.
- `cargo test -p windfall-engine --test engine effects`: **47 passed**,
  183 unrelated tests filtered out. Includes all ten R2 regressions,
  unchanged limiter policies, utility render/automation/routing cases and
  the existing busy callback allocator guard.
- `cargo test -p windfall-engine --lib rack::tests`: **three passed**.
- `cargo test -p windfall-engine --lib state::tests`: **three passed**.
- `cargo test -p windfall-engine --lib plugins::tests`: **eight passed**.
- New edge-path and hosted-delay callback guards: **zero alloc/realloc/free
  calls** during plan adoption, zero-to-delay/rapid serial edits, insertion,
  mix/bypass, stop/seek/replay and retirement. Controller collection runs
  outside the guard. The edge fixture remains sounding through retirement.
- `cargo clippy -p windfall-dsp -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed after the final insertion-wait fix**.
- Prettier on both utility documents and `git diff --check`: **passed**.

No second full workspace, desktop build or UI suite was run. This round
changes no TypeScript/UI source; the first-repair UI checks above retain
their commit provenance. Exact shared-WASM regeneration/parity remains
with the parent, as requested.

Generated bindings/document WASM, root parity/integration documentation,
the project store/flow changes, and the unrelated automation fixture count
remain with the parent. This commit contains source and documentation only.
The badge does not include live native plugin latency: project bindings do
not contain that runtime figure. Post-repair worst-case device deadline,
hardware listening and platform walkthroughs remain external checks; the
original throughput measurements retain their original provenance.
