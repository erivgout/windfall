# DSP, plugins, analyzers and audio editor QA — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Independent source/parity review of the shared working tree. Scope is software feature acceptance against the summaries in `docs/parity/parity.json`; this report does not certify hardware audio, listening quality or other operating systems. Source remained read-only. Native and frontend execution are owned by the parallel verification workers; a source inspection is not a passing test run.

## Acceptance decisions

No new DSP instrument/effect/plugin/editor row can be unconditionally promoted from this review alone. Existing accepted effects have substantial focused signal and integration coverage, rather than registry presence alone. The following full row IDs are eligible to retain done when the matching fresh native and frontend suites pass:

- `inst-3x-osc`: oscillator/envelope signal tests in `crates/windfall-dsp/tests/dsp/synth.rs`, production engine instrument/sequencing tests and generic instrument controls.
- `fx-fruity-compressor`, `fx-fruity-reeverb-2`: numerical compressor transfer/attack/release and reverb decay/damping/diffusion tests in `crates/windfall-dsp/tests/dsp/{compressor,reverb}.rs`, plus engine effect rendering and project parameters. Their current done records refer to the phase-2 core scope.
- `fx-fruity-balance`, `fx-fruity-center`, `fx-fruity-mute-2`, `fx-fruity-phase-inverter`, `fx-fruity-soft-clipper`, `fx-fruity-fast-dist`, `fx-fruity-stereo-shaper`: DSP utility and utility-repair suites, engine `utility_effects.rs`, checked project persistence/history/automation, generic controls and accepted composition recorded in `docs/UTILITY-EFFECTS.md`, `docs/UTILITY-REPAIRS.md` and `docs/integration/2026-10-08-completion-audit.md`.
- `fx-fruity-fast-lp`, `fx-fruity-free-filter`, `fx-fruity-bass-boost`: independent DSP transfer/registry tests, `windfall-project/tests/filter_family.rs` (every control, disk reopen, history, all mode tags, invalid-edit atomicity, automation), engine `filter_family.rs` and shared-WASM controls; accepted downstream review and composition are documented in `docs/FILTER-FAMILY.md`.
- `fx-fruity-squeeze`: DSP `tests/lofi.rs`, project `tests/lofi.rs`, engine `tests/engine/lofi.rs` and UI coverage of its 16 controls; accepted composition is recorded in `docs/LOFI.md`.
- `fx-fruity-chorus`, `fx-fruity-flanger`, `fx-fruity-phaser`: independently authored DSP references, partition/reset/automation/real-time guards, project control persistence and engine live/offline/stem tests in the modulation suites. The accepted Phaser tail repair and matching generated-WASM/UI composition are recorded in `docs/MODULATION-EFFECTS.md` and the completion audit.

## New full-row candidate: spectrum and spectrogram

`vis-fruity-spectroman` has the summary “A real-time spectrum and spectrogram display.” Production spectrum and spectrogram panels, registered actions and app-shell integration implement both halves. Current views display 64 bounded power bands and up to 16 oldest-to-newest history rows. They consume production realtime frames without per-frame React renders; they clear omitted, invalid and replaced-stream data.

This row is a conditional done candidate, pending fresh execution of:

- `cargo test -p windfall-engine --test spectrum_view --test spectrogram_view`: two public-engine spectrum tests compare actual PCM power with a Parseval reference and check antiphase stereo, zero callback allocation and stream recovery; three spectrogram tests cover ordered bounded publication, empty/seek/project invalidation and overflow recovery.
- Frontend `features/spectrum/{view,panel}.test.tsx` and `features/spectrogram/{view,panel}.test.tsx`: finite/invalid/empty/bounded rendering, lifecycle and action/dialog routing.
- Matching generated bindings/runtime verification from the parent run. `2026-10-09-native-bindings-tests.log` contains 10 passing verifier regression tests, but these verifier tests alone do not establish the current generated files match Rust.

No effect-slot placement or parameter serialization is needed for these view-only summaries. This candidate does not close `vis-wave-candy`, which additionally requires configurable oscilloscope/vectorscope/meter behavior, nor `fx-fruity-parametric-eq2`, whose spectrum must appear within the EQ editor.

## Explicit unfinished scope

- `inst-fruity-granulizer`, `inst-fruity-slicer`, `inst-slicex`, `inst-wave-traveller`: new DSP primitives exist, but `granular/params.rs` defaults the sample table to zero length (silence), limits it to 4096 mono frames, and no app asset-loading path was found outside bindings. Generic knob access does not populate source PCM. Wave Ride's bounded sample controls are not a drawn-curve editor. These rows must stay open.
- `inst-fruity-slicer-2`: playlist detector/linked-source slicing is distinct from a playable note-triggered slicer. Existing summary explicitly requires both detection and independent triggering.
- `fx-fruity-convolver`: correct partitioned kernel/reference tests are valuable, but default zero-length IR is a unit impulse; the payload is capped at 2048 mono samples. Only mix/gain are generic descriptors and no desktop IR load/edit path was found. This does not supply a usable sample-loading convolver workflow.
- `fx-transporter`: `TimeTransport` captures preceding history on a manual rising trigger. No transient-driven automatic capture was found.
- `fx-fruity-scratcher`: `Scratch` processes live captured history; the row requires a loaded sample. No loaded-sample asset workflow was found.
- `fx-gross-beat`: step volume gates and history transport are partial foundations. The declared playback-position and volume drawn-curve workflow is absent.
- `fx-frequency-splitter`: band splitting DSP does not establish independently routable host output bands. The real host-routing workflow remains missing.
- Other newly registered DSP kinds must be judged against their full individual summaries. Registry/defaults/descriptors, serde and generic access tests are insufficient proof of instrument/effect feature completeness.
- `fx-fruity-limiter`, `fx-fruity-parametric-eq2`: retain their explicit missing compressor/gate/history modes and embedded live-spectrum scope. Core Delay now has modulation; its updated acceptance decision appears below.
- `editor-edison`: the selected-clip editor really implements trim/extract/normalize/reverse/fade/silence/cut with derived WAVs, history and source/recording guards. Effect-slot recorder integration, cleanup DSP and advanced tools remain missing. Keep in-progress.
- `core-stem-separation`, `core-denoising`, `editor-newtone`, `editor-newtime`: analysis lifecycle and a numerical monophonic pitch foundation do not deliver these user workflows. The completion audit says the pitch kernel is integrated and 17 parent cases passed while pitch editing, realtime correction/harmony and production app registration remain open. The older pitch document's CLOSED/R2 wording is stale relative to that audit; this is not evidence that the editor is complete.
- `fmt-host-vst3`, `fmt-host-clap`, `fmt-host-plugin-manager`: real Windows fixtures support a substantial implementation. `docs/plugins/runtime-repairs.md` now records repaired R4 document-adoption/native-overflow paths with 87 host scoped tests, 29 desktop plugin tests and numerical/session output checks. The parity VST3 note still describes these R4 cases as under repair and should be refreshed after fresh native validation. Native editor/manual external-plugin operation, audio crash containment and other-platform desktop readiness remain separate open scope; the full host rows cannot be newly marked done here.
- `fmt-host-bridging`: an in-process scanner isolation test is not audio crash containment. Pending bridge activation, native fault, reset/backpressure/cancellation, editor/discovery and integration guarantees must be independently accepted before closure.

## Run evidence and disposition

The first combined native run failed because the disk filled (`2026-10-09-native-tests.log`, OS error 112). That is an infrastructure failure, not a pass and not a demonstrated DSP regression. `2026-10-09-native-tests-resume.log` was still compiling when this report was authored. Fresh results belong to the native worker's final report. Frontend source/type/build and focused results belong to the frontend worker's report. Until those outcomes arrive, the decisions above distinguish prior accepted software evidence from new conditional acceptance.

## Follow-up evidence and candidate scope refinement

Fresh frontend spectrum/spectrogram view/panel tests pass: four files, 13 tests, recorded in `2026-10-09-frontend-menus-spectrum.log` (five files/14 with a menu test). Current-WASM generic effect/editor mapping also passed 52 tests in `2026-10-09-frontend-effect-editors.log`. Fresh native analyzer results remain required for the spectrum candidate.

The initial review did not exhaustively reject every new synthesis row. Direct source inspection supports full behavioral candidates, contingent fresh DSP and cross-kind project/native-render/UI integration evidence: `inst-fruity-dx10` maps to FourOp's light fixed eight-voice FM bank (algorithms, numerical sideband spacing, serde, controls and real-time tests); `inst-plucked` maps to Pluck; `inst-sakura` maps to dispersive dual-string AcousticString; `inst-fruity-kick` maps to pitch-swept Kick; `inst-simsynth` maps to TripleOsc; `inst-transistor-bass` maps to AcidLine's internal step sequence, slide and accent. Generic controls can suffice for these summaries; bespoke artwork is not a completion requirement. The currently inspected focused project/engine synth suites primarily instantiate the basic subtractive synth, so cross-kind integration must still be established.

Conversely, `inst-boobass` explicitly requires monophonic behavior while FingerBass's physical engine currently supports eight voices. MatrixFm alone does not establish Sytrus's subtractive methods, and RingHybrid alone does not establish Toxic's built-in sequencer/effects. These are concrete summary gaps, not generic requests for further review.

### Six authored full-runtime synthesis acceptance cases

`crates/windfall-engine/tests/builtin_instrument_acceptance_qa.rs` now adds six public-API native acceptance cases. They are authored and formatted, not yet executed by this reviewer. The native worker owns `cargo test -p windfall-engine --test builtin_instrument_acceptance_qa`.

Each creates a real document/instrument/note, edits a musically meaningful family parameter, requires finite nonzero PCM and a relative audio-difference energy above 1e-4, restores exact sound through undo/redo across different render partitions, saves/reloads an actual `.windfall` file with exact retained instrument parameters and PCM, then compares actual public live processor output with offline PCM after reported host latency. The comparison uses a prefix before the loop boundary; it does not infer musical behavior from a no-panic registry sweep.

The family-specific changes and exact eligible rows are:

- FourOp FM operator level/ratio: `inst-fruity-dx10`.
- Pluck string brightness: `inst-plucked`.
- AcousticString dispersion/stiffness: `inst-sakura`.
- Kick pitch sweep: `inst-fruity-kick`.
- TripleOsc oscillator octave: `inst-simsynth`.
- AcidLine internal step pitch/accent/slide: `inst-transistor-bass`.

Their existing family DSP suites independently verify pitch/sidebands/algorithm spectra, physical dispersion/decay, pitch sweeps, oscillator response and internal sequencing/slide/accent. Passing these new document/runtime cases together with fresh family DSP and matching generic-instrument UI tests can close these six full summaries. Pending execution is not recorded as a pass, and this does not extend to sample-loading, resynthesis, multisampling or synthesis variants with additional missing requirements.


### Core delay rows: current source qualifies both implemented summaries

`fx-fruity-delay-2` is a full-summary candidate: core Delay implements independent stereo echoes, feedback, low/high-cut feedback filtering and signed stereo offset. Independent existing DSP tests verify exact echo spacing at multiple rates, geometric feedback, channel isolation, signed offset timing and frequency-selective repeat attenuation. Public engine `effects.rs` already exercises delay integration and tempo following; generic project command/history/automation and effect controls cover the registered Delay type.

`fx-fruity-delay-3` is also a full-summary candidate. The old parity statement that modulation is pending is stale. Current `delay.rs` appends persisted/automatable `modRateHz` and `modDepthMs`, a frame-clock sine LFO, bounded interpolated tap motion and reset semantics. Focused native tests verify zero-depth exact legacy output, nonzero-depth actual audio differences/finite output in stereo and ping-pong modes, bounded reads during time fades and held zero-rate phase/reset. Existing independent delay signal tests verify tempo-synced delay, feedback filters and saturation of loud repeats. The frontend worker repaired the specialized editor's missing modulation knobs and reports its exhaustive current-WASM editor mapping green.

New public native `crates/windfall-engine/tests/delay_modulation_acceptance_qa.rs` is authored to close the remaining integration-evidence gap. It dispatches the two actual descriptor commands, proves rate alone with zero depth preserves exact PCM, proves nonzero depth changes actual echo PCM substantially, checks undo/redo across render partitions, actual disk persistence of both controls and live/offline agreement after reported host latency. Native execution remains owned by the verification worker; authored tests are not claimed passing here.

`fx-fruity-delay` must remain open when mapped to core Delay: its summary requires inverted stereo as well as ping-pong and a low-pass filter, while core `DelayMode` has only `Stereo` and `PingPong`. EchoBank has a defined inverted mode, but an independent acceptance mapping/runtime test would be needed before using that separate processor to close this row.

`fx-fruity-delay-bank` and `fx-multiband-delay` have more implementation than their stale source-only notes imply. Current effect registry, parameter variants, generated descriptors and generic parameter-access tests include EchoBank and FrequencyDelay. The actual FrequencyDelay is sixteen bands with individual delay/level/pan; the three-band BandDelay processor is a separate type. EchoBank has eight chainable delay/filter units. The dedicated `delay_family.rs` has independent serial/parallel graph, signed sends, filter transfer, channel/pan and sixteen-band references plus stability/partition/RT checks. These summaries are plausible complete software candidates if their fresh family tests and document/native-render integration pass. Historical foundation documents list additional original-plugin controls beyond the brief parity summaries; those extra options do not automatically block the summaries, but source/registry existence alone does not supply native integration proof.

### Native compilation repair: bridge discovery metadata

The resumed native compile exposed incomplete integration wiring: the desktop plugin module omitted its existing Windows bridge module, the desktop manifest lacked the directly used `windfall-dsp` dependency, and bridge discovery called an unavailable `Control::describe` API. This reviewer repaired the owned plugin wiring and implemented the missing protocol end-to-end, preserving isolated discovery rather than substituting an in-process native call.

A fresh helper now returns authentic native parameter names, IDs/ranges/current values, automatable/read-only/stepped flags, native state and complete audio layout from its creating owner. Desktop discovery retains auxiliary input ports as the established in-process path did. Metadata/state/count/string/port validation and the existing 1 MiB encoded metadata limit apply on the wire; oversized descriptions return a recoverable control error. Inspection is refused after the helper processes audio so discovery cannot stop active playback. Native owner reactivation follows a refused query/save operation.

Two new wire unit regressions check state/metadata round-trip, malformed/duplicate/missing-state rejection and encoded-budget refusal. A real CLAP/VST3 helper regression compares the returned metadata/layout/state with a reloaded native reference, exercises subsequent actual audio, then proves that refusing a late inspection retains a healthy capture-capable playback helper. Targets were sent to the native worker; no passing execution result is claimed here. These repairs address compilation/integration readiness and do not close plugin platform/editor/bridge rows.

### Actual bridge sidechain defect repaired

QA found that desktop `plugins/bridge.rs` implemented only HostedEffect's ordinary process method. The trait's default selection/key methods discarded a chosen native auxiliary bus and its detector PCM even though the host bridge transport already supported them. The facade now forwards both `set_sidechain_input` and `process_sidechain`, including the existing bounded offline-sidechain path; ordinary process supplies no key so old detector samples cannot leak into later plain processing. Publication/acknowledgement and offline failure propagation use the same existing path.

Two new real desktop native CLAP/VST3 regression cases exercise the actual Audio trait adapter in both realtime and offline modes. A stereo detector [.25,.75] folds to .5 on a mono auxiliary port and must change main [.1,.2] into [.6,.7]. Explicitly selecting a nonexistent port preserves [.1,.2]; default selection uses the first auxiliary port; ordinary processing clears previous detector PCM. Realtime forwarding and bus selection have zero-host-allocation guards, and tests require real completed native blocks and healthy helper/error state. VST3 lacked any existing multi-input detector fixture, so a new test-only twenty-second class supplies a mono auxiliary port while preserving all prior class IDs/behavior; fixture scanner count assertions were updated. The standalone fixture must be rebuilt before execution. Native test results remain owned by the verification worker; authored regressions are not counted as passing evidence yet.

### Native runtime execution and fixture corrections

The helper-configured run in `2026-10-09-native-tests-all-runtime.log` passes both actual desktop CLAP/VST3 selected-sidechain live/offline regressions and both settled-gain capture regressions. The first invocation's four failures occurred before launch because the helper environment variable was absent. These passing focused regressions establish the repaired forwarding behavior; they do not establish whole plugin platform parity.

The wider N4 session cases require the actual current desktop executable, which implements both scanner and audio helper entry points. Supplying the standalone audio helper caused scanner refusal and unusable catalog entries before loading; the native owner was instructed to build/use `windfall-desktop.exe` and rerun without weakening those integration assertions.

Two host ABI tests retained obsolete v4 byte expectations despite the current v6 sidechain/event-reserve protocol. They now assert exact version 6 and exact header/slot/region sizes (256/226112/904704 bytes), reject all prior versions 1 through 5, and retain disjoint reset-publication checks. The shared fixture DLL copy is serialized before any parallel helper can map it on Windows. The VST3 event-flood fixture now emits 10000 events, exceeding the enlarged 9217-event native note/release storage; prior 5000-event traffic no longer exercised overflow. Loss visibility, unknown-block, no-false-acknowledgement and recovery assertions remain unchanged. These fixture corrections are authored and handed to the native worker for execution, not yet claimed passing.
