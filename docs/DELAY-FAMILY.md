# E3 delay-family source contract

Source checkpoint; no effect registry, project, engine, IPC, generated bindings,
UI, parity or release changes. Ownership is confined to the two new public DSP
modules, their private submodules, this document, the auto-discovered integration
test, and two `pub mod` declarations. Existing effects retain their indices.

## Public references and scope

Read 2026-10-08: [Image-Line Delay Bank manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Delay%20Bank.htm)
describes eight units, signed sends to the next unit, input/output balance,
pre/post filtering, eight filter responses, up to three sections, stereo offset,
stereo feedback modes, and echo granulation. Its filter gain range is 0.1–2;
it does not specify a numerical delay-time range. Windfall supplies its own
transfers, explicit time range and headroom policy below. Oversampling and
granulation are not in this first source checkpoint.

[Image-Line Multiband Delay manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Multiband%20Delay.htm)
describes sixteen nominal 20 Hz–20 kHz bands, independent delay (0–1000 ms),
level (0–1) and pan, a 100 ms scale range, and several splitter responses.
Windfall implements two explicitly complementary splitter responses. A linear
phase splitter, eight curve snapshots/morph, harmonic semitone delay/weighting,
pitch-glide mode, and wet compression/saturation/HP options remain subsequent
work. These sources describe controls, not proprietary algorithms. No assets,
presets or sound-identical claims are made.

## EchoBank

Exactly eight units, numbered 0–7. Each receives
`input_gain * global_input * balanced(dry) + next_send[i-1] * node[i-1]`.
Here `node` is the processed, enabled, balanced unit signal before its main
output gain. Main output sums `output_gain[i] * node[i]`; next sends are
independent of main-output level, so intermediate serial units can be unheard.
There is no cyclic cross-unit graph; feedback cycles are local stereo delay
loops. Unit 7 has no next destination; its next control is reserved and has no
audible destination. Setting all next sends to zero is parallel routing;
zeroing inputs after the first and enabling next sends is serial routing.
Any mixture gives a feed-forward chain with parallel taps. Signed sends and
output levels permit phase inversion. Unit enable ramps its dry injection and
output contribution; linked input, history and filters continue running.

Input and feedback each have separate filter controls: off, lowpass, bandpass,
notch, highpass, low shelf, peak, high shelf; frequency 20–20000 Hz; Q 0.5–10;
shelf/peak gain −18–18 dB; gain 0.1–2; one, two or three sections. Reuse the
existing public SVF filter-family transfer. The off path is unity before gain.
All sections run continuously and their selection is ramped. Input filtering
can move before/after the delay using ramps at both the write and read stages.
For post weight `w`, the transient is `(1-w)*D((1-w)*Fpre(x)+w*x)` plus
`w*Fpost(D((1-w)*Fpre(x)+w*x))`; it includes dry and cascaded filter terms
midway through the edit. It is exactly pre-filtered at zero and post-filtered
at one, without allocating a second full history. Feedback filtering applies only to recirculation;
the first echo is consequently clean of the feedback filter.

Time is 1–24000 ms or the existing fourteen `NoteDivision` values at 10–1000
BPM; longest whole note at 10 BPM is 24000 ms, without truncation. Stereo offset
is −1000–1000 ms, extending one channel, so preparation reserves 25 seconds.
Time is rounded to whole samples, minimum one, and a left impulse at frame 0
appears at frame `round(time_seconds * actual_rate)` on the left. Normal
feedback preserves channels. Windfall inverted swaps the injected channels,
and repeats stay on that swapped side; ping-pong preserves injection and swaps
on every recirculation. This is Windfall's explicitly defined channel-inversion
behavior; the manual does not specify a sample equation. Signed output and send
controls provide polarity inversion. Stereo separation scales the side signal
from zero (mono) to one (original stereo). Balance preserves unity at center
and attenuates the opposite side at either endpoint, without crossfeed.
Its explicit law in both processors is
`[L * (1-max(pan,0)), R * (1+min(pan,0))]`.

Gain/pan/mode/enable/filter edits use frame-count ramps; time edits use two
fixed taps and a latest pending request. An ongoing fade finishes before the
pending fade starts. Its first frame retains the old tap exactly. No tap arena
grows under rapid automation; at most two reads per channel per unit. New taps
wait for valid history after a live edit. Reset discards history and applies the
latest requested values on the first nonempty frame. Empty process calls and
duplicate writes do not advance or restart anything.

The feedback range is 0–1, multiplied by the global feedback amount. Filters
can boost, and unity feedback may sustain. The stored loop signal is bounded
to ±64 after filtering/gain and summation; processor inputs are sanitized to
±1000 and nonfinite values to zero. This explicit hard headroom boundary may
distort extreme settings. No guarantee of decay is made for a boosting loop.
`tail_samples()` is `usize::MAX` while feedback is nonzero or fading; zero
feedback reports a conservative filter/chain/history horizon. `gap_samples()`
also covers filter history and every possible unit hop. No host should treat
an unlimited tail as a duration to allocate. User-selected render horizons
remain required for sustained feedback.

Echo times are musical processing, not algorithmic PDC: latency is zero.
Warm-up/readiness reports the longest enabled input-to-output tap path,
including signed links and current/target ramp values, separate from PDC.
Unused parallel units do not add a serial warm-up penalty.
Host bypass accounting must retain wet tails until its fade reaches zero and
clear the processor on dormant wake (the existing utility repair contract).
Direct processors expose no slot bypass. Actual EffectSlot/engine integration
and its bypass regressions wait for the parent's serialized registry window.

## FrequencyDelay

Exactly sixteen independent stereo histories, delays 0–1000 ms, levels 0–1,
pan −1–1 and enabled flags. Deselecting a band crossfades to its unprocessed
split signal while continuing its delay history. Global dry/wet 0–1, feedback
0–1, scale −1–1, short range toggle, bandwidth 0.5–2 and splitter choice are
stable descriptor-addressed controls. Positive scale gives `scale * delay`;
negative scale gives `abs(scale) * (1000 - delay)`; zero gives zero time. Short
range multiplies by 0.1. Delay zero passes the current band signal; feedback
uses the preceding output sample to avoid an algebraic loop. Delayed feedback
uses the same selected tap as the audible echo. All feedback writes have the
same ±64 boundary. Gains and bandwidth are smoothed in signal/coefficient
space, independently of block size. Tempo has no effect on this processor.

Nominal crossover `i` is `20 * 1000^((i+1)/16)` Hz, `i=0..14`. Bandwidth scales
log distance from the geometric spectrum center; cutoffs are capped at 0.49
of the actual rate with the shared SVF math's positive normalized floor. At
low rates coincident cutoffs yield zero-width bands; indices/count never change.
First and last bands extend to DC and Nyquist to retain all input energy.

Let `L_i(z) = c_i(1+z^-1)/(1-(1-2c_i)z^-1)`, with
`c_i = tan(pi*f_i/rate)/(1+tan(pi*f_i/rate))`. Gentle uses `C_i=L_i`; steep uses
`C_i=L_i^2`. Both have frequency-dependent phase. The bands are `B_0=C_0`,
`B_i=C_i-C_(i-1)` for 1–14, `B_15=1-C_14`. Thus `sum(B_i)=1` by telescoping,
including during cutoff and splitter interpolation. This is not sixteen
unrelated bandpasses and not an assertion of linear phase. The neutral setting
(all delays zero, feedback zero, levels one, pans center) reconstructs the
original normalized audio samples within floating point summation error in
the linear headroom region. The explicit ±64 internal bounds take priority
for extreme signals. Equal delays produce
a correspondingly delayed unity signal. Independent tests use analytic complex
transfer functions and impulse sums rather than the production splitter.

PDC is zero; per-band delay is intentional. Warm-up/readiness is the longest
active/pending tap. Tail and gap include crossover decay, tap transitions and
old history. Nonzero feedback has an unlimited conservative tail. Reset clears
all crossover, last-output and valid-history state without touching heap owners.

## Storage and API

Sample rates: finite 1–384000 Hz are supported; finite out-of-range rates clamp;
NaN/Inf fall back to 48000 Hz. `actual_sample_rate()` exposes this policy.
Constructors hold empty rings; only preparation allocates histories, replacing
and releasing all old histories on the control side. Exact-length ring storage
is `ceil(seconds * actual_rate)+1` f32 values per channel; no power-of-two padding
or uncharged tap vectors. Reset invalidates by counters in constant time.
`prepared_bytes()` includes the entire inline processor and heap histories;
allocator bookkeeping is excluded. `preparation_bytes(rate)` remains a final
payload estimate for compatibility; an estimate alone does not admit memory or
guarantee allocation. `preparation_requirements(rate)` and `try_prepare` supply
checked sizing, peak accounting and observable refusal. Preparing at a lower
rate replaces every ring instead of retaining high-rate capacity.
No process/reset/set_params/set_tempo allocation, reallocation, free, lock, wait
or IO; no heap clones. One caller-owned block at a time. Dropping is control-side.

Public modules: `echo_bank::{EchoBank, EchoBankParams, EchoUnitParams,
EchoFilterParams, EchoFilterMode, EchoFeedbackMode}` and
`frequency_delay::{FrequencyDelay, FrequencyDelayParams, FrequencyBandParams,
FrequencySplit}`. Both implement `Effect`, all complete settings implement
`ParamSet`, all persisted structs/enums derive serde and TS without autoexport.
Descriptor order is explicit in source and tested against nested JSON paths.

Both processor module paths also expose the same owned preparation types:
`PreparationBudget`, `PreparationRequirements`, `PreparationError`,
`PreparationRefusal`, `PreparationStatus`. The processor methods are:

```rust
fn preparation_requirements(&self, sample_rate: f32)
    -> Result<PreparationRequirements, PreparationError>;
fn try_prepare(&mut self, sample_rate: f32, max_block: usize,
    budget: PreparationBudget) -> Result<(), PreparationError>;
fn preparation_status(&self) -> PreparationStatus;
```

`PreparationRequirements` holds the sanitized `sample_rate`, `retained_bytes`
and `peak_bytes`. `PreparationBudget` has independent `retained_bytes` and
`peak_bytes` limits. Retained payload includes the inline processor and new
history capacities. Peak adds all existing history capacities and the fixed
staging headers (640 bytes for sixteen histories, 1280 for thirty-two on this
64-bit target). Ordinary call-stack temporaries and allocator bookkeeping are
excluded; hosts must reserve their own overhead/headroom. Every size addition,
multiplication and each Vec's `isize` pointer-offset limit is checked before
reservation. `try_reserve_exact` allocates empty vectors fallibly, then resize
initializes within reserved capacity; there is no `vec![0; capacity]` abort path.
Actual capacities are checked against admission limits before publishing.

All sixteen/thirty-two histories stage before any live sample rate, parameter,
filter, ramp, tap, history or clock mutation. On success, histories swap and
fixed inline preparation/reset applies the existing sanitized parameter target
and tempo. Old histories retire on the calling control thread. On failure,
partial staged histories retire there and the previous prepared processor
remains usable with exactly the same audio state. The only live change on
refusal is its copyable status latch; no time, rate, band or unit is shortened.

Errors are `CapacityOverflow { maximum_delay_samples, history_count }`,
`RetainedBudgetExceeded { required_bytes, budget_bytes }`,
`PeakBudgetExceeded { required_bytes, budget_bytes }`, or
`ReservationFailed { history_index, requested_bytes }`. The latter identifies
`2 * unit_or_band + channel`, left then right. Retained-budget refusal takes
precedence if both limits fail. Errors implement Display/Error and own no heap
data. `PreparationStatus` exposes `is_prepared` and `last_refusal`, an optional
`PreparationRefusal { sample_rate, error }` recording the sanitized requested
clock that was refused. `actual_sample_rate()` continues to report the old
clock. A first refusal leaves `is_prepared=false` and the existing sanitized
passthrough behavior, rather than installing a different clock. Reset, no-ops,
setters and processing preserve the latch; only successful preparation clears
it. `preparation_requirements` is a read-only query and does not latch errors.

The existing `Effect::prepare` signature is unchanged and delegates to
`try_prepare` with `PreparationBudget::UNLIMITED`. It too retains prior state
and latches any refusal. Legacy callers must check `preparation_status()` off
thread before treating a preparation as successful. Concrete host integration
must use `try_prepare` with actual admitted limits, not the unlimited wrapper.
`max_block` retains the Effect argument shape; these frame-based processors do
not allocate block-sized scratch. None of these preparation methods belongs in
an audio callback. Static `preparation_bytes` saturates to `usize::MAX` if its
checked calculation cannot be represented; the fallible API reports the error.
Preparation requires exclusive ownership of a detached/stopped processor.
Hosts prepare a candidate off thread while the distinct installed plan remains
active; this API does not permit racing preparation against a callback.

EchoBank has 220 descriptors: four global controls followed by 27 per unit.
Unit offset is `4 + 27*i`: enabled, inputGain, inputPan, sync, timeMs, division,
stereoOffsetMs, feedback, feedbackMode, feedbackPan, separation, six input-filter
fields (mode/frequencyHz/q/gainDb/gain/sections), the same six feedback-filter
fields, filterPost, outputGain, outputPan, nextSend. The last nextSend is a
zero-span reserved field, forcibly sanitized to zero, with no ninth destination.
All outputGain and nextSend descriptors use `None` for their signed coefficients;
their indices, ranges, defaults, smoothing and JSON are unchanged. Input and
filter gain descriptors remain `Gain`. Read-only source tracing of
`apps/desktop/src/features/params/format.ts` and
`apps/desktop/src/components/audio/units.ts` establishes why this matters:
`gain` formats through `20*log10(value)` (nonpositive becomes `−∞ dB`) and parses
through `10^(dB/20)` (`"-1"` becomes positive 0.891251, displayed infinity becomes
zero). `none` instead uses the signed plain number and parseNumber paths:
−1 displays as `−1.00` and parses as −1. This is source behavior, not an executed
editor acceptance claim. Native regressions check all signed metadata fields,
stable indices/JSON and independently timed negative output/send impulses.
FrequencyDelay has 71 descriptors: dry, wet, feedback, scale, shortRange,
bandwidth, split, then delayMs/level/pan/enabled at `7 + 4*i` for bands 0–15.
Missing objects/fields use serde defaults. Parameter setters also sanitize
directly constructed malformed values; empty calls and duplicate targets keep
the same clocks. `params()` returns the sanitized target, not the current ramp.

Next requested window: append two kinds/variants and dispatch arms in `effect.rs`
(no existing reorder), parent-generated TS/descriptors/WASM, project old-v1
default/command/automation/undo tests, engine same realtime/offline/slot/PDC and
bounded-plan preparation tests, then purpose-built eight-unit links editor and
sixteen-band drawn delay/level/pan editor. A large generic parameter list does
not close either row. All other E3 delays/modulation/reverbs remain in scope.

The current host sums tails with ordinary `usize::sum` in
`windfall-engine/src/state.rs` around line 1235. Before admitting either kind,
the serialized engine window must use saturating aggregation for unlimited
tails and test multiple sustaining effects plus finite render horizons. This
file is closed to E3; no host change is included here. Consider control-side
boxed variants for the two new `AnyEffect` arms so the 35 KiB EchoBank state
does not enlarge every existing effect slot. All owner retirement remains
off callback under the utility repair contract.

The proposed host contract reserves retained and peak requirements against the
project/rack budget, then invokes the concrete `try_prepare` seam off thread.
Only an `Ok(())` instance may be installed in audio. A typed refusal releases
the reservation and candidate off thread, reports required/budget or failing
history bytes to the user, and retains the current plan. Charge other live/
retiring plan owners separately: an empty candidate's peak includes its own
staging, not a different rack's old processor. Budget admission cannot replace
checking the actual allocation result. Hosts must neither shorten times nor
clamp a requested device rate to save memory. The new API is per-instance;
project-wide admission, installation and retirement remain integration work.
At 384 kHz ten banks still exceed 6 GB; replacing a live bank at that rate
temporarily needs approximately 1.229 GB of its own payload.

## Original checkpoint evidence (immutable 6326f883)

Windows/MSVC, 2026-10-08, isolated task target, one Cargo job and one Rust test
thread. No workspace, desktop, Tauri or UI build was run. All TS export tests
wrote to the task's `target/e3-bindings`, never app artifacts.

Payload bytes at rates 1/8000/48000/384000 Hz:

- EchoBank instance: 37168 / 12835568 / 76835568 / 614435568 bytes.
- One stereo unit's history: 208 / 1600008 / 9600008 / 76800008 bytes.
- FrequencyDelay instance: 4448 / 1028320 / 6148320 / 49156320 bytes.
- One stereo band's history: 16 / 64008 / 384008 / 3072008 bytes.

The instance figures include 35504 inline bytes for EchoBank and 4192 for
FrequencyDelay. Reprepare tests verify the pre-admission estimate equals all
retained payload and that a smaller rate releases the larger arrays.

The ignored release observation was explicitly executed. Each measurement
processed 16384 stereo frames in 64-frame blocks after priming 1/2-ms taps.
All eight bank units use three-section resonant input and feedback filters,
unity feedback and next sends; all sixteen frequency bands use the steep
splitter and unity feedback. Automation changes every unit/band time between
valid taps, bank cutoff/Q, bandwidth and tempo every 64 frames. Constant vs
automated elapsed microseconds on this run:

- 8000 Hz: EchoBank 37142 / 38972; FrequencyDelay 2505 / 3184.
- 48000 Hz: EchoBank 37700 / 41623; FrequencyDelay 2489 / 3375.
- 384000 Hz: EchoBank 39579 / 39585; FrequencyDelay 4091 / 3662.

This is a single shared-machine timing observation, not a device guarantee or
a deadline test. Cache/OS/competing jobs affect it. Bank reads are bounded at
16 settled / 32 fading history reads per frame, with 576 continuously running
SVFs. Frequency reads are bounded at 32 settled / 64 fading reads with 60
one-pole updates. Reset and setters traverse fixed inline control/filter state,
never history length. No callback sample coefficient tables or tap vectors exist.

Commands (Git Bash, sourced MSVC environment):

```bash
source scripts/msvc-env.sh
export CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=target/e3-native
export TS_RS_EXPORT_DIR="$(pwd)/target/e3-bindings"
cargo test -p windfall-dsp --lib
cargo clippy -p windfall-dsp --lib --test delay_family -- -D warnings
cargo test -p windfall-dsp --release --test delay_family -- --include-ignored --nocapture
rustfmt --edition 2024 --check crates/windfall-dsp/src/echo_bank.rs crates/windfall-dsp/src/frequency_delay.rs crates/windfall-dsp/tests/delay_family.rs
git diff --check
```

128 library tests and 25 release delay-family tests pass (including the one
normally ignored CPU observation); strict target Clippy and owned formatting
pass. Full-duration bank 24-second plus 1-second-offset impulses and frequency
1000-ms impulses execute at all four rates, not just scaled-down times. Tests
also cover all filter modes/orders and domain endpoints, independent analytic
bank lowpass/feedback equations, sixteen individual impulse and DFT transfers,
phase, level/pan, parallel and signed serial routing, unity during bandwidth
and splitter edits, feedback including zero-time causality, extreme nonfinite
signals, block partition, repeated targets/empty calls/reset, stale-history
invalidation, live band deselection, pending-tap readiness, old taps and
conservative tail/feedback crossings. All guarded process/setter/reset/tempo
paths make zero alloc/realloc/free calls. Finite-tail sample exhaustion is
directly tested at 1 Hz; the high-rate finite filter horizons use conservative
pole-domain bounds, not multi-minute exhaustion measurements at each rate.

Actual slot bypass/dormant restoration, native/live versus offline host paths,
project history/serde migration, generated inventory, drawn editors, device
deadlines and listening remain outside this first source window. No acoustic
equivalence or full parity completion is claimed.

## R1 repair evidence (incremental child of 6326f883)

2026-10-08, same Windows/MSVC task target and single job/test thread. Before
the production edits, the new signed-descriptor regression failed with
`Gain != None` at units.0.outputGain. The new fallible-API fixture then failed
to compile against 6326 (`E0422`/`E0599`, missing types/methods). This safe RED
step did not invoke the old infallible vector abort path or attempt OS OOM.

Four private native tests exercise actual staging and reservation refusal.
The test-only allocator returns null once for the selected valid, fallible
history reservation; std's real `try_reserve_exact` reports the allocation
error. The hook is scoped to that reservation, thread-local and absent from
production. Every history position (sixteen bank, thirty-two frequency) is
refused in turn. Tests account for each earlier staged allocation and its
retirement, verify retained live bytes/clock/params/readiness/tail unchanged,
and compare subsequent stereo samples bit-for-bit with an independently
prepared, untouched processor while filter, tempo, gain and tap edits run.
Legacy prepare is tested after partial staging, as are first refusal,
unprepared passthrough, latch survival through reset and successful recovery.
Private checked-size fixtures cover sample-count/byte-count/aggregate/header/
retained/peak overflow without large reservations. Public tests reject budgets
one byte below either limit, compare audio after refusal, then admit the exact
limits at full 1/8000/48000/384000 Hz and reprepare down to 1 Hz. Callback
allocator guards remain active after refusal and successful preparation.

The repair adds 32 inline bytes to each processor for its copyable refusal
latch; all history counts, lengths and per-unit/per-band figures above remain
unchanged. Current retained payload at 1/8000/48000/384000 Hz is:

- EchoBank: 37200 / 12835600 / 76835600 / 614435600 bytes (inline 35536).
- FrequencyDelay: 4480 / 1028352 / 6148352 / 49156352 bytes (inline 4224).

At 384000 Hz this is still 614.4 MB / 49.2 MB, without shortening any delay.
Initial preparation peak is 614436240 / 49157632 bytes. Replacing a live
instance at the same rate peaks at 1228836304 / 98309760 bytes. At all four
rates, same-rate replacement peaks reported by checked payload accounting are:

- EchoBank: 39504 / 25636304 / 153636304 / 1228836304 bytes.
- FrequencyDelay: 6016 / 2053760 / 12293760 / 98309760 bytes.

132 library tests and 26 release integration tests pass; the original CPU
observation is the sole ignored test in this repair run. All twenty-four
original signal/allocator cases run again, including full 25-second bank and
1000-ms frequency impulses at all four rates. Strict DSP Clippy, owned
formatting and diff checks pass. The earlier CPU observation and its counts
remain attributed to immutable 6326, and were not rerun solely to update the
count. DSP callback equations and frame clocks were not changed by this
repair; the P3 positional-control naming judgement remains a suggestion.

Repair validation uses the commands above except that the release invocation
omits `--include-ignored`. `cargo clippy -p windfall-dsp --lib --tests -- -D warnings`
also checks the private test-only allocator/rollback code. The new preparation
cases are part of the library run. Foundation R2 review, fallible host admission,
saturating engine tail aggregation, registry/project integration, drawn editors and all other
remaining E3 work remain pending; this repair does not close parity.

## R2 foundation status and read-only host admission proposal

2026-10-08. This inventory is against bound branch `gpt/t3-delay-family-e3`,
immutable source `42d58df48e499a9fc4c3997f0382310976bc4415`, whose parent is
`6326f883a434af507a1436532d497889fec400e2`. Independent R2 returned no hard
Standards findings and no Spec findings; the positional-controls P3 judgement
is unchanged. Both R1 P2 findings are closed. The reviewers independently ran
two and five attributed release cases; reservation rollback tests were inspected,
not independently executed. The 132-library/26-release results above remain
owner-attributed. No completed tests or CPU observations were rerun for this
proposal. The earlier statement that R2 was pending records the repair's earlier
checkpoint, not the current review status.

Only this document is changed for this assignment. Every API and policy below
is **proposed**, not installed or tested host behavior. This branch has no E3
registry variants. Parent N4 owns pool/render/stems/plugins work; T1's Controller
window is meter-only. Controller ownership, generation/retirement contracts and
desktop transactions need parent-serialized windows with their owners. The
inventory does not incorporate newer parent commits or imply permission to edit
any of those files. Host/editor acceptance, both parity rows and remaining E3
requirements remain open.

### Actual construction and limits at source 42d58df4

| Existing boundary | Observed behavior and consequence |
| --- | --- |
| `windfall-dsp/src/effect.rs`: `AnyEffect::new` (466), `prepare` (509) | Fifteen boxed original/E1 variants; no bank or frequency variant. Construction and preparation are infallible. A new variant must dispatch to the concrete `try_prepare`, not merely call the trait's void method. |
| Same file: `EffectSlot::new` (686), `prepare` (709) | Construction already allocates default dry histories/tap storage. Preparation sets the slot clock, calls void effect preparation, allocates dry histories, tap storage, two scratch buffers and aligned flags, then resets the processor. Calling this after a successful E3 preparation would prepare the histories twice. Calling it after a refusal could reset the previous valid effect and wrap an unprepared fresh effect as if healthy. |
| `windfall-engine/src/rack.rs`: `EffectUnit::build` (103) | Creates `AnyEffect`, slot and prepared buffers at `MAX_BLOCK = 256`, then applies tempo/params/enabled/mix. It returns no error. Native plugin installation happens later, so even a plugin-bound slot currently has a prepared builtin fallback; admission must count that actual allocation until N4 changes the construction contract. There is no separate `Rack` struct: runtime chains belong to `PlanState`. |
| `plan.rs`: `compile` (509), `compile_render` (573), `keep_leaving` (334) | Plans contain settings and owner-generation metadata, not prepared E3 histories. Compilation clips to 128 mixer tracks and 10 active effects per track. Departure definitions can coexist with active definitions. These are logical limits, not byte limits. Render compilation clones the pool and can select a separate plugin provider; provider identity/revision still matters. |
| `state.rs`: `PlanState::build` (629), `take_over` (892) | Build calls plugin preparation, creates compensation/buffers and constructs fresh units. Known units and departures use empty seats for subsequent ownership transfer. `Ledger` is a projection of expected owners, not ownership of their histories. Transfer moves the generation-matched physical unit; a same-ID restore can retain a distinct heard departure. |
| `controller.rs`: `try_prepare_project` (198), `compile_prepared_project` (207) | `PreparedProject` contains a compiled `Plan` and sampler pool only. Its fallible result concerns sampler preparation; it does not make effect DSP ready. |
| Same file: `set_plan` (232), `attach` (505) | With a stream, `set_plan` builds runtime DSP under the Controller mutex before queuing it. Without a stream it stores metadata. `attach` builds all units fresh at the requested rate, then publishes link/ledger and clears backlog. Neither returns DSP preparation failure. |
| `message.rs` and Controller `send`/`maintain` (615/638) | Message capacity is 1024; garbage capacity is 4096 with four-slot headroom. Controller backlog is an unbounded `VecDeque`. These count messages, not histories or bytes. Queued `SetPlan` and retired `State` still own buffers. The defensive garbage-overflow path deliberately leaks rather than freeing on RT. |
| `pool.rs` and `sampler_processing.rs` | Pool clones share source maps/caches. `share_sampler_budget` preserves the sampler budget across replacement. Sampler banks have a shared 256 MiB reservation/lease budget including unpublished preparation, with one off-thread FFT workspace at a time. This is a useful ownership precedent, not an effect or process-wide budget. |
| `clip_processing.rs` | The clip cache retains at most 32 variants/256 MiB except a single larger clip. This counter concerns rendered cache audio; source identity/audio is also held. Cloned cache counters are not a global ownership lease. Decoded input audio has no aggregate pool byte bound here. Native plugin allocation is opaque to these counters. |
| `render.rs`: `render_reporting` (100) | Prepares samplers, creates a new Controller and attaches a fresh runtime. Headless rate is only normalized with `max(1)`. Full output uses `Vec::with_capacity`/growth; block size and total output allocation have no memory admission. Its report has sampler error only. |
| `stems.rs`: checked streaming (302), stems (325), `Pass::new` (559) | Checked streaming currently checks samplers only. `TrackOutputs` uses one pass; `ToMaster` uses a mix pass if needed and then sequential fresh stem passes. `leave_only` changes inputs but leaves bus effects present: profile the compiled pass, not just the selected track. Each pass constructs fresh DSP independently of live playback. `Outlet::held` can accumulate quiet audio awaiting an automatic-tail decision. Streaming alone therefore does not bound memory. |
| Desktop `session/export.rs` | One active export per Session, rates 8k..384k, a day/frame and WAV-size horizon. These are job/format limits, not delay-memory admission. Other Sessions/API renders can coexist. Pool snapshots preserve sampler charges; they must also preserve the future delay budget. |

Paths in the table are relative to `crates/` unless prefixed with desktop.
`EffectLife` atomics describe heard/finished progress, not ownership or a
reusable authorization to allocate. A unit marked gone, disabled or dormant
still owns its complete prepared histories until its physical owner is dropped.

The current zero-PDC slot also has a measurable cost outside E3 payload.
At block size 256, two four-sample dry rings use 32 bytes, one dry tap uses
16 bytes on this 64-bit build, scratch uses 2048 bytes, and flags use 256 bytes:
2352 retained heap bytes, plus slot/unit/plan/container payload. Default ring/tap
allocations can overlap their replacements during construction. Future admission
must measure capacities and include these costs; the processor's reported
614435600/49156352 bytes already includes its own inline state, so do not count
that state twice when profiling its box.

### Proposed shared byte scope and preparation reservation

Use one owner-scope admission ledger shared by live Controller, replacement
documents/pools, pending preparation jobs, queued plans, retiring plans and
their render snapshots. N4 should provide the shared resource anchor if its
current work already introduces one. New pools, fresh caches, new render
Controllers, provider snapshots and subsequent stem passes must inherit the
anchor; `SamplePool::default()` must not quietly reset admission for a related
replacement or render. A deliberate unrelated engine/session may create a new
scope; a separate global cap is needed if the parent intends a multi-session
limit rather than a per-session limit.

Concrete proposed initial policy: a configurable **2 GiB managed prepared-delay
payload limit** per scope, with stricter lower limits for tests/callers and no
single-large-instance exception. Count E3 state, histories and the associated
host slot/unit/plan storage covered by the profile, plus unpublished staging.
Reserve all arithmetic with checked sizes and atomic compare/reserve or an
off-thread ledger lock. Do not wait for space while holding document/Controller
locks; return an admission error with requested, currently charged and limit
bytes. This policy is proposed for parent acceptance, not an existing limit.
It limits concurrent owners, not supported DSP rates, time ranges or band count.

At 384k, three bank payloads alone total 1843306800 bytes; four total
2457742400 and exceed 2147483648 before slot costs. One active bank plus a
fresh replacement needs approximately 1.229 GB before wrapper costs. The old
bank cannot be credited as free while it is audible, queued for transfer, or
awaiting garbage collection. Ten active banks alone require 6144356000 bytes;
the project slot-count limit does not admit them. No automatic time reduction,
rate substitution, partial sixteen-band allocation, or bypass-on-refusal is allowed.

This is a managed payload policy, not a process RSS guarantee. The existing
sampler sub-limit, decoded samples, clip cache, output buffers, allocation
bookkeeping and third-party plugin allocations must be reported separately or
covered by N4's broader broker. Their limits must not be advertised as already
solving cumulative E3 ownership. Admitting bytes also does not establish CPU
deadline or device capability; the original observations do not establish those.

Prepare a complete candidate serially off RT, with a flight reservation acquired
**before any large histories or host buffers are allocated**. For fresh owners,
let `R_i`/`P_i` be the checked retained/peak requirements from an empty processor,
`H` the candidate host/container retained charge, and `W_i` its extra transient
wrapper/boxing charge. A serial candidate requires a checked reservation of
`H + sum(R_i) + max(P_i - R_i + W_i)`; shared old owners are already charged by
the scope and must neither be subtracted early nor double-counted in this new
reservation. Concurrent candidates each reserve their whole additional peak.
Host profile estimates must include actual buffer capacities, ring rounding,
box layout/alignment and plan queue storage, not just history length.

Pass finite per-instance retained/peak caps backed by that reservation into
the concrete `try_prepare`. After success verify `prepared_bytes()` and wrapper
capacities against their profiles before publication. R1 also checks actual
history capacities before committing them. Allocator bookkeeping is outside
these payload numbers; an allocator/profile with unbounded capacity excess cannot
support a strict physical-peak claim. A capacity discrepancy must refuse the
candidate and retire its buffers off thread, rather than silently retain extra
arrays or treat a post-hoc byte estimate as admission. Establish the reservation
extent used by the selected allocation helpers as part of host acceptance.

On any reserve/preparation/stale/cancelled error, drop all earlier fresh units,
wrapper buffers and staged plugins on the proper control/worker owner thread,
then release their reservation. Keep the old runtime, plan, projected ledger,
project, history cursor, pool and transport untouched. A successful flight splits
its reserved charge into physical-owner leases without a release/re-reserve gap;
release staging headroom only after staging is actually freed. Field/drop ordering
must destroy buffers before releasing their charge.

An exclusively detached in-place reprepare may reserve the checked extra peak
`requirements.peak_bytes - existing.prepared_bytes()` while its existing lease
remains held. Resize its lease only after success and retirement of old buffers.
Never reprepare an attached callback owner in place. Ordinary parameter, tempo,
enabled and mix edits reuse the full-range prepared owner and need no new history
allocation. Distinct active/departing generations each own a lease; transfer in
`take_over` moves the lease with the unit. Progress atomics, a meter clone or a
Ledger entry must not release or duplicate that physical charge.

Carry leases unchanged in queued `SetPlan`, active/dormant/departing units and
`Garbage::State`. Release them only on physical control-side destruction, including
candidate cancellation and final offline-pass cleanup. A deliberately forgotten
garbage item must remain charged, making any leak observable through refusal.
No lease allocation, destruction, broker access or reservation occurs in audio
processing/reset/setters/tempo/ownership transfer. Verify device feeder destruction
also returns owners off RT; backend callback destruction is not assumed safe.

Byte admission is necessary but not a bound on zero-history metadata churn.
Propose a separate 16-ticket limit for prepared-but-not-yet-adopted `SetPlan`
transactions, including backlog, with an off-thread adoption acknowledgement.
Reject before musical commit when no ticket is available. Do not overwrite queued
plans or collapse progress/generation semantics to reclaim space. Existing transport
messages retain their ordering; parent utility ownership must approve this policy.

### Proposed typed API and publication contract

Keep `Effect::prepare` unchanged. Add narrow fallible construction seams in the
serialized DSP/host window. Suggested signatures/types, not current exports;
the first two belong to DSP and must not depend on engine types or leases:

```rust
AnyEffect::try_new_prepared(params, actual_rate, max_block, budget: PreparationBudget)
    -> Result<PreparedEffect, EffectPreparationError>;
EffectSlot::try_from_prepared(effect: PreparedEffect, max_block, budget: PreparationBudget)
    -> Result<EffectSlot, EffectPreparationError>;
EffectUnit::try_build(plan_effect, track, context, permit)
    -> Result<(EffectUnit, Option<GainReductionMeter>), PlanPreparationError>;
PlanState::try_build(plan, context, reservation)
    -> Result<(PlanState, Ledger), PlanPreparationError>;
Controller::try_prepare_runtime_project(project, pool, context)
    -> Result<PreparedRuntimeProject, PlanPreparationError>;
Controller::try_set_prepared_project(project, prepared: PreparedRuntimeProject)
    -> Result<(), PlanPreparationError>;
Controller::try_attach(actual_rate)
    -> Result<Processor, PlanPreparationError>;
```

`PreparedEffect` has a private verified-state constructor, using only DSP-local
budget/status/error types. The engine keeps its flight reservation during those
calls, then attaches a lease to the physical `EffectUnit`; the DSP crate never
imports the host broker or project IDs. `PreparedRuntimeProject` has private
fields and owns its host reservation/prepared owners. `PreparedEffect` contains
an E3 processor only after `try_prepare` returned `Ok`, status is prepared with
no refusal, and its actual clock equals the requested engine clock. A previously
valid processor with a latched failed replacement is not a fresh candidate for
a new clock. E3 supports
1..384000 Hz; host input outside that range must return an unsupported-rate error
instead of installing a sanitized 384k processor in a differently clocked engine.

The slot constructor consumes prepared DSP and initializes its own buffers;
it **never calls void `prepare` again**. Provide fallible slot rings/tap/scratch
and boxed-state construction, with checked limits. Existing `Box::default`,
`DelayLine::new`, `TapCrossfade::new` and `vec!` are not fallible just because the
inner E3 histories are. A small reviewed allocation helper/ownership representation
is required for boxed inline state on the supported Rust toolchain; do not assume
unstable `Box::try_new` is available. Exact optional block constructor seams need
parent/utility permission; no whole Effect trait refactor is proposed. Preserve
the existing fifteen kinds and their dispatch/control behavior. Recovery/budget
claims for other legacy/native constructors need their own supported profiles;
an opaque plugin allocator cannot be made recoverable by wrapping it in Rust Result.

Propose a DSP-local `EffectPreparationError` carrying the concrete E3
`PreparationError` or a slot/box checked-capacity/reservation refusal, with
component and requested bytes. Map it at the engine boundary, preserving the
source detail; DSP remains independent of engine types. Propose engine
`PlanPreparationError` variants for `Admission { requested, charged,
limit }`, checked capacity overflow (component), unsupported rate, owner-scoped
`DelayPreparation { track, effect, generation, actual_rate, source:
PreparationError }`, wrapper allocation refusal, sampler preparation, stale
context, cancellation and pending-ticket refusal. Formatting happens off RT.
Report the original history index/requested bytes on real DSP reservation refusal,
not just an estimate or generic sampler error. Queue/adoption rejection uses a
fixed scalar acknowledgement and retires the complete candidate through garbage.
The general error/status may need a later IPC window; no generated exports here.

Do not rename a compiled metadata-only `PreparedProject` into healthy DSP.
Either keep it explicitly as compiled input to the new runtime builder or give
it a distinct compiled type. Only the sealed runtime type may publish an E3
owner. A refused fresh instance's normalized passthrough is diagnostic behavior,
not an admissible replacement. There is no healthy empty-seat fallback on failure.

Use a two-phase Controller/document transaction:

1. Under existing short locks capture actual stream/device epoch and rate,
   Controller publication serial and projected Ledger, previous plan identity,
   project generation/edits/replacement/loading stamps and exact pool source
   identities. Capture native provider identity/revision and binding ownership
   separately from mutable parameter values. Copy metadata only; do not clone
   DSP histories or hold controller/document locks during construction.
2. Apply the command/history move to a private document; run `keep_leaving` on
   its private plan using the existing owner-generation rules. Profile, reserve
   and build all new owners on workers, with cancellation. Empty seats are sealed
   reuse promises `(id, kind, generation, rate, native owner)` or explicit
   departure promises, never substitutes for failed construction.
3. Prebuild the replacement document/history, pool handles, native parameter
   stages and event/queue payloads on the control side as part of the ticket.
   Obtain a short publication permit using the existing owner-approved lock
   order. Revalidate all stamps, pool identities, native factory revision, actual
   rate/epoch, projected owner promises and pending-ticket availability **before**
   dispatching into the live document, moving its undo cursor, changing a gesture,
   replacing pools, committing native parameters, changing transport, installing
   Controller metadata, or emitting a project patch. Refusal/staleness destroys
   only the candidate. Retry requires a new context/reservation, not an old permit.
4. Move the already built candidate document/pool and plan/state into their
   owners as one validated transaction, with reserved queue/backlog storage and
   no fallible construction between final checks and queue insertion. A second
   live-document dispatch must not allocate or fail after installing half the
   transaction; use the prepared mutation/document under the session owner's
   history/gesture contract. Queued plans continue projecting ownership in order.
   Before
   `Processor::adopt` mutates voices or transfers histories, bounded preflight
   validates every reuse/departure promise against the currently installed owner
   generations. On an invariant mismatch, retain the old runtime and retire the
   entire candidate; never create missing effects on RT. Such mismatch is a host
   invariant failure requiring acknowledgement/reconciliation, not successful edit
   admission. Normal publication must guarantee these promises cannot go stale.

For a stopped/detached Controller, propose retaining a sealed prepared candidate
at the **explicitly configured** rate under the same scope, for later attachment
at that exact rate. Do not invent an implicit clock. If no rate is configured,
return a needs-rate refusal for an audio-ready transaction; compile-only loading
must be explicitly unprepared and may not claim healthy audio. A different actual
device rate requires new admission while the detached candidate remains charged.
This needs the utility owner's detached-identity contract/window. `try_attach`
must finish preparation before publishing a new link/ledger, clearing backlog,
panicking hardware or changing shared transport. `device.rs::build` already has
a Result boundary and should propagate refusal there. Preserving an old viable
stream requires N4's device replacement protocol; if the device has already been
lost, preserve the documented detached/stopped state instead of claiming that an
old device remains usable.

### Exact serialized file hooks and transaction coverage

| Future file window | Required hook and owner coordination |
| --- | --- |
| `crates/windfall-dsp/src/effect.rs` | Registry additions, fallible prepared dispatch, sealed slot construction and buffer profile. The registry remains closed now. Existing two lib declarations need no further exports for this proposal. |
| `crates/windfall-dsp/src/blocks/delay_line.rs`, `blocks/tap_crossfade.rs` | Optional exact fallible checked constructors/empty staging, approved by their owner, preserving callback tap behavior. Alternatively propose a scoped E3 slot adapter for review; do not duplicate or weaken bypass/warm-up policies. |
| `crates/windfall-engine/src/pool.rs`, shared broker module chosen by N4 | Preserve the common scope through pool clones, replacement, `cached_clip_pool`, render provider snapshots and worker jobs. Keep sampler-bank identity/leases intact; no replacement-cache budget reset. |
| Engine `rack.rs`, `state.rs` | Fallible fresh unit construction, complete candidate rollback, measured host/compensation/container capacities, unit-owned leases. Maintain the utility owner's exact generation/departure/reuse transfer; require rate in reuse promises. |
| Engine `plan.rs`, `controller.rs` | Private compilation/context and `keep_leaving`, two-phase preparation/publication, typed error latch and configured detached-rate candidate; no long preparation under mutex. Coordinate with utility ownership and T1's meter-only edits. |
| Engine `processor.rs`, `message.rs` | Pre-adoption validation and ordered acknowledgement; move leases with units; retired state returned for off-RT destruction; unexpected leaks stay charged. Preserve queue headroom and no callback frees. Utility/retirement owner approval is required. |
| Engine `device.rs` | Fallible actual-rate attachment before link/stream publication; rollback/retry and feeder destruction coordinated with device lifecycle owner. |
| Engine `render.rs`, `stems.rs`, `plugins.rs` | N4 window: shared scope, typed checked render constructors/results, sequential pass cleanup, actual compiled-pass profiling, provider/revision validation and native retirement thread. |
| Desktop `session/edit.rs` (`dispatch`, `publish_with_prepared`, `push_project`, `prepare_history`) | Current sampler-only branch does not cover ordinary effect insertion/replacement. Route every owner-affecting candidate through full runtime admission before live dispatch/history mutation; keep cheap same-owner param/tempo edits allocation-free. No patch/native-parameter commit before the publication permit. |
| Desktop `session/sampler_processing.rs` (`SampleEditTicket::prepare`, prepared commit, refresh jobs) | Extend existing private candidate/stamp workflow to complete host runtime admission, not just sampler banks plus compiled metadata. Background refresh must validate controller/device/native epochs too. |
| Desktop `session/files.rs::install` | Prepare decoded candidate pool and staged plugins with the shared scope before replacing Document/Pool, generation/history, plugin document or transport. Source-42 calls effect preparation only later through Controller; it does not already guarantee this rollback. |
| Desktop `session/clip_processing.rs`, `audio_editor.rs`, `slicer.rs` | All existing prepared-candidate publication paths must use the same sealed runtime transaction, including recovered sources and source-identity checks. No alternate compiled-only installation path. |
| Desktop `session/export.rs::export_audio`/`write_files` | Keep immutable project/pool/plugin snapshot validation; acquire shared admission before large pass allocations. Use checked rendering for non-stem export too. Map preparation failure to error, not `Outcome::Cancelled`; retain file staging/rollback and old destination files. Root/N4 serialized window only. |

There are no source edits, imports, tests, generated types, Cargo changes or
editor changes in these windows in this checkpoint. Parent must reconcile this
source-42 inventory with N4's newer work before assigning exact hunks.

### Saturating tails and bounded offline horizons

`state.rs::PlanState::settled` (1222) uses ordinary `sum` for tail/gap at
1235/1236, then ordinary additions for direct/onward holds, ring deadline and
quiet deadline. Two sustaining banks can overflow the tail sum. Change tail and
gap folds to saturating addition and all associated `u64` deadline/hold additions
to saturating addition. Explicitly map the `usize::MAX` infinite-tail sentinel to
`u64::MAX` before conversion, including on 32-bit targets; widening a 32-bit
sentinel as a finite deadline is incorrect. A finite gap can still allow a track
to settle after genuinely quiet output; saturation must not force every feedback
effect to render forever or declare it finished before a late echo.

E3 algorithmic PDC remains zero. Its musical-delay/warm-up/gap horizons must not
be assigned to the compensation rings. Review checked compensation arithmetic in
`state.rs::Layout::of` (including latency/path sums) against the existing finite
one-second compensation bound; the bound is not a substitute for checked sizes.

Render `most`, `body_end`, `loud_end + hold` and interleaved byte/frame products
in `render.rs` (157 onward) and `stems.rs::Pass` (611 onward) also need explicit
checked construction and saturating deadline comparisons. Reject overflowing,
unsupported or unadmitted horizons/blocks with typed errors. Do not saturate a
frame count to MAX and then attempt that allocation. The finite requested export
tail is a truthful truncation horizon for still-sustaining audio, not evidence
that its feedback tail has ended. Keep the actual supported rate/time ranges.

N4 should bound/fallibly prepare render blocks and tap buffers, and charge
in-memory output/automatic-tail retention separately. Propose a finite capped
chunk size for streaming I/O (splitting caller blocks leaves frame DSP unchanged),
with no silent shortening of output. For `Outlet::held`, use a reviewed bounded
staging/spool strategy for quiet audio that may precede a later echo, or return
an explicit storage refusal before exceeding its reservation. A ring that merely
discards old quiet frames would lose required timeline silence. Use the existing
file transaction for export-stage cleanup; in-memory `render` needs a checked
Result core rather than an apparently successful empty audio fallback.

One ToMaster pass at a time may release its charge only after the entire pass,
Controller queues/garbage and native owners have been destroyed on their proper
worker threads. Live playback remains charged throughout export. Multiple jobs
or snapshots cannot each receive an independent unlimited delay budget. Cancelled
render, allocation refusal and stale plugin snapshot must remain distinct results.

### Required host acceptance evidence in the later windows

No host tests were executed here. Before registry exposure, parent acceptance
should require:

- Shared-budget refusal before construction of a fourth 384k bank profile,
  without allocating four banks or attempting OS OOM. Use strict small budgets
  and deterministic allocator seams to exercise real prepared candidates; do
  not shorten production ranges or substitute an estimate-only test for rollback.
- Failure at every E3 history and new wrapper/boxing reservation position after
  earlier units have staged; charged bytes return only after cleanup. Old audio,
  clock, params, histories, project, history cursor/gesture, pool, projected Ledger,
  pending order and transport remain unchanged. Stale/cancelled candidates follow
  the same ownership path.
- Repeated same-ID remove/restore, kind/rate/provider changes, disabled/dormant
  units, paused audio, full queues/backlog, delayed garbage collection and export
  alongside playback. Physical outgoing owners stay charged; successful history
  transfer does not duplicate or release their lease. Verify acknowledgement and
  reuse promises before any RT mutation, and no callback allocation/free/lock/wait.
- Replacement pools, clip-cache snapshots, sampler sharing and render provider
  snapshots retain the same scope; ToMaster sequential cleanup cannot outrun
  native/queue retirement. Unsupported engine clocks fail explicitly. Native
  allocation/retirement limitations and decoded/output bytes are reported honestly.
- Multiple `usize::MAX` feedback tails and finite gap horizons at nonzero and
  near-overflow frame clocks, explicit 32-bit sentinel conversion, late echoes
  across silent gaps, bypass/dormant restoration and finite export horizons.
  Checked in-memory/streaming buffer refusal is an error with old files preserved,
  not cancellation or apparently healthy silence.

These gates add host evidence to the accepted DSP foundation. They do not replace
the existing full-rate signal proofs, future drawn bank links/sixteen-band editor,
device/listening validation or the rest of E3, and do not close parity by themselves.
