# Utility effect review repairs

Repair branch: `gpt/t3-utility-repairs`, based on
`055365868ef59a2c5b58833737b5142bc58971f2`. The three P2 findings were pinned
to `5254a40ef7a35aed202f17df7092c10d184c7bb8`. Tests below reproduced the
findings on the repair base before their corresponding fixes.
The first repair is `99792c909c82e69c49dea60f223759aa99f1a72c`. The second
round integrates parent `bde3fd77dacdd9c1d4c92cf3a77e5c4d1d1eab55`, which
already contains that repair and the other accepted parent changes.
The second repair is `1e0fa52512031c351da0a1c6541786abec5bb805`.
The third review independently executed the four pinned R2 cases at that
commit successfully, then identified the three history/splice gaps below.

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

## Third review: compiled reproductions

Each exact R3 regression failed natively before its corresponding fix.
The values here are measured test failures, replacing the review's predicted
source traces. The earlier R1/R2 regressions and their thresholds remain.

1. **Departing fixed stages:** opposite 0.25-amplitude 500 Hz sampler tracks,
   with one containing a settled 1/1 ms matrix followed by an idle 1 ms
   limiter, were processed for 4092 frames. Limiter removal produced first
   residual `-0.35355338`, with peak `0.47606185` over the splice. Layout now
   retains departing limiter, hosted-plugin and other fixed-delay stages at
   a zero target while the rack fades them out. Retired plugin latency comes
   from the held native ledger even after its binding leaves the project.
   Tests remove enabled/bypassed limiters and real hosted 32-frame delays
   before, between and after two matrices, with irregular blocks and zero
   callback allocator calls.
2. **Reference history:** three constant-one tracks A/B/C, with A at 1/1 ms,
   B at 0/0 ms and C plain, were processed for 4092 frames. Setting B to
   2/2 ms reduced the expected sum 3 to `2.6041667`. New staged references
   now inherit the aggregate raw input history, and subsequent unmatched
   stages reconstruct their required input from the preceding retained
   stage. Only actually retained causal history is marked valid. A repeated
   short-history reference test then exposed scalar fallback reading beyond
   its history (`2.975` instead of 3); that path now waits for valid history
   too. Repeated reference switches preserve unity from both 64 and 4092
   priming frames, including larger pending targets and irregular blocks.
3. **Short history after reset:** a prepared full-wet 0/0 ms slot processed
   64 constant-one samples, then requested 50/50 ms. The minimum output
   was zero. Matrix wet and dry taps now retain their previous audible
   transfer until both new channel taps have enough actual input history.
   Compensation uses the same readiness threshold, separate from the
   minimum-channel PDC figure. Tests preserve unity through the wait and
   fade at full/half/dry/bypassed policies, reset/reprepare and irregular
   blocks. An impulse proves the final 2400-frame tap is applied. Opposite
   sampler tracks cancel through a 5-to-50 ms request during the wait,
   repeated reset/replay and eventual shorter targets; solo output remains
   audible. These callbacks also make zero alloc/realloc/free calls.

Source addition/replay, channel and matrix moves, matrix insertion/removal,
and source removal have constant-input and guarded callback coverage. The
transport's intentional restart fade and actual earlier source silence are
kept distinct from unavailable history in a newly constructed delay line.

A further compiled solo test exposed a departing splice interrupted by an
unrelated plan: at removal frame 80, changing an unused track's fader
produced adjacent step `0.09216105`, above the 0.04 bound for this
0.25-amplitude 500 Hz tone plus its 240-frame splice. Cancellation alone
had missed this because both the rack and compensation dropped together.
With the parent's narrow `Plan::keep_leaving` reservation, each effect now
shares a progress marker with its rack slot. Heard departures keep their
definitions across successive plans; the fade is not restarted. Completion
is published without a lock, and the next control-side plan excludes the
definition and its latency record. The native owner then retires through
the existing control-side garbage collection path.

The marker carries no owner handle or authority. A concrete prepared slot
also tracks whether that owner has actually been heard. Never-heard slots
are omitted or dropped without a departure fade, so speculative preparation
and a revised native factory cannot activate a future owner as a leaver.
Departing plugin latency is stored separately from active reuse metadata;
restoring a removed native id prepares a new active owner. Tests check 40
successive unrelated plans, eventual single native destruction off callback,
40 speculative preparations with zero native processing and all 40 retired,
native revision replacement, restored ids, and progress after a track move
or unrelated factory change. Active project slots retain their existing
limit; departing history lasts only its actual finite splice, rather than
accumulating completed or never-heard definitions across edits. Pending
message/garbage storage still follows the existing controller queue policy.

## Fourth review: restore during an unfinished removal (3073d220 source)

The review's new tone predictions were reproduced with compiled native
regressions before changing production source. At 48 kHz, a 0.25-amplitude
500 Hz sampler tone was primed for 4092 frames through an idle 1 ms limiter
followed by a 1/1 ms identity matrix. Removing the limiter, processing 80
frames, and restoring the same slot id produced adjacent step
`0.09216105` at edit frame 128. The prepared real 32-frame hosted-delay
counterpart produced `0.07232481` at the same frame. Both exceeded the
existing **0.04** bound. The constant-one restoration regression had proved
fresh native ownership, but concealed this temporal splice cut. A rapid
restore/remove tone regression also failed at `0.46637738` before the fix.

With the approved internal departure hook, a heard outgoing definition stays
serially ahead of a restored active definition. Its finite removal fade
continues from the actual remaining count. The fresh owner runs unheard
until that departure completes and then primes its own full delay from the
post-departure input before beginning its 5 ms insertion fade. Removing an
already partly audible insertion starts at its current outer wet weight;
it does not jump back to a fully wet removal. Same-id parameters and
automation address only the active definition. Leaving definitions keep the
settings of their audible owner, while still receiving the existing native
control-boundary service. No native facade or adoption hook is changed.

The progress marker has an internal generation number, separate from the
persisted effect id and native ownership. Compensation stages and departing
native-latency metadata distinguish the outgoing and current generations,
including a revised 32-to-64-frame native binding and provider revision.
Each serial compensation stage follows its own rack generation's remaining
wait. While validating that serial handover, a compiled overlapping-removal/
restoration regression measured residual
`0.06888252` with an aggregate wait; per-generation waits close that case.
A non-periodic 173 Hz cancellation check also exposed `0.44261545` after a
completed departure: a fresh leading stage had adopted the aggregate tap and
counted the retained downstream matrices twice. A new leading stage ahead of
an unchanged suffix now inherits raw input history at identity; completed
zero-delay departure stages do not add another delay. These are fixes
inside the prepared serial compensation contract, not added phase exceptions.

For a restored id there is at most **one audible outgoing owner and one
current active owner**. The fresh owner cannot become audible until the
predecessor has finished. Removing an unheard fresh owner excludes it from
leaving definitions, so repeated restores cannot build a lineage or restart
the old fade. The predecessor field contains only one progress marker, never
another definition or native handle. A Plan regression performs 1000
restore/remove rounds at `MAX_EFFECT_SLOTS`, verifies at most twice that many
definitions, preserves each original outgoing generation, resolves active
indices to fresh slots, and removes finished predecessors. Native tests
verify live-owner counts and 40 superseded fresh restores with zero processing
and one destruction per preparation. The original 40 speculative-owner guard
also remains unchanged and passing.

Prepared units, rings, tap vectors and latency records are constructed off RT.
Handover moves owners or copies into reserved storage; generation matching
and wait assignment only scan existing bounded plan/stage storage. No callback
creates or destroys an owner or progress marker, nor takes a lock or waits.
As with the prior departure policy, finishing publishes progress; the next
control snapshot omits the completed definition and metadata, and existing
control-side collection destroys its native owner. The retirement regression
includes that snapshot before asserting the unchanged two-created,
one-destroyed ownership result.

The two exact tone tests now measure maximum steps **0.016350782** (limiter)
and **0.016432416** (hosted delay), below the unchanged 0.04 bound. Coverage
also restores fixed processors before, between and after two matrices, uses
irregular and single-frame partitions, removes partial insertions, repeats
restores, checks the final requested delay, preserves unity, and cancels
opposite 173 Hz tracks at full/half/dry/bypassed slot policies. Actual native
parameter and playlist automation probes update only the fresh owner while
the old one remains audible. Revised bindings keep their own latency and
retire the departing native owner only on the control side. All these
callbacks are guarded for zero alloc/realloc/free calls.

## Fifth review: revision-only replacement while restoration waits (7491dfb6 source)

This review's source-equation prediction was reproduced with compiled native
RED before production edits. At 48 kHz, opposite 0.25-amplitude 173 Hz sampler
tracks were primed for 4092 frames, with a real hosted 32-frame delay followed
by a 1/1 ms identity matrix on one track. The hosted slot was removed for 80
frames, restored for another 80, then **only the same factory's revision** was
incremented. The project, binding state, parameter values/layout, slot id and
factory Arc were unchanged. The measured cancellation residual over 400
revision frames was **0.05907429**. A 64-revision-frame remove/restore variant
left **three live native owners**, exceeding the approved bound of two. Both
assertions failed in `cargo test -p windfall-engine --test engine
utility_effects_r5 -- --nocapture`. The 48-frame downstream matrix delays the
first mismatch reaching the output; observing only 64 revision frames would
miss the eventual cancellation failure.

A shared factory Arc exposes its current revision, not the revision a previous
plan prepared. Active effect definitions now carry an internal control-side
snapshot of binding/provider/revision identity. Compilation snapshots initial
identity; installation refreshes it before comparing with the previous frozen
snapshot, including plans compiled before a retry. A revision-only preparation
therefore gets a fresh progress generation. It cannot inherit a superseded
owner's heard marker, departure authority or compensation stage. This adds one
optional identity value per definition, no persisted id or model change, and
no native owner handle. Unchanged native owners and built-in effects retain
their progress across ordinary plans and track moves.

State now adds the actual predecessor's remaining removal count when a concrete
prepared or arriving rack unit begins insertion. Marker equality is no longer
used as proof that the concrete owner was retained. The serial predecessor has
already been adopted at that point, so the fresh owner's wait is exactly that
remaining removal plus its own priming. Retained owners keep their existing
countdown without extending it on intervening plans. Prepared compensation
stages use the new generation and its actual wait, while superseded unheard
units retire through the existing control-side collection path. No native
facade/adoption hook, controller, processor or navigation seam is changed.

The exact cancellation GREEN residual is **0.000000014901161**, below the
unchanged **1e-6** bound. The solo 173 Hz remove/restore variant measures maximum
step **0.006233236**, below the unchanged **0.04** bound. The exact subsequent
remove/restore at revision frame 64 also cancels after the original departure
finishes, stays within two live owners, and destroys each retired owner once
outside the callback. Six R5 tests additionally cover:

- Repeated revision-only replacements before, between and after two matrices,
  with unchanged bindings and alternating prepared 32/64-frame latency.
- Precompiled plans installed after the same factory revision changes, seven
  single-frame callbacks per revision, irregular continuation blocks, and the
  final requested latency.
- Forty revised preparations across twenty pairs of plans, where the first
  plan of each pair is superseded before callback processing. Each such owner
  processes zero blocks, drops once on the control side, and never joins the
  departure lineage. At control collection boundaries only the audible
  outgoing and current active owner remain; final collection leaves one owner.
- Moving the revised native id to the master after its departure has completed,
  preserving constant unity and reusing the current owner. The prior tone,
  automation, move/topology boundaries and speculative-owner tests remain.

All adoption, history, revision, further remove/restore and retirement callbacks
use the existing allocator guard and report zero alloc/realloc/free calls.
Snapshot/hash work occurs on the control side; audio-side predecessor matching
uses the existing track's prepared definitions, and no added callback lock,
blocking wait, owner destruction or unbounded storage is introduced. The
one-outgoing/one-active representation and control-side retirement contract
from R4 remain intact. No new phase or readiness exception is added.

## Sixth review: detached preparation identity (a4b46967 source)

The exact detached precompiled-plan case was compiled RED before production
edits. At 48 kHz, a plan was compiled with factory revision 0, then the same
factory advanced to revision 1. The plan was installed without a stream and
later attached, preparing a real hosted 32-frame delay followed by a 1/1 ms
identity matrix. After 4092 priming frames, an unchanged project update lost
the running native owner. The solo 0.25-amplitude 173 Hz tone measured an
adjacent step of **0.068483695**, exceeding the unchanged **0.04** bound. The
opposite-track version measured cancellation residual **0.2646036**, exceeding
**1e-6**. Both tests failed in
`cargo test -p windfall-engine --lib utility_r6 -- --nocapture`; these are
executed native results, not the review's source-equation predictions.

Detached installation now refreshes the requested binding/provider/revision
snapshot. This alone cannot freeze attachment's identity: the revision can
change again after installation. Each attachment already returns a ledger of
the native identities and latencies actually prepared on the control side.
Plan progress adoption now compares against that frozen preparation ledger,
not the previous plan's possibly older compilation snapshot or its mutable
factory's current revision. Subsequent ordinary and precompiled updates reuse
the running owner when that identity matches; an actual revision replacement
still receives a fresh generation under the R5 policy. Departing records are
excluded from active identity lookup.

The exact GREEN residual is **0**, and the solo maximum step is **0.0056612906**.
The hosted owner continues processing across four unchanged updates, is
prepared exactly once, and retains the reported **80-frame** chain latency.
Removing it preserves cancellation through its splice, then reports the
matrix's **48-frame** latency and retires that native owner once on the control
side. Irregular blocks include 1, 7, 29 and 137 frames.

Four controller-unit regressions cover the exact cancellation and solo cases,
a further revision change after detached installation but before attachment,
and a failed attachment retry followed by a suspended stream's reopening.
The factory records the revision used for each actual prepared owner. The
unheard abandoned owner processes zero blocks and drops once outside audio;
the later owners use revisions 2 and 3, continue processing through unchanged
updates, and retire once when their processors are dropped on the control
side. Tests stay in the controller module because attachment is crate-private;
no public test hook is added.

Every callback in these cases is allocator-guarded for zero alloc/realloc/free
calls. The production change adds only a control-side identity lookup and
snapshot refresh; it adds no callback allocation, lock, wait, native-owner
handle or storage. Immutable plans are not cloned or mutated at attachment.
The approved one-outgoing/one-active bound, routing/automation authority,
control-side retirement and all R1-R5 contracts remain unchanged. Controller
transport/navigation, processor, native facade/adoption and DSP registry
behavior are outside this repair. No additional limitation is introduced to
excuse the detached adoption failure.

## Transition policy and bounds

When history is ready, a retarget freezes the current tap mixture at its
current weights and starts a new 5 ms linear fade to the latest whole-sample
destination. An unchanged target does not restart a fade. If history is
short, one pending setting holds the latest request while the old transfer
continues sounding and collecting input. Repeated requests coalesce into
that setting; they never replace a partially audible destination. An
increase extends readiness, and a shorter target can become ready sooner.
Five milliseconds after the **last** request is both accepted and ready,
the output uses exactly its requested tap. Reset takes the latest configured
taps immediately, clears history and pending contributions, and preserves
the initial configured processor latency.

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
ordered stages keyed by effect/instrument identity and internal effect generation. Equal current delays
prefer the route with greater prepared reach, retaining a zero-delay
matrix reference. Compensation factors out an identical incoming prefix,
or a fixed instrument/plugin prefix that can be removed from the reference's
fixed leading stages. Remaining matrix, limiter and fixed group-delay
stages process in the same order as the reference. Ordinary limiter setting edits retain
the limiter's destination-joining policy; matrix stages and outer removal
splices preserve their audible tap mixtures. A fixed distortion stage represents its group delay,
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

Each matrix stage has one bounded pending target while its input fills and
reaches its exact latest tap 5 ms after that target becomes ready. Prepared
wet/dry counters and compensation validity saturate at their storage bounds.
A request made during an unheard rack insertion also waits for its inner
tap fade to finish before the outer splice starts, keeping compensation
and the resulting outer transfer synchronized. At the route output, completion additionally
waits for downstream shared delay to carry the last upstream fade samples.
Overlapping edits in an unchanged eligible serial route retain all currently
audible contributions and synchronize wet, dry and compensation clocks.
Changing reference topology reconstructs new stage input using a bounded
snapshot of retained preceding tap weights. This preserves constant unity
but is not an inverse of an arbitrary varying filter; the phase limits
below still apply. History reconstruction costs at most the retained frame
count times the preceding stage's prepared tap bound, on plan adoption.

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

## Third-repair verification (1e0fa525 source)

Commands reuse `target/utility-repairs-native` and
`target/utility-repairs-bindings` in this worktree. Every native command
starts with `source scripts/msvc-env.sh`, with `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1`; Cargo processes run sequentially. Exact new-case red
commands were `cargo test -p windfall-dsp --test dsp r3_ -- --nocapture` and
`cargo test -p windfall-engine --test engine utility_effects_r3 -- --nocapture`.
The later interruption red command used the filter
`utility_effects_r3_an_intervening`. All completed before their respective
fixes; none of the reported failures are predicted traces.

- `cargo test -p windfall-dsp --test dsp`: **152 passed**, three existing
  timing/demo tests ignored. DSP source was unchanged by the later Plan
  progress refinement. All original transfers, partitions/reset properties,
  nine repair regressions and prepared allocator bounds remain covered.
- `cargo test -p windfall-engine --test engine effects --quiet`:
  **58 passed**, 183 unrelated tests filtered out, after the final source
  refinement. Includes all eleven R3 tests, the seven original utility
  transfers, R1/R2 routing/automation/render cases, fixed limiter/hosted
  policies and callback allocator guards.
- `cargo test -p windfall-engine --lib rack::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib state::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib plugins::tests --quiet`: **eight passed**.
- `cargo clippy -p windfall-dsp -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed after the final source refinement**.
- `cargo test -p windfall-dsp --test dsp utility_repairs --quiet`:
  **nine passed** after the final engine refinement.
- Prettier on both utility documents and `git diff --check`: **passed**.
- New readiness, reference reconstruction, splice progress, restoration and
  retirement paths: **zero callback alloc/realloc/free calls**. New shared
  progress uses atomics; no lock was added. This is allocator instrumentation
  and source inspection, not an OS scheduler or native plugin hardware test.

No full workspace, desktop build or UI suite was duplicated. The runtime
owner's `adopt_parameters` facade and adoption hooks are outside this patch
and must be retained by the parent integration. Generated descriptors/WASM
and combined native/shared-WASM parity remain with the parent. The explicit
causal graph and one-second prepared-bound limitations above remain; unity
dropouts and premature splice completion are not covered by those limits.

## Fourth-repair verification (3073d220 source)

All commands reuse this bound worktree's `target/utility-repairs-native` and
`target/utility-repairs-bindings`. Native commands use
`source scripts/msvc-env.sh`, `CARGO_BUILD_JOBS=1`, and
`RUST_TEST_THREADS=1`, with only one Cargo process at a time.

The original exact-case RED command was
`cargo test -p windfall-engine --test engine utility_effects_r4 -- --nocapture`:
both new exact tone tests failed at the measured values above before source
edits. The additional original-source rapid RED used the filter
`utility_effects_r4_repeated`. During implementation, the non-periodic
cancellation and differing restoration clocks were also compiled RED before
their respective history/wait fixes; the latter command used the filter
`utility_effects_r4_overlapping`.

Completed fixed-source checks:

- `cargo test -p windfall-engine --test engine effects --quiet`:
  **65 passed**, 183 unrelated tests filtered out. Includes all prior R1/R2/R3
  regressions, seven R4 restoration tests, original utility behavior,
  limiter policies and callback allocator guards.
- `cargo test -p windfall-engine --test engine utility_effects_r4 -- --nocapture`:
  **seven passed**, with the exact GREEN tone maxima printed above.
- `cargo test -p windfall-engine --lib repeated_restores --quiet`:
  **one passed**, including 1000 rounds at the maximum active slot count.
- `cargo test -p windfall-engine --lib rack::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib state::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib plugins::tests --quiet`: **eight passed**.
- `cargo test -p windfall-dsp --test dsp utility_repairs --quiet`:
  **nine passed**. DSP source is unchanged in this followup.
- `cargo clippy -p windfall-dsp -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed**. The new Plan test was adjusted
  to Clippy's constant-array chunk API without changing its assertions.
- Prettier on both utility documents and `git diff --check`: **passed**.
- New restoration, per-generation waits, parameter/automation routing,
  speculative-owner exclusion, history transfer and control-side retirement
  callbacks: **zero alloc/realloc/free calls**. Progress and test probes use
  atomics; source inspection finds no added callback lock or wait.

No full workspace, desktop or UI suite was duplicated. No parent commit,
native adoption facade/hook, controller, processor, navigation field,
persisted id/model, descriptor or generated artifact is changed. Generated
WASM and combined acceptance remain with the parent. Existing independent
varying-branch/topology phase limits and the one-second compensation fallback
remain as previously documented; the solo restoration splice cut, completed
identity-prefix history, and differing serial restoration clocks are fixed
without another exception. Hardware listening and worst-case device deadline
measurements are not claimed.

## Fifth-repair verification (7491dfb6 source)

Checks reuse the existing worktree-local `target/utility-repairs-native` and
`target/utility-repairs-bindings`, with `source scripts/msvc-env.sh`,
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, and sequential Cargo processes.
The exact RED command before production edits and final focused GREEN command
were `cargo test -p windfall-engine --test engine utility_effects_r5 -- --nocapture`.
No source-equation prediction is presented as executed failure evidence.

Completed fixed-source checks:

- `cargo test -p windfall-engine --test engine utility_effects_r5 -- --nocapture`:
  **six passed**, including printed exact cancellation and solo maxima above.
- `cargo test -p windfall-engine --test engine effects --quiet`:
  **71 passed**, 183 unrelated tests filtered out. Retains all R1-R4 closures,
  unchanged tone/cancellation bounds, utility/limiter behavior and allocator
  guards, plus all six R5 cases.
- `cargo test -p windfall-engine --lib repeated_restores --quiet`: **one passed**.
- `cargo test -p windfall-engine --lib rack::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib state::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib plugins::tests --quiet`: **eight passed**.
- `cargo test -p windfall-dsp --test dsp utility_repairs --quiet`: **nine passed**.
  This followup changes no DSP source.
- `cargo clippy -p windfall-dsp -p windfall-engine --all-targets -- -D warnings`
  and `cargo fmt --all --check`: **passed after the final source edits**.
- Prettier on both utility documents and `git diff --check`: **passed**.
- Every guarded new revision/precompiled-plan adoption, continuation,
  remove/restore, move and retirement callback: **zero alloc/realloc/free calls**.

This is a source-only followup atop 7491dfb6; no parent imports, amendment,
generated artifact, new review thread, push or release is included. Only
Plan/State lifecycle source, owned regression tests and this repair document
change. EffectSlot policies, DSP registries, native adoption facade/hook,
controller/processor and navigation fields remain with their owners. Combined
acceptance/artifacts remain with the parent. Existing independent varying-branch
and topology phase boundaries and the one-second fallback are unchanged; the
revision-only readiness/cancellation and two-owner failures are fixed without
new exceptions. Hardware listening, native worst-case device deadlines and
combined shared-WASM parity are not claimed.

## Sixth-repair verification (a4b46967 source)

All native commands use the existing worktree-local
`target/utility-repairs-native` and `target/utility-repairs-bindings`,
`source scripts/msvc-env.sh`, `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1`, with one Cargo process at a time.

The exact two-case RED and initial GREEN command was
`cargo test -p windfall-engine --lib utility_r6 -- --nocapture`.
After adding the attachment variants, that command passed **four tests**.
Final fixed-source checks:

- `cargo test -p windfall-engine --lib controller::tests -- --nocapture`:
  **12 passed**, including all four R6 tests and the eight existing stream,
  transport and automation-state regressions. Exact printed GREEN values
  match those above.
- `cargo test -p windfall-engine --test engine effects --quiet`:
  **71 passed**, 183 unrelated tests filtered out. Preserves all original
  utility/limiter policies, R1-R5 closures, native-owner authority and guarded
  restoration, automation, revision, history and retirement edge paths.
- `cargo test -p windfall-engine --lib repeated_restores --quiet`: **one passed**,
  including 1000 rounds at the maximum active slot count.
- `cargo test -p windfall-engine --lib rack::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib state::tests --quiet`: **three passed**.
- `cargo test -p windfall-engine --lib plugins::tests --quiet`: **eight passed**.
- `cargo test -p windfall-dsp --test dsp utility_repairs --quiet`: **nine passed**.
  DSP source is unchanged. These final suites cover **107 distinct native
  tests**; the focused four-test rerun is not added again to that count.
- `cargo clippy -p windfall-engine --all-targets -- -D warnings` and
  `cargo fmt --all --check`: **passed after the final source edits**.
- Prettier on this repair document and `git diff --check`: **passed**.
- Every new detached adoption, unchanged update, attachment retry, reopening
  and removal callback asserts **zero alloc/realloc/free calls**.

This source-only incremental repair is atop immutable a4b46967. Production
edits are confined to controller identity preparation/adoption and the
Plan/Ledger identity seam; tests are in the existing controller unit module.
No parent import, amendment, artifact, new review thread, push or release is
included. DSP registries, native facade/adoption hook, processor and
transport/navigation behavior are unchanged. No full workspace, desktop or UI
suite was repeated. Combined artifacts and acceptance remain with the parent.
Existing independent varying-branch/topology boundaries and the one-second
fallback remain; the detached native-owner loss is fixed without a new
exception. Hardware listening, device deadline measurements and combined
shared-WASM parity are not claimed.

## CI portability: DC step reference (95a0c48f source)

CI run `37740818762`, pinned to
`95a0c48f588c21a0378707ff0decb6d4dc2eb24f`, passed strict Clippy on macOS
and Ubuntu but failed
`utilities::dc_rejection_cutoff_and_step_response_are_rate_independent`.
Both actual remote logs report **0.37561557** measured versus **0.37561253**
expected at the unchanged **2e-6** bound, with **151 passed, one failed and
three ignored**. The inspected logs are
`C:/Users/ewhee/AppData/Local/Temp/windfall-95a-macos-ci.log` (failure around
2050-2064) and `windfall-95a-ubuntu-ci.log` (around 2448-2460). These are
executed remote failures; no local macOS or Ubuntu execution is claimed.

Before source edits, the unchanged exact test passed on Windows with the
existing private Cargo cache. A bounded standalone native diagnostic then
reproduced the failing reference mechanism, using the existing worktree-local
DSP library and only nine rate/cutoff combinations. At 44.1 kHz, 1 Hz and sample
441, the f32 pole is **0.99985754**, bits `0x3f7ff6aa`. Explicit f32
exponentiation by squaring yields **0.37561253**, exactly the remote expected
value. The actual processor yields **0.37561557**, while f64 exponentiation
of that same quantized pole also yields **0.37561557**. The diagnostic's
unchanged 2e-6 comparison failed with error **3.0398369e-6** and native Rust
assertion exit 101. It also exposed f32-squaring reference errors above the
bound at 44.1 kHz/5 Hz and 96 kHz/1 Hz. Windows' native f32 `powi` gave
**0.37561554** for the first case, explaining why the old test passed locally.
This matches a platform-dependent exponentiation rounding mechanism; remote
compiler disassembly and a particular remote libm implementation were not
inspected. The DSP already maintains f64 state and is unchanged.

The corrected closed-form step reference promotes the intended f32 coefficient
to f64 before exponentiation, and promotes the actual f32 input amplitude
before its normalization. It compares the final output at the original
**2e-6** bound. All nine measured Windows outputs match this f64 reference
after its final f32 conversion. No recurrence copied from the processor is
used as the committed oracle.

A separate physical response check computes the ideal pole
`r = exp(-2π cutoff / rate)` entirely in f64. Coefficient quantization must be
accounted for: the 96 kHz/5 Hz step differs from that ideal by approximately
**7.24e-6**, despite matching the intended quantized transfer. For the tested
poles in `[0.5, 1)`, one f32 coefficient ULP is exactly `f32::EPSILON / 2`.
The test explicitly bounds the intended f32 pole's deviation from the ideal
by this quantum. It then propagates the quantum through the monotone analytic
step `S(r) = x (1+r)/2 r^n`, using
`max(S(r+ULP)-S(r), S(r)-S(r-ULP))` as the coefficient error budget. The
original **2e-6** state/output bound remains after this separately derived
coefficient contribution. This additional assertion neither substitutes a
looser bound for the quantized transfer nor permits unbounded coefficient
error.

The original first-sample **1e-6**, two-second DC rejection **1e-5**, physical
cutoff measurement **0.02 dB** around -3.0103 dB, zero latency and exact
declared-tail silence assertions are preserved. The cutoff measurement still
uses independently generated f64 sinusoidal phase and f64 RMS. No test is
skipped, and the existing ignored timing benchmark is unchanged.

Completed fixed-source Windows checks use
`target/utility-repairs-native`, `target/utility-repairs-bindings`,
`source scripts/msvc-env.sh`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`
and one Cargo process:

- `cargo test -p windfall-dsp --test dsp utilities::dc_rejection_cutoff_and_step_response_are_rate_independent --locked -- --nocapture`:
  **one passed**, covering all nine rate/cutoff combinations.
- `cargo test -p windfall-dsp --test dsp utilities:: --locked --quiet`:
  **18 passed**, one existing timing benchmark ignored.
- `cargo test -p windfall-dsp --test dsp utility_repairs:: --locked --quiet`:
  **nine passed**. The two final suites cover **27 distinct tests**; the
  focused one-test run is not counted again.
- `cargo clippy -p windfall-dsp --all-targets --locked -- -D warnings` and
  `cargo fmt --all --check`: **passed**.
- Prettier on this repair document and `git diff --check`: **passed**.

Only the exact existing utility test's numerical reference/physical error
accounting and this evidence document change. Product DSP, engine lifecycle,
E1 tests, generated artifacts and prior utility contracts are untouched.
The incremental source-only repair is atop immutable 6d80370e; no parent
import, amendment, push or release is included. Parent-owned CI must rerun
the corrected test on macOS and Ubuntu before either platform is reported
GREEN. Hardware, device deadlines and combined parity are not claimed.
