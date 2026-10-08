# Utility effect review repairs

Repair branch: `gpt/t3-utility-repairs`, based on
`055365868ef59a2c5b58833737b5142bc58971f2`. The three P2 findings were pinned
to `5254a40ef7a35aed202f17df7092c10d184c7bb8`. Tests below reproduced the
findings on the repair base before their corresponding fixes.

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
its audible compensation transition follows the same requested delays as
the dry/wet transition. The deliberate stereo difference is preserved.
The existing project/automation/parameter unions, persisted indices,
seven transfers, control history and format remain unchanged. Matrix delay
automation still reports **Changes the latency**; latency automation is not
enabled by these repairs.

## Verification

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

Generated bindings/document WASM, root parity/integration documentation,
the project store/flow changes, and the unrelated automation fixture count
remain with the parent. This commit contains source and documentation only.
The badge does not include live native plugin latency: project bindings do
not contain that runtime figure. Post-repair worst-case device deadline,
hardware listening and platform walkthroughs remain external checks; the
original throughput measurements retain their original provenance.
