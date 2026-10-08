# Filter family: first DSP delivery

The owned `windfall_dsp::filter_family` module implements three stereo
processors through the existing `Effect` and `ParamSet` contracts. They are
independent processors, with authored reference tests, rather than named EQ
presets. The existing seven-band EQ is unchanged.

This stage does **not** complete `fx-fruity-fast-lp`, `fx-fruity-free-filter`,
or `fx-fruity-bass-boost`. The DSP registry now constructs all three through
`AnyEffect` and `EffectSlot`. The approved downstream test window now proves
project commands/persistence, generated controls and live/offline/stem behavior
in the isolated worktree. Interrupted removal/restore still fails on this
branch's older engine policy and needs the parent's utility composition.
Generated artifacts remain parent-owned. No parity status is changed here.

## Behavioral sources and equivalence scope

The [Fast LP manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Fast%20LP.htm)
describes a low-CPU automated lowpass with cutoff and resonance controls.
Windfall's **Fast lowpass** implements those controls with a defined
second-order transfer; its maximum cutoff remains a filter, not a bypass.

The [Free Filter manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Free%20Filter.htm)
lists lowpass, highpass, bandpass, notch, low shelf, peaking and high shelf,
with frequency, Q and gain. Windfall's **Selectable filter** implements all
seven. Gain changes only shelf and peaking responses.

The [Bass Boost manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Bass%20Boost.htm)
describes frequency and strength controls and the need for gain headroom.
Windfall's **Bass shelf** provides a monotonic shelf with a dB boost control.

These references establish behavior categories. They do not publish transfer
equations, exact parameter ranges, smoothing or default values. The contracts
below are Windfall's own design, not a claim of identical proprietary DSP.
No proprietary code, assets, presets or measured proprietary fixtures are used.

## Stable controls and limits

Control indices are the following fixed order; parameter JSON uses camelCase.

| Processor | Index / field | Unit / range | Default |
| --- | --- | --- | --- |
| Fast lowpass | 0 `cutoffHz` | Hz, 20–20000, logarithmic | 20000 |
| | 1 `q` | ratio, 0.5–10, logarithmic | 1/√2 |
| Selectable filter | 0 `mode` | choice, 0–6 | `lowpass` |
| | 1 `frequencyHz` | Hz, 20–20000, logarithmic | 1000 |
| | 2 `q` | ratio, 0.5–10, logarithmic | 1/√2 |
| | 3 `gainDb` | dB, −18–18, linear | 0 |
| Bass shelf | 0 `frequencyHz` | Hz, 40–1000, logarithmic | 150 |
| | 1 `gainDb` | dB, 0–18, linear | 0 |

Mode values in index order are `lowpass`, `highpass`, `bandpass`, `notch`,
`lowShelf`, `peak`, `highShelf`. Unknown automation indices return false/None.
Choice automation rounds/clamps finite indices and leaves the current mode
on nonfinite writes. Other nonfinite controls fall back to their defaults;
finite out-of-range controls clamp. Missing serialized fields use defaults.

`prepare` sanitizes the sample rate to 1–384000 Hz; nonfinite rates use 48000.
The tuning ratio is clamped to 0.00001–0.49 cycles/sample. Thus the effective
cutoff is held below Nyquist even at tiny rates, and never reaches a singular
coefficient. No existing biquad primitive is called with its invalid tiny-rate
frequency bounds. Shelf integrator tuning is additionally scaled as below.

No processor adds a bypass control. Future registry integration must use the
existing `EffectSlot` bypass/mix policy. The fast and selectable lowpass default
settings are not mathematically dry. Bass shelf at 0 dB, and selected shelf or
peak at 0 dB, are exact unity when the edit settles.

## Transfer equations

Let `g = tan(π clamp(f/fs, 0.00001, 0.49))`, `k = 1/Q` and
`u = (1 − z⁻¹)/(1 + z⁻¹)`. A trapezoidal state-variable filter has denominator

`D = u² + k g u + g²`.

Its low, band and high outputs are respectively `L = g²/D`, `B = g u/D`
and `H = u²/D`. The implementation uses the existing `Svf` integrator with
`a1 = 1/(1 + g(g+k))`, `a2 = g a1`, `a3 = g a2` and two integrator histories
per channel. There are no delay buffers or cross-channel terms.

Fast lowpass uses `L`. Its analog asymptotic rolloff is 12 dB/octave;
bilinear frequency warping steepens the final digital octaves. At the actual
cutoff its magnitude equals Q, giving −3.0103 dB for Q=1/√2. DC gain is one
and the Nyquist zero is double. Q=10 gives +20 dB at the cutoff, so resonance
requires upstream headroom. This path uses one SVF per channel.

Selectable filter uses `L`, `H`, `kB` and `L+H` for its first four modes.
Bandpass has a unity peak at every Q and 6 dB/octave asymptotes on each side;
Q changes bandwidth. Notch has unity DC/Nyquist and a zero at the cutoff.
Lowpass/highpass have 12 dB/octave stopbands and magnitude Q at the cutoff.

For the three gain modes let `A = 10^(gainDb/40)`. Each uses its own
continuously running SVF, and a combination `m0 x + m1 B + m2 L`:

| Mode | Integrator g | Damping k | m0 | m1 | m2 |
| --- | --- | --- | --- | --- | --- |
| Low shelf | g/√A | 1/Q | 1 | k(A−1) | A²−1 |
| Peak | g | 1/(QA) | 1 | k(A²−1) | 0 |
| High shelf | g√A | 1/Q | A² | k(1−A)A | 1−A² |

Here the table's k denotes each row's damping. These produce the cookbook
shelf/peak transfers through trapezoidal integrators without direct-form
state remapping. Shelf endpoints are A² and unity; the midpoint has half the
gain in dB. Peak center gain is A². Shelf Q above 1/√2 intentionally adds
overshoot; its peak can exceed the nominal shelf gain. There is no limiter.

Bass shelf deliberately has two controls and no resonance. With
`C = 10^(gainDb/20)` and `p = g/√C`, its transfer is

`T = (u + Cp)/(u + p) = 1 + (C−1) p/(u+p)`.

The existing `OnePoleFilter` computes the lowpass term using coefficient
`c = p/(1+p)` and a single history per channel. DC gain is C, Nyquist gain is
one, and the named frequency has gain √C (half the boost in dB). This shelf
is monotonic, has no resonant overshoot, and does not change high-frequency
gain apart from the documented transition band. Maximum 18 dB boost means
7.9433× DC gain: reserve 18 dB upstream steady-state headroom. Transients and
rapid automation still need ordinary mixer peak monitoring. Output above
±1 is preserved rather than clipped or scrubbed.

## Transitions, histories and realtime behavior

The first settings after construction/prepare/reset apply immediately. Empty
blocks do not consume that state. Later continuous edits ramp signal-space
coefficients (g, damping, output gains, or first-order coefficient) linearly
for `max(1, round(0.005 fs))` audio frames. The end is exact; frequent
retargets begin from the current values. Repeated writes of the same target
do not restart a ramp. No sine/tangent/power/log calculation occurs in the
per-frame path. Parameter changes compute those operations once per update.

Selectable filter runs its main, low-shelf, peak and high-shelf SVFs every
frame, including inactive modes. Seven nonnegative weights crossfade to the
requested mode over the same frame count. Retargeting an unfinished mode edit
starts from the existing mix; there is no dry insertion, history reset,
inactive bank catch-up or unbounded list of old modes. At constant controls
the result is exactly the weighted sum of independently running responses.
Changing frequency/gain/Q simultaneously also ramps the underlying banks.

Bass shelf keeps its lowpass live at zero boost, so raising boost has current
input history. Returning to 0 dB multiplies that state by exactly zero once
the ramp finishes. Reset clears every audio history and snaps all controls
to the latest sanitized settings.

State smaller than 1e−20 is flushed with existing primitives every 64 **audio
frames**, using a persistent clock. This never replaces NaN, infinity or
over-range output; finite-state/stability checks observe raw output. No
callback allocator, lock, file/console IO, sleep or wait is present. Storage
is fixed-size; it does not depend on block size, edit count or sample rate.

## Latency and tail policy

All three report zero latency, zero warm-up and zero delay readiness. They
have phase/group delay intrinsic to their transfers, but no host-compensated
lookahead delay. Tail and gap both conservatively reserve decay time, rather
than treating an IIR zero crossing as the end of a tail.

Tail bounds are calculated only at prepare. For an SVF, the complex-pole
radius is `r = sqrt((1−gk+g²)/(1+gk+g²))`; overdamped real-pole radii are
calculated from the corresponding quadratic roots. The global bound uses
the endpoint damping and integrator gains over all supported controls,
including inactive banks and independently ramped gain/damping extrema.
Shelf and peak domains are bounded separately to avoid mixing extrema from
different banks. For the
first-order shelf, `r = abs((1−p)/(1+p))`. The reported bound is
`ceil(64/−ln(r)) + rampFrames + 64`. The 64 natural-log-unit margin
allows for resonant amplitude, repeated poles and floating-point residue.

This policy assumes controls stop moving once a tail is allowed to finish.
Continued user automation is new activity; a host must keep processing it.
Repeatedly modulating a time-varying filter is not covered by a frozen-pole
decay proof. Invalid input audio is outside this module's finite-audio
contract; parameter/sample-rate sanitization is separate from audio clipping.

## Independent checks and remaining integration

`crates/windfall-dsp/tests/filter_family.rs` is auto-discovered. Its authored
double-precision direct-form reference derives polynomials from the transfer
equations; it never calls product coefficient constructors or SVF updates.
Checks cover impulses at five rates/frequency/Q/gain extrema, impulse DFT,
tones, logarithmic sweeps with independent stereo noise, reported-tail decay,
bass headroom, mode crossfades including rapid retargets, arbitrary
partitions and duplicate automation writes, silent/reset state, stereo
independence, descriptors/JSON, tiny/invalid rates, and guarded callback
alloc/realloc/free. The allocator verifies that its own allocation/free
counter is active. An ignored release-only throughput measurement covers
128-frame stereo blocks with constant and changing controls.

Native checks use Git Bash `source scripts/msvc-env.sh`,
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, and `CARGO_TARGET_DIR=target/e1-native`.
No Tauri, full workspace, browser or package install is needed. Bindings must
use `target/e1-bindings` once their ownership window is handed over. Headless
walltime throughput is not actual audio-device deadline or listening proof.

The DSP registry adds three distinct kinds/parameter variants and preserves
defaults/JSON/automation index order. All three route through the normal
effect slot. Remaining acceptance must cover save/load/undo edits and
automation, expose controls and all modes, and verify matching
live-engine/offline-export behavior. Engine removal/restore continuity still
requires the utility owner's accepted engine seam. No production hostbridge
or T8 spectrum work is included.

## Executed source checks and measurements

Initial immutable checkpoint: `f0ad5e44f43e92acebd166c003c8fc186c0d0a96`,
based on `015da36c2d8094323101988f2713a07104d3121d`. A separate source
follow-up adds sweep, stereo-history, zero-gain return and tail-decay tests
and separates shelf/peak tail domains. Independent review of the initial
checkpoint is pending; the follow-up needs its own fixed-source review.

Executed on Windows, 2026-10-08, Intel Core i9-14900F (24 cores, 32 logical
processors), Rust `1.99.0 (b940084d7 2026-09-28)`. All Cargo commands sourced
`scripts/msvc-env.sh` and used one build job, one test thread and the isolated
`target/e1-native` directory.

- `cargo test -p windfall-dsp --test filter_family -- --nocapture`:
  13 passed, one ignored throughput measurement (1.47 s test execution).
- `cargo test -p windfall-dsp --release --test filter_family -- --include-ignored --nocapture`:
  14 passed, including throughput (3.03 s total test execution).
- `cargo clippy -p windfall-dsp --test filter_family -- -D warnings`: passed.
- Owned Rust files were formatted with `rustfmt --edition 2024`; diff
  whitespace checks passed. No workspace build, Tauri, bindings export,
  WASM rebuild, package install, push or PR was performed.

The release measurement uses the repository's unchanged release profile,
30,000 blocks of 128 stereo frames per case (3.84 million stereo frames),
refilling preallocated buffers with deterministic input. The timed region
includes those copies, processing and optional once-per-block automation,
but excludes construction/prepare and test console output. Automation cycles
descriptor extrema; this measures bounded parameter-update cost alongside
audio processing, rather than predicting a particular musical workload.

| Processor | Constant µs/128 | Automated µs/128 | Constant M stereo frames/s | Automated M stereo frames/s | Fixed storage |
| --- | --- | --- | --- | --- | --- |
| Fast lowpass | 0.510 | 0.549 | 251.23 | 233.06 | 80 bytes |
| Selectable filter | 3.121 | 3.302 | 41.01 | 38.77 | 408 bytes |
| Bass shelf | 0.278 | 0.294 | 460.71 | 435.38 | 72 bytes |

At 48 kHz these automated cases consumed respectively 0.021%, 0.124% and
0.011% of a single core's audio-duration walltime. Other T3 work was active;
earlier runs varied (for example, selectable-filter constant processing was
2.702 µs). These are scoped throughput observations, not scheduling bounds,
device-buffer deadline tests, underrun measurements or listening acceptance.
No instrumentation was added to product callbacks.

At 48 kHz the global reported tails are 489229 frames (fast lowpass),
1378281 (selectable filter) and 34754 (bass shelf). The tests process past
the reported bound with frozen controls at 1, 48000 and 384000 Hz and check
residue below 1e−18; the tested extrema include minimum cutoff, maximum
resonance/boost and Nyquist-limited tuning. Bounds are deliberately
conservative and do not claim that the entire reported interval is audible.

The mode-transition test first failed on rapid retarget: weights already
aiming at zero retained their old arrival time, so their sum could dip.
The initial checkpoint fixes this by retargeting the whole weight vector
from its current mix only when the requested mode changes. The independent
live-bank sum now matches through interrupted fades; duplicate writes and
unrelated parameter edits retain their original timing.

## DSP registry checkpoint and exact downstream proposal

The registry source window starts from immutable
`796a62f99d60c45f19865efb45c374b3265a6a85`, without parent imports or amendments.
`EffectKind::ALL` appends `FastLowpass`, `SelectableFilter`, `BassShelf` after
the original 12 entries; old discriminants/order are retained. Serialized
tags are `fastLowpass`, `selectableFilter`, `bassShelf`. `EffectParams`
delegates sanitization and indexed controls to the existing parameter types;
`AnyEffect` holds the actual three processors and delegates all realtime and
metadata calls. Wrong-kind updates return false without changing state.
All three have no gain-reduction meter and maximum PDC latency zero.

The acoustic implementation and its reference suite are unchanged in this
window. The entire existing `EffectSlot` section matches the prior source
after newline normalization. No utility smoothing, latency, readiness,
warm-up, bypass or removal policy is edited. The descriptor example already
derives its entries from `ALL` and requires no source patch.

`tests/filter_family_registry.rs` adds six checks: stable kinds/tags/names and
default/control metadata; JSON defaults and malformed value rejection;
sanitized finite settings; sample-identical typed-versus-enum audio dispatch
including automation/noops/reset/tempo and every readiness/tail call;
actual slot transfer and mix; existing bypass/dormant-wake behavior; and
guarded alloc/realloc/free for enum and slot callback paths. Invalid JSON
types/modes are rejected by serde. Valid numeric JSON is preserved on parse
and sanitized by the existing explicit sanitization/processor-update policy.
Construction, prepare and destruction remain outside allocator guards.

The two separately granted generic test seams change only imports/three
exhaustive random-setting arms in `tests/dsp/realtime.rs` and three appended
labels in `tests/dsp/params.rs`. All original assertions and utility cases
are preserved. No utility test file is edited.

Executed checks for this registry window:

- `cargo test -p windfall-dsp --test filter_family_registry -- --nocapture`:
  all six new tests passed.
- `cargo test -p windfall-dsp --all-targets`: 299 passed, four ignored
  optional measurements/renders; breakdown 128 library, 152 existing DSP,
  13 acoustic references and six registry tests. The descriptor example
  also compiled. Existing automatic TS export tests wrote only to private
  `target/e1-bindings` via `TS_RS_EXPORT_DIR`; committed artifacts were
  untouched. No `gen-bindings` script or WASM build ran.
- `cargo clippy -p windfall-dsp --all-targets -- -D warnings`: passed.
- Changed Rust files pass `rustfmt --edition 2024 --check`; whitespace and
  the unchanged slot-source comparison passed.

Cargo uses the same MSVC environment, one job/test thread and private native
target as the acoustic stage. The first all-target compile identified the
old exhaustive allocator-test match; only its granted new arms were added.
No downstream crate, frontend or full workspace check is claimed.

The following proposal established the minimal downstream windows. The parent
subsequently authorized the test-only paths and mechanical appends described
in the next section; production model/engine/UI edits remain closed:

1. **Project acceptance:** add an auto-discovered
   `crates/windfall-project/tests/filter_family.rs` covering add/replace,
   save/load, malformed/partial settings, whole-params and indexed edits,
   undo/redo, all seven modes, automation target ranges and unchanged
   control indices. `src/model.rs` already reexports the DSP union;
   `lower.rs::add_effect` uses `kind.default_params()` and `checked_effect`
   iterates descriptors. No duplicate project union or production model/
   command/check/lower/edit/patch/lib edits are proposed without an actual
   failing contract. No production project window proved necessary.
2. **Engine acceptance:** the actual staged exhaustive consumer is
   `crates/windfall-engine/tests/engine/realtime.rs::effect_at` at the
   `Balance | ... | Distortion` arm. Append the three new kinds to that
   descriptor-driven arm, preserving its body and allocator assertions.
   Add a new filter-family engine test module and its `tests/engine/main.rs`
   registration (or a standalone harness using existing public test
   contracts) for chain-position tone/impulse behavior, all modes,
   automation, latency/tail/gap, live-versus-offline export and stem parity,
   callback allocations and accepted removal/restore continuity.
   `src/rack.rs::EffectUnit::build` already calls `AnyEffect::new`; no
   alternate adapter/routing path or Plan/Rack/State production edit is
   proposed unless those tests reveal a specific defect. Utility acceptance
   remains required; no hosting fixture/hostbridge activation is proposed.
3. **Artifacts and UI acceptance:** after project/engine source acceptance,
   grant normal binding/descriptor generation (private target
   `target/e1-bindings`) and the shared-WASM refresh through the parent's
   serialized artifact owner. The generated `EffectKind`/`EffectParams`,
   four new parameter/mode type files, binding index and descriptor JSON
   then carry this stable API. Mechanically append the three keys to
   `apps/desktop/src/features/params/access.test.ts`'s exhaustive
   `Record<EffectKind, true>`. Add a focused filter-family UI test under
   `features/effects` exercising mixer add/reset/replace, all mode choices,
   frequency/Q/gain controls, undo/redo, persistence and automation actions
   using the actual command-backed simulation. Existing
   `features/params/descriptors.ts` derives `EFFECT_KINDS` from JSON, and
   `features/effects/effect-editor.tsx` already selects
   `GenericParamEditor` for these kinds; no cosmetic EQ alias or custom
   editor/menu source change is required by the inspected contracts. If
   a compiler exposes another exhaustive consumer, name its exact hunk
   before expanding ownership. IPC/backend paths remain closed; do not
   introduce a second implementation of project commands.

Full parity still requires independently reviewed native/project/UI/export
evidence and any requested device/listening checks. Passing the DSP suite
does not close the engine utility restore issue or establish a usable UI.

## Downstream test-only acceptance checkpoint

This source increment is based on immutable registry commit
`25b692b3c4f58d28adfad9a5935b258e5005190e`. It changes no DSP implementation,
registry, project production, engine production, UI production, dependencies,
lockfile or file version. The exact owned source paths are:

- New `crates/windfall-project/tests/filter_family.rs` (four tests).
- New `crates/windfall-engine/tests/engine/filter_family.rs` (six tests),
  `tests/engine/main.rs`'s single `mod filter_family` registration, and only
  three appended kinds in `tests/engine/realtime.rs::effect_at`'s existing
  descriptor-driven arm.
- New `apps/desktop/src/features/effects/filter-family.test.tsx` (13 tests),
  and only three appended `Record<EffectKind, true>` keys in the confirmed
  `features/params/access.test.ts` path.
- This acceptance record. Existing utility case bodies are untouched.

The project tests exercise actual `Document` commands, bounded settings,
coalesced gestures, noop/dirty behavior, rejected-index/kind/NaN atomicity,
all seven mode tags, version-one serde defaults, temporary-disk save/load,
every descriptor's automation range and target, curve edits and undo/redo of
removal/replacement. Project commands intentionally map infinities to range
endpoints through the existing `checked_setting` policy; this differs from
direct DSP nonfinite writes. Undo's id allocator is monotonic, so undo content
comparison preserves the live counter rather than asserting an id rewind.

The engine tests use the existing public `Rig` harness, avoiding any direct
`prepare_project` call. The parent's newer fallible preparation API can
therefore stay encapsulated in its existing harness. All nine processor paths
(fast, seven selectable responses, bass shelf) have routed stereo impulses
compared against independently expanded f64 bilinear numerator/denominator
recurrences. Separate 50/1000/10000 Hz left and 60/1200/12000 Hz right tones
provide 54 analytic gain comparisons. This checks the actual sampler/track/
master path, not a descriptor dispatch in isolation. Results at 48000 Hz:

| Measurement | Observed maximum | Acceptance bound |
| --- | --- | --- |
| Absolute impulse sample error | 3.0267984e-9 | 2e-6 |
| Tone gain error / max(reference gain, 1) | 7.446086640503532e-7 | 3e-4 |
| Guarded callback alloc/zeroed-alloc/realloc/free | 0 calls | 0 |
| Reported live latency | 0 frames | 0 |

Each descriptor control is automated through real project automation clips;
live blocks of 1 and 137 frames, offline blocks of 101 frames and both
`TrackOutputs` and `ToMaster` stems are sample-identical. Automation changes
the signal relative to static settings. A last-song-frame impulse demonstrates
real post-boundary ringout in automatic-tail export. Repeated identical plans,
parameter edits, bypass/wake, removal and restore after departure settles
are processed under the existing global engine allocator guard. This is
headless signal evidence, not a device deadline or listening result.

The UI tests use generated descriptors, real generic controls, mixer actions,
stores and the shared Rust WASM document. Only jsdom's unavailable resize
surface and toast display are replaced. They exercise actual menu discovery,
frequency/Q/gain keyboard edits and indices, all seven mode choices, gesture
undo/redo, reset and noop, dry/wet/bypass/copy/replace, backend save/reopen,
automation from actual control context menus and restoration after removal.
Persistence equality uses the authoritative document snapshot: incremental UI
patches omit the id allocator and are not a complete serialized project.
The real app's `ValueContextMenus` provider wraps the mixer, as it does in
production, so automation actions are not substituted by direct test hooks.

Executed checks for this increment, with one Cargo process, one job/test
thread, MSVC setup and private worktree targets:

- `cargo test -p windfall-project --test filter_family`: four passed.
- `cargo test -p windfall-engine --test engine filter_family:: -- --nocapture`:
  five passed, one **failed**, described below. The failure is active, not
  ignored. The five independent cases also pass when the known failing case
  is explicitly skipped for isolation; that run is not an all-green suite.
- `cargo test -p windfall-engine --test engine
  realtime::effects_and_instruments_never_allocate_or_free_on_the_audio_path
  -- --nocapture`: passed with all 15 kinds in the existing exhaustive guard.
- Strict scoped Clippy for the new project target and engine test target:
  passed (`cargo clippy -p windfall-project --test filter_family -- -D warnings`
  and `cargo clippy -p windfall-engine --test engine -- -D warnings`).
- `scripts/gen-bindings.sh target/e1-bindings/generated`, with Cargo target
  `target/e1-bindings`: generated 174 matching files for validation.
- `scripts/build-sim.sh`: rebuilt actual shared WASM in this worktree's
  private `target/sim` (1881168 bytes). SHA-256:
  `9feb181f0a3d6aed64d9aabd26c61cedcf015b1437889bfa5ca7b309d18dde3a`;
  recorded source-input checksum:
  `26ac50ad9e44561b20ca18d811b00fc8f66e98082d9ecbe89580c07791cc250f`.
- Focused Vitest with the regenerated artifacts: 70 passed (13 new filter
  tests plus 57 descriptor/access tests), 9.43 seconds total headless walltime.
  Frontend dependencies were reused through a local junction; no install ran.
- `tsc -b`, focused ESLint, changed-file Rustfmt/Prettier and `git diff --check`:
  passed. All regenerated tracked artifacts are restored and the four new
  generated type files removed before the source-only commit. Parent commits
  matching bindings/descriptors/WASM; this child hands over no artifacts.

### Active utility composition failure and exact next integration action

`restore_during_removal_preserves_the_current_wet_history_and_splice_share`
keeps a filter's same slot/id, removes it after 4096 tone frames, processes
80 of the 240 departure frames and restores it before departure ends. The
last departure sample agrees with `dry + (wet - dry) * 161/240`. The restored
first sample must preserve the current mixture and histories, allowing one
frame of fade movement. On this branch's older utility policy, Fast lowpass
1000 Hz/Q=1.3 instead starts with dry output:

- Actual restored first left sample: 1.4208998e-16.
- Retained-history/current-share reference: 0.0018127069.
- Allowed one-frame movement plus numeric tolerance: 0.0000132590485.

The generic transfer, steady restore, automation and exported audio tests
remain green. The failure was sent to the parent as a concrete new signal
fixture for the already-owned utility restore-during-removal repair. E1 does
not alter `Plan`, `Rack`, `State`, effect-slot policies or readiness to hide
it, and does not import the parent's production repairs into this branch.

The exact remaining action is to compose this source increment with the
accepted utility repair, run the active restore fixture and the scoped engine
cases afresh there, and independently review the composed project/UI tests
using the parent's committed matching artifacts. The model union/lower,
engine constructor and generic UI already consume the registry, so no new
production adapter is proposed. No full-workspace, Tauri, full-Vitest, device,
listening, hosted-plugin activation or three-row completion claim is made.
