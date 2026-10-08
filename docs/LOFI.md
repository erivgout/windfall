# Lo-fi reduction

This source checkpoint continues immutable foundation `d073d6e3bd14042b11f27621c2d773432516fef9`, whose direct parent is `427ed0a7a07f3cae41ab1b91133b5e317b0f3dc9`. Windfall publishes its own **Lo-fi reduction** as append-only `EffectKind::Lofi` (position 15, JSON `lofi`). The actual `Lofi` processor implements public `Effect`/`ParamSet` and is adopted through `AnyEffect`/`EffectSlot`. Its 16 controls use existing project persistence, commands/history, automation, engine routing, offline/stem rendering and generic desktop editing. Parent-controlled parity accounting and independent Standards/Spec acceptance remain separate. There is no dependency/version/lock change.

## Sources, original implementation and license

The [Image-Line public manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Squeeze.htm), consulted 2026-10-08, describes bit reduction, preserved/replaced sample runs, independent/equal/double/half duration relationships, nominal replacement amplitude, filter cutoff/resonance, pre/post placement, mix and gain. Windfall implements those behavioral categories using original bounded equations, plus distinct capture-rate reduction and blended drive. The manual does not specify coefficients or quantizer rounding; no proprietary acoustic equivalence is claimed. The finite activity policy below is an explicit product difference. Vendor naming is used only in this documentation reference, not product labels.

The [Cycling '74 reducer reference](https://docs.cycling74.com/reference/degrade~/), consulted the same date, documents separate bit and resampling reduction with intentionally omitted interpolation/dither. It informs scope, not copied code. [Andrew Simper's linear trapezoidal SVF derivation](https://cytomic.com/files/dsp/SvfLinearTrapOptimised2.pdf) supplies primary mathematical context for a resonant lowpass. Windfall's private two-integrator solve below is derived from that filter topology; no source-code listing was copied. Its independent test reference expands the bilinear transfer into direct-form polynomials instead.

All code, test signals and examples are authored here. Existing gain, finite-state flush, rate sanitization and frame conversion helpers are reused under the repository's GPL-3.0-or-later license. Existing smoothing/filter blocks were read; no shared block is modified. No proprietary binary/code, sound, preset, screenshot, art or UI was copied. No assets or dependencies were added. License: existing root `LICENSE` and [GNU GPL v3](https://www.gnu.org/licenses/gpl-3.0.html).

## Stable published API

`LofiParams` is Debug/Clone/Copy/PartialEq/Serialize/Deserialize/TS/Default/ParamSet. JSON is camelCase with struct-level serde defaults. Both choice enums are Copy/serde/default/TS, and all three types have `#[ts(export)]`. The original nine controls retain indices, ranges and defaults; seven append after them. Future controls must append. Version-one projects accept `{"type":"lofi"}` and an original nine-field object through defaults, without a format bump. Raw serde/TS derivation does not itself sanitize a user-created struct.

| Index | Rust / JSON | Plain label | Range, default |
| --- | --- | --- | --- |
| 0 | `bits` / `bits` | Resolution | integer 2–16, 8 |
| 1 | `quantize` / `quantize` | Quantization | fraction 0–1, 1 |
| 2 | `rate_ratio` / `rateRatio` | Rate reduction | host/capture ratio 1–64, 4 |
| 3 | `drive_db` / `driveDb` | Drive | dB 0–36, 6 |
| 4 | `distortion` / `distortion` | Distortion | fraction 0–1, 0.25 |
| 5 | `cutoff_hz` / `cutoffHz` | Tone cutoff | Hz 20–20000, 8000 |
| 6 | `filter` / `filter` | Tone filter | fraction 0–1, 1 |
| 7 | `mix` / `mix` | Mix | fraction 0–1, 1 |
| 8 | `output_db` / `outputDb` | Output | dB -24–12, -3 |
| 9 | `preserve_ms` / `preserveMs` | Preserve time | ms 0–1000, 10 |
| 10 | `replace_ms` / `replaceMs` | Replace time | ms 0–1000, 2 |
| 11 | `run_relation` / `runRelation` | Run relationship | choice 0 independent, 1 equal, 2 double, 3 half; 0 |
| 12 | `replacement_value` / `replacementValue` | Replacement value | fraction 0–1, 0 |
| 13 | `replacement` / `replacement` | Replacement | fraction 0–1, 0 |
| 14 | `resonance` / `resonance` | Resonance | fraction 0–1, 0 |
| 15 | `placement` / `placement` | Filter placement | choice 0 post, 1 pre; 0 |

Descriptor scales are linear except logarithmic ratio/cutoff. Choice JSON tags are `independent/equal/double/half` and `post/pre`; unknown tags are rejected. Integer JSON must fit u8. Direct DSP float controls clamp finite values, replace NaN/Inf with their defaults, and clamp bits to 2–16. Numeric integer writes round halfway away from zero. Numeric choices round/clamp; nonfinite writes preserve the current choice. Project commands reject NaN atomically and treat infinities as descriptor endpoints, matching the existing document contract. Serialization alone is not validation.

`LofiParams::neutral()` disables quantization, distortion, replacement and filtering, selects ratio one, mix one and output zero dB. It preserves finite floating-point input exactly, including signed zero and headroom through f32::MAX. `mix=0` with zero output gain also preserves input exactly while histories run. Output gain follows the inner mix and therefore trims both dry and wet. Outer mixer slot mix/bypass remain separate existing controls. The existing EffectSlot host input guard clamps finite input to ±1000 and converts nonfinite host input to zero before any effect runs; direct Lofi headroom/invalid-input guarantees apply before that host guard. This feature does not change the shared guard.

Prepare clamps finite sample rates to 1–384000 Hz; NaN/Inf use 48000 Hz. Zero/negative/tiny rates clamp to one. Advisory max block is unused: storage is fixed and arbitrary nonempty block lengths are valid. The direct contract requires equal stereo lengths and finite samples. Debug asserts on nonfinite signal input; release exposes offending NaN/Inf unchanged without putting it into that channel's history. It does not conceal invalid signal state as plausible finite audio.

## Original transfer, clocks and channel independence

Order: input → optional pre tone → drive blend → quantization blend → timed replacement → sample-and-hold → optional post tone → inner dry/wet mix → output trim. Transfer and histories use f64; frame-ramp coordinates and final samples are f32. Each channel owns its capture phase/held sample, compensated run phase, activity countdown and two tone-filter histories. Controls advance once per stereo frame. Clocks begin in sync under equal controls, while audio histories remain independent. Disabled replacement preserves antiphase under odd transfers; enabled positive fixed replacement deliberately introduces DC and breaks odd symmetry, without crossfeed.

Drive uses G=10^(driveDb/20), `S(x)=G*x/(1+abs(G*x))`, and `(1-d)*x+d*S(x)`. Saturation is odd, monotonic and bounded by ±1; its small-signal slope is G, without loudness compensation. Amount zero preserves headroom; partial drive retains the unshaped share.

For depth b, M=2^(b-1)-1, step Δ=1/M:

```
Q_b(x) = round_away_from_zero(clamp(x,-1,1)*M)/M
```

This has 2^b-1 symmetric midtread levels, exact zero and ±1 endpoints; one b-bit code is unused for symmetry. Two bits give {-1,0,+1}; one bit is excluded to retain zero. Half-step ties round away from zero. Full quantization deliberately saturates above full scale; partial quantization blends with the original share and retains headroom. Within full scale error is at most Δ/2. No dither/noise is added.

Resolution ramps in step size s_b=1/(2^(b-1)-1). Four fixed comparisons locate adjacent depths l and h=l+1, then blend their quantizers by `(s_l-s)/(s_l-s_h)`. Settled values exactly equal integer Q_b; intermediate grids are a smoothing morph, not a PCM-format claim. Step-space interpolation gives coarse grids sufficient transition time.

Capture phase starts at one. A frame with phase ≥1 captures the current replaced/quantized signal, subtracts one, then adds 1/R for the next frame. Otherwise it retains the held value, without interpolation. A 1e-12 crossing tolerance removes floating-point boundary error. Constant R captures at ceil(k*R), starting zero; N frames have floor((N-1)/R)+1 captures. Fractional R alternates neighboring integer hold lengths. Edits retain phase; settled unity captures/reanchors every frame. Blocks never restart phase. Maximum held age is 63 frames at R=64.

## Timed preserved/replaced runs

At prepared rate Fs, P=round(Fs*preserveMs/1000). Independent I=round(Fs*replaceMs/1000); equal I=P; double I=2P; half I=round(P/2). Times use host frames, independent of capture ratio/tempo. Maximum cycle is three seconds (one-second preserve plus double replace), held in phase/counters rather than an audio buffer. The independent replace knob retains its saved value while another relationship is selected.

Normalized run phase begins at zero. A phase below P/(P+I) preserves shaped/quantized audio; the rest blends toward nominal positive v: `(1-a)*x+a*v`. Compensated phase addition advances 1/(P+I) per frame and wraps, with a 1e-12 boundary tolerance. Settled durations follow the independent analytic modulo pattern `n % (P+I) < P`. Duration smoothing changes rounded frame counts without restarting normalized phase. P=0 gives all replacement, I=0 all preservation, both zero bypass replacement and freeze phase. Replacement precedes capture, so a held replaced sample can cross a preserved boundary.

**Finite activity policy:** raw input magnitude ≥1e-20 arms a per-channel one-cycle countdown. Replacement can generate nominal DC for at most a whole cycle after the last such input, then ceases. Idle silence remains silence, even at nonzero nominal value. Phase continues while idle; input onset does not restart it. The threshold agrees with the finite-state flush scale and is explicit, not NaN concealment. Amount zero retains the foundation reducer sound.

This is an intentional difference from literal continuous fixed replacement on silence; the manual does not describe gating. A continuous generator would need an infinite-tail host contract. Existing engine State sums finite usize tail/gap bounds, so an infinite sentinel would overflow those sums; this feature uses bounded activity and needs no controller/Plan/Rack/State/runtime/session production change.

## Tone, smoothing, aliasing and headroom

Resonance zero preserves the foundation's two monotonic real poles. f=min(cutoffHz,0.45*Fs), alpha=1-exp(-2*pi*f/Fs). Each pole updates y=(1-alpha)*old+alpha*input. This has exact DC gain one, convex histories and impulse `h[n]=alpha²*(n+1)*(1-alpha)^n`. The pole-frequency label is not the full cascade's exact digital -3 dB corner; at low normalized frequencies the cascade is about -6 dB at f and approaches 12 dB/octave attenuation.

Nonzero resonance morphs toward a second-order resonant lowpass, with Q=1/sqrt(2)+(4-1/sqrt(2))*resonance, g=tan(pi*f/Fs), k=1/Q. Stored s1,s2 use this original trapezoidal solve:

```
a = 1/(1+g*(g+k))
band = a*(s1+g*(input-s2))
low = s2+g*band
s1 = 2*band-s1
s2 = 2*low-s2
```

Blend monotonic and resonant outputs by resonance, then blend filtered/raw by tone amount. The independent reference uses a direct-form bilinear expansion of H(s)=1/(s²+s/Q+1): numerator [g²,2g²,g²], denominator [1+k*g+g²,2(g²-1),1-k*g+g²]. DC gain is one; resonant gain near f approaches Q, bounded at four. Pre tone changes what enters drive/quantization; post tone darkens the held output. Both sets of histories run continuously. During placement transitions both the pre-conditioned input and post-conditioned output crossfade over 5 ms; settled endpoints choose one location. Finite state below 1e-20 is flushed.

Every actual target change arrives after N=max(1,round(0.005*Fs)) stereo frames. Ramps use linear gain, ratio, fractions, filter alpha/g/k, placement fraction, run frame counts and quantization step size. Each evaluates frozen start + (target-start)*elapsed/N in f64, emits f32 and snaps exactly at N. Fast retarget starts from the last emitted value; identical sanitized writes and unrelated targets do not restart arrival. Empty blocks consume neither ramp/clock nor fresh state. Prepare/reset clear histories and snap targets; writes before the first actual frame also snap. Tempo is ignored. Quantizer, sample-hold and timed boundaries have intentional waveform steps: automation smoothing does not make arbitrary audio continuously differentiable.

There is no oversampling or anti-alias promise. Drive/quantizer harmonics and capture aliases fold into the host band intentionally. The post lowpass cannot undo in-band folded energy. Full drive and quantizer branches are bounded by ±1; partial processing retains caller headroom. At resonance zero and input bound H, final magnitude is at most 10^(12/20)*max(H,1). Resonant filtering can overshoot and changing coefficients do not have that convex amplitude bound; leave headroom. There is no output limiter. Extreme near-f32::MAX input, resonance or positive gain can expose Inf on final conversion. Exact neutral and dry endpoints do not clip or overflow.

## Realtime storage and reporting

All Lofi storage is fixed, including maximum-rate/block prepare. Process/set/reset/tempo allocate/reallocate/free zero bytes; no locks, waits, IO or unbounded loops. The sample loop performs no transcendental calculations. Actual parameter writes do a bounded number of gain/exp/tan conversions; the host may issue those writes every frame under automation, so that coefficient work remains part of callback cost. The quantizer locates grids with four comparisons. AnyEffect owns one off-callback Box, while EffectSlot/engine perform their existing off-thread preparation. The measured processor/parameter sizes and throughput appear below.

Latency, warm-up and delay readiness report zero. Hold age and filter phase/group delay are coloration, not fixed compensatable PDC. Frozen tail and gap are deliberately conservative:

```
f_min=min(20,0.45*Fs); f_max=min(20000,0.45*Fs)
g(f)=tan(pi*f/Fs)
r(f)=sqrt((1-g(f)/4+g(f)^2)/(1+g(f)/4+g(f)^2))
r_max=max(r(f_min),r(f_max))
tail=gap=ceil(160/-ln(r_max))+ceil(3*Fs)+64+N
```

Q=4 and cutoff endpoints bound resonant pole radius; monotonic poles decay faster. Two tone stages introduce polynomial decay factors. 160 resonant time constants plus maximum activity cycle, hold and full ramp provide margin for finite f32 history and rounding. This assumes frozen settled controls/no continuing input, not an acoustic duration or termination promise under continuing automation. The same conservative gap may extend quiet auto-tail export to its configured cap. Engine quiet measurement remains the final termination mechanism.

## Usable defaults and examples

Default eight-bit/quarter-rate capture, gentle drive, 8 kHz post tone and -3 dB trim are audible immediately; replacement/resonance are opt-in. Windfall-authored starting points:

```rust
use windfall_dsp::{Effect, FilterPlacement, Lofi, LofiParams};
let mut effect = Lofi::default();
effect.prepare(48_000.0, 512);
let texture = LofiParams {
    bits: 10, quantize: 0.6, rate_ratio: 2.5,
    drive_db: 6.0, distortion: 0.2, cutoff_hz: 6000.0,
    filter: 0.75, mix: 0.4, output_db: -3.0,
    ..LofiParams::default()
};
let gate = LofiParams {
    preserve_ms: 10.0, replace_ms: 5.0,
    replacement: 1.0, replacement_value: 0.0, ..texture
};
let resonant_pre = LofiParams {
    cutoff_hz: 1500.0, resonance: 0.65,
    placement: FilterPlacement::Pre, ..gate
};
effect.set_params(&resonant_pre);
let (mut left, mut right) = ([0.0; 137], [0.0; 137]);
effect.process(&mut left, &mut right);
```

Desktop Add effect/actions and the generic editor derive all plain labels from real Rust descriptors. Numeric controls support keyboard/gestures/default reset; run relationship and filter placement use radio choices with keyboard navigation. Right-click automation is available for every control. Existing slot bypass/mix, copy/replace, undo/redo and save/reopen retain the full settings.

## Acceptance and reproduction

The owned DSP tests use nearest-code distance, analytic capture/modulo counts, closed-form monotonic impulse/DTFT, direct-form resonant reference and separately synthesized 16x high-rate nonlinear spectra. They never call production clock/quantizer/filter helpers. Cases cover all 2–16-bit codes, signed ties/endpoints/saturation/headroom, integer/fractional clocks, irregular partitions, independent stereo/noise/antiphase, representative 44.1/48/96/192 kHz and invalid/tiny rates, cutoff/DC/impulse/spectra/stability, intentional aliases, retarget/noop/empty/reset/silence, all new run/placement/resonance controls and exact block parity 1/7/64/137/512. Four allocator entry points are guarded, with an independent positive control. Debug-invalid/release-invalid tests are complementary mandatory cases; only the explicitly invoked CPU measurement is optional.

Project tests cover all 16 gestures, noops, undo/redo, file reopen, version-one defaults, choice tags, invalid atomicity, automation ranges and curve removal/restoration. Engine tests drive actual routing, all-control curves, live/offline/both stem modes, tail and callback guards. Desktop tests use actual regenerated Rust WASM/descriptor data, replacing only jsdom layout surfaces, and check add/copy/replace, numeric/choice controls, every automation index and persistence/history.

Run Git Bash from this bound worktree:

```bash
source scripts/msvc-env.sh
export CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target/e4-native
export TS_RS_EXPORT_DIR=C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-lofi-e4/target/e4-native/ts-bindings
cargo test -p windfall-dsp --test lofi -- --test-threads=1
cargo test -p windfall-project --test lofi -- --test-threads=1
cargo test -p windfall-engine --test engine lofi -- --test-threads=1
cargo clippy -p windfall-dsp --lib --tests -- -D warnings
cargo clippy -p windfall-project --test lofi -- -D warnings
cargo clippy -p windfall-engine --test engine -- -D warnings
cargo test --release -p windfall-dsp --test lofi -- --test-threads=1
cargo test --release -p windfall-dsp --test lofi scoped_cpu_throughput -- --ignored --exact --nocapture --test-threads=1
scripts/gen-bindings.sh
scripts/build-sim.sh
# From apps/desktop, existing node_modules executables (no install):
# vitest run src/features/effects/lofi.test.tsx src/features/params/access.test.ts
# tsc -b; eslint src/features/effects/lofi.test.tsx src/features/params/access.test.ts
# Restore/exclude generated bindings and sim artifacts before source commit.
git diff --check
```

Task-local target and TS exports are ignored; no broad workspace build/install is used. Generated descriptors/bindings/WASM are validated locally, then excluded from this source checkpoint. Parent regenerates combined artifacts. No push, PR or release is performed here.

First-run failures and repairs, with all authored numerical thresholds retained:

- Test compilation: `E0061`, `LofiParams::decl()` lacked the ts-rs 12 `&Config` argument; changed to `decl(&Config::default())`. `E0308`, a consumed output iterator yielded f32 while the test pattern expected `&f32`; corrected the pattern.
- `callback_allocation_zero_for_all_extremes_and_fast_automation`: the processor guard returned `[0,0,0,0]`, but the guard's positive-control assertion expected `[1,1,1,2]` and got `[0,1,0,1]`. Optimization removed the unused alloc/realloc/free group. Opaque `black_box` pointer/layout use made all entry points observable; the exact positive-control assertion passes. A separate zero-call construction/maximum-prepare guard was then added.
- `smoothing_noops_fast_retargets_empty_reset_and_step_bounds`: at frame 238, `abs(current-previous) <= 0.02` failed for `0.9392756 → 0.95935667`. A linear bit-number ramp spent too little time near the coarse grid. Changed the internal resolution coordinate to quantization step size with bounded adjacent-grid interpolation, retaining the same 240-frame arrival and 0.02 assertion. The unchanged assertion passes.
- Strict Clippy: `manual_range_contains` on the impulse output bound `y >= 0 && y <= 1`; used `(0..=1).contains(&y)` with identical bounds. Strict scoped Clippy then passed.
- Expanded acceptance: all 225 resolution-depth pairs at seven constant amplitudes passed the unchanged 0.02 step bound. `drive_arrives_in_five_ms_at_representative_rates_with_analytic_ramp_reference` then failed `abs(actual-reference) < 3e-6` at 96000 Hz/frame 436: actual `0.9070949`, analytic `0.9070918881618402`. Replaced only the private lo-fi ramp with fixed-start/integer-frame interpolation in f64; the same 3e-6 assertion passes at all four representative rates. No shared smoothing block was changed.

Extension failures and repairs (old signal thresholds unchanged):

- `timed_preserve_replacement_relationships_values_zero_runs_and_hold_order`: 44100 Hz/frame 132300 expected the next preserved value 0.5 but got replacement 0.3. Compensated run-phase addition repaired long-cycle drift; exact boundary/count assertions pass through 192 kHz.
- Existing plain-name test lacked the appended name; appended Lo-fi reduction without changing old names/order. The three-filter registry test expected 15 kinds; appended total 16 and bounded its unchanged old three-kind assertion to positions 12..15. Lo-fi separately asserts position 15/tag/default/controls.
- Desktop typecheck reported TS6133 for an unused copied `useUiStore` import; removed it. The new UI replacement case expected a fresh id from replacing lo-fi with itself (`expected 30 not to be 30`). Existing actions deliberately disable same-kind replacement; corrected the test to assert its unchanged settings/history, replace through Balance, then back to Lo-fi with a fresh id/defaults. No old assertion or product behavior was weakened.

Final verification on Windows x64, 2026-10-08: all 21 scoped lo-fi signal tests passed in debug and release; the optional CPU test was separately invoked and passed all eight cases. Broader affected-crate regressions passed: DSP library/all integration targets (351 tests before the final two stress cases, plus those two in the final scoped run), project library/all integration targets (335), engine public integration target (285). Existing DSP optional CPU tests remain their pre-existing ignores; no new signal case is skipped. Strict DSP lib/tests, project lo-fi and engine integration Clippy passed with `-D warnings`. Desktop focused regression tests passed 103/103 across lo-fi, filter-family, parameter access and generic editor; tsc -b and scoped ESLint passed.

The four-entry DSP allocator guard reported [0,0,0,0] for alloc/zeroed/realloc/free, including maximum-rate/block direct construction/prepare and all-control set/process/reset/tempo. Its independent positive control reported [1,1,1,2]. Published slot controls and engine callback guards reported zero calls. The processor is **624 bytes**, including **56-byte** Copy parameters, 17 frame ramps and both channels; direct prepare uses **zero heap bytes**, including at 384 kHz and advisory usize::MAX block size. The published AnyEffect allocates one 624-byte Box off callback. Existing prepared EffectSlot heap payload is that Box plus two four-f32 zero-latency delay lines (32 bytes) and 9*max(1,max_block) bytes of stereo scratch/alignment storage, excluding inline slot bookkeeping and allocator overhead; at block 512 that is 5264 bytes. No new duration-sized storage exists.

Frozen tail/gap: 581785 / 633229 / 1266393 / 2532721 frames at 44.1 / 48 / 96 / 192 kHz. Tests drain a maximum three-second replacement cycle and Q=4 tone with an independent silent channel, and retain the foundation's maximal-finite-headroom tail test. These conservative bounds are about 13.2 seconds, not claims of that audible decay duration. At artificial 1 Hz, a 64-frame hold can outlast a whole three-frame activity window before another capture; the test explicitly accounts for that policy.

Local generated TypeScript/descriptors contained the appended kind and all 16 controls. The actual Rust WASM build was 2001474 bytes and passed source/hash validation; those artifacts are excluded from the source checkpoint. A T3 collaborative browser at 1280×800 added Lo-fi reduction to Clap, displayed every control/default (444px editor width = 444px scroll width), changed run relationship to Double and placement to Pre, keyboard-edited Replacement to one and undid to zero, then created a Preserve time automation clip. The effect chain has vertical scrolling for the last row; no horizontal clipping or browser alert was observed. This checks the browser document/UI, not a native hardware audio device.

The release headless throughput run used 30000*128 stereo frames per case, seeded input, buffer copies and black_box observability; automated timing includes descriptor writes. Automated cases visit each of all 16 controls and alternate minimum/maximum on successive whole descriptor cycles. Hardware identifier: Intel64 Family 6 Model 183 Stepping 1, GenuineIntel. Jobs/test threads were one; the machine was not pinned or isolated from other work. These are one-run wall-clock observations, not deadline or listening claims.

| Rate | Default M frames/s | Default µs/128 | Automated M frames/s | Automated µs/128 |
| --- | --- | --- | --- | --- |
| 44100 | 17.484 | 7.321 | 28.109 | 4.554 |
| 48000 | 17.327 | 7.387 | 28.099 | 4.555 |
| 96000 | 17.405 | 7.354 | 24.038 | 5.325 |
| 192000 | 17.627 | 7.261 | 18.294 | 6.997 |

Automation sometimes skips quantizer work at zero amount, so its faster cases are not a worst-case CPU ceiling. At 192 kHz the default/automated walltime fractions were 1.089%/1.050% of generated audio duration. Maximum processor storage is fixed; maximum realtime CPU safety is not inferred from this measurement.

## Source checkpoint paths

- `crates/windfall-dsp/src/lofi.rs`, `src/effect.rs`, `src/lib.rs`
- `crates/windfall-dsp/tests/lofi.rs`, `tests/dsp/params.rs`, `tests/dsp/realtime.rs`, `tests/filter_family_registry.rs`
- `crates/windfall-project/tests/lofi.rs`
- `crates/windfall-engine/tests/engine/lofi.rs`, `tests/engine/main.rs`, `tests/engine/realtime.rs`
- `apps/desktop/src/features/effects/lofi.test.tsx`, `apps/desktop/src/features/params/access.test.ts`
- `docs/LOFI.md`

These are source-only changes on the existing bound branch. The immutable foundation is not amended/imported; parent owns combined generation, reviews, parity accounting and push.

## Remaining limits

Finite replacement on silence intentionally differs from a continuous nominal-value generator. No proprietary acoustic/byte parity, device deadline, listening, installer or cross-platform qualification is established here. Independent Standards/Spec acceptance and parent combined generation/parity decisions remain open. No roadmap/parity accounting is edited. Other E4 drawn waveshaper, modular rack and guitar/pedal/cabinet processors remain separate work. This checkpoint adds no controller/Plan/Rack/State/runtime/session production seam.
