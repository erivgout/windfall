# Windfall implementation roadmap

## Current execution priority — user instruction, 2026-10-08

New implementation coverage and deferred artifact/QA work are tracked in
[FEATURE-PASS.md](FEATURE-PASS.md).

Implement every feature in `WINDFALL_PLAN.md` and all non-excluded parity rows
first. The user explicitly deferred QA, independent reviews, builds and test
execution until the complete feature implementation pass is finished. Do not
gate new feature work on review rounds or verification. Record implemented
features awaiting QA without presenting them as verified or release-ready.
No GitHub CI or Actions may be added, enabled, dispatched or run. Historical
verification commands below are deferred follow-up work, not current gates.

The full project objective remains in scope. Native plugin integration,
remaining instruments/effects, analysis workflows, editing/recording tools,
extras and release functionality remain required; defer verification rather
than reducing feature requirements or replacing real behavior with placeholders.

Planned against **`c37ae0b938a60ce90b3f86878467b5670b35c24e`**, 2026-10-07. This is an implementation handoff, produced with the improve skill. It changes neither the parity matrix nor source code. Reconcile it against the integrated branch before dispatching work; the six active areas below are reservations, not claims that their work has landed.

The objective remains the whole of `WINDFALL_PLAN.md`, phases 0–7, and every row of `docs/parity/parity.json`. The base matrix has **342 rows: 59 done, 28 in progress, 253 todo, and two justified won't-do rows**. Its 17.8% accounting score is not a measure of acoustic equivalence, platform coverage, or release readiness. The plan's headline 39 instruments/71 effects/6 visuals is smaller than the matrix's 41/80/7 because the matrix also includes manual-only entries. Do not delete those entries to improve the score.

Current integrated status is tracked in [PARITY.md](parity/PARITY.md) and the [completion audit](integration/2026-10-08-completion-audit.md). The inventory below preserves the original planning snapshot, including its original statuses.

## 1. Dispatch order and ownership

Finish and integrate the current batch first. Record the actual accepted commit for each reservation in the next dispatch prompt; none was available to this base-commit survey.

| Reservation | Current owner’s work | Hold for that owner |
| --- | --- | --- |
| A-MIDI | Native MIDI devices, note input/output and hardware/controller integration | Hardware backend, live note capture, MIDI settings/controller UI and shared MIDI event changes |
| A-EDITOR | Native audio editor | Audio editing jobs, editor UI, source replacement/attachment and editing commands |
| A-SLICE | Playlist slicing and phase-3 slicer work | Clip splitting/source-offset semantics, transient/slice maps and slicer UI/model |
| A-VST3 | Desktop VST3 state integration | Active audio-half capture, plugin owner tokens/revisions/fingerprints, save/export/editor synchronization |
| A-PIANO | Parallel piano tools | Piano transforms, specialist tool UI and any associated note-model fields |
| A-LIBRARY | Parallel browser library | Recursive search, tags/favorites/backups/catalog UI and library persistence |
| Integration | Parent | Generated TypeScript/descriptors/WASM, shared seam reconciliation, root README/parity changes, final CI and publication |

Do not independently reimplement these areas while they are reserved. After merging, inspect their actual tests and unfinished cases rather than assuming every related umbrella row is closed.

**Recommended next batch: four bounded assignments, N1–N4 below.** N1 completes sampler stretch, N2 adds seven real utility effects, N3 makes projects portable, and N4 delivers a first Windows audio-plugin process bridge. These address distinct product gaps and prepare several later families. N4 is larger and riskier; give it a dedicated owner and review its transport contract before expanding scope. Do not simultaneously dispatch a second DSP-enum owner or a second plugin-runtime owner.

Shared files are serialized integration points, not four-way ownership: `windfall-project/src/{model,command,lower,edit,check,patch}.rs`, `windfall-ipc/src/lib.rs`, `session/mod.rs`, `commands.rs`, `lib.rs`, `lib/ipc/{backend,tauri,mock}.ts`, and action registration. A worker may propose the narrow changes listed in its brief, but the parent must assign an exclusive edit window or apply that seam patch. Separate new modules do not make simultaneous changes to these files safe. In particular, A-MIDI/A-PIANO may already change notes and A-EDITOR/A-SLICE may change prepared audio; N1 waits for their combined base.

Dependency order after the active batch:

1. N1/N2/N3/N4 can proceed on isolated worktrees with the shared-file rule above. Merge and regenerate once the accepted changes are combined.
2. T0 rack/sample workflows follows the active piano/editor work; T1 timing/regions and T2 event/control/sidechain contracts; then T3 mixer workflows and T4 native recording refinement. T5 presets follows N3 and A-LIBRARY. T6 render/freeze follows T1 and A-SLICE.
3. T7 modular rack follows T2 and N4. Run the instrument/effect families in section 6 against these reusable contracts, one registry owner at a time.
4. M1 worker/model infrastructure can start independently after A-EDITOR defines its attachment contract. M2–M4 then implement actual inference/editor workflows. F1 import fidelity follows each newly supported timeline/event/processor capability rather than waiting until the end.
5. X1–X5 extras reuse commands, control routing, library/content and the worker contract. P1 native platform hosting and R1 release engineering can start earlier; their external verification remains a separate gate.

Effort is qualitative: M is a bounded multi-day change; L is multiple changes with a dedicated review; XL needs several staged deliveries. It is not a calendar estimate. Source-backed missing behavior has high confidence; quality, throughput and external compatibility remain measured acceptance questions.

## 2. What the base actually establishes

This survey read every matrix row, manifests, architecture/feature documents, representative implementation seams and their tests. Existing pass records are evidence from earlier work, **not test runs performed by this research task**. No new full build, UI suite, audio soak, hardware test or OS verification was run here.

| Area | Source-backed behavior / recorded evidence | Partial or genuinely absent behavior | External evidence still needed |
| --- | --- | --- | --- |
| Engine and device | Immutable plan compilation, sample-accurate notes, bounded voice/audio-clip pools, routing graph PDC, streaming export, retirement queues. `crates/windfall-engine/src/{plan,state,processor,message,mixer,render,stems,device}.rs`; `tests/engine/{realtime,sequencing,mixer,rendering,stems,device}.rs` | Serial graph processing; no sidechain buses, manual PDC offset, general multi-output device routing or song subregion contract. `PlanTrack.edges` carries audible post-fader edges. | `docs/perf/soak-2026-10-06-wasapi-480.txt` records 600 seconds, zero xruns, **480 actual frames** despite 128 requested. This does not prove 128-frame operation, input latency or other devices/OSes. |
| Beat/project | Rack, notes/steps sharing a model, automatic mixer routing, sample preview, undo and `.windfall` JSON, sample copying and dated backups. `windfall-project/src/{model,document,lower,file}.rs`; `session/{files,library,autosave}.rs`; rack/browser tests | No project archive, numbered Save new version, reusable template/preset system. Four generated factory drum sources are not a loop/content library. Missing step graph, channel groups and advanced fill remain work. | Fresh-user native install/build-a-beat/save/reopen/export walkthrough. Browser/mock persistence alone does not establish desktop paths/dialogs. |
| Sampler | One-shot/gated tape playback, envelope/cuts, forward/ping-pong loops, reverse/trim/fractional interpolation, implicit release, undo/save/load and callback allocation guards. `docs/SAMPLER-LOOPS.md`; `engine/src/{plan,voice}.rs`; `project/src/model.rs` | `PlanSampler` holds one source and a key-dependent resampling offset. Independent duration/pitch preparation and sampler filters/LFOs are missing. | Listening review of loop seams/interpolation and prepared stretch on musical material; no hardware claim from loop tests. |
| Song/editors | Pattern/audio/automation clips, picking, draw/paint/delete/select/mute, tick offsets/fades/reverse, bends/holds, tempo map, GPU canvas/fallback. Piano draw/erase/select, velocity/pan and ghosts. Corresponding `features/{piano-roll,playlist,automation}` and engine tests | Note has only id/start/length/key/velocity/pan at base. PlaylistTrack has id/name/muted. No meter map, arrangements, track/color/solo/height/group associations, clip groups, event lanes, manual PDC or performance scheduler. Reserved tools may add some of these. | Windows WebView2 canvas measurements exist in `docs/perf/canvas-10k-notes.md`; macOS WKWebView/Linux WebKitGTK and native detached-window interactions remain unmeasured. |
| Stretch | `windfall-stretch` is already a **pure Rust implementation** using rustfft, not a vendored Signalsmith runtime. Playlist preparations share immutable buffers, stale guards and playback/export path. `engine/src/{clip_processing,pool}.rs`, `session/clip_processing.rs`, `stretch/VALIDATION.md` | Approximate formants, fixed fit-to-current-tempo ratio, bounded cache with an oversized-single-clip exception. A sampler cannot reuse one playlist variant for every key. No pitch/warp editing workflow. | Musical listening and target-machine performance; synthetic sine/impulse figures do not establish vocal transparency. |
| Record/MIDI | Recording callback writes frames to a preallocated queue; writer imports a kept WAV as one clip. Synthetic failures, undo and save tested. MIDI SMF converter/import review/export shared with WASM. `docs/RECORDING.md`, `windfall-midi/README.md`, `session/tests/{recording,midi,import_recording}.rs` | Input rate must match output. No clock/latency compensation, monitoring/count-in/multitrack takes at base. File MIDI is not hardware MIDI. MIDI owner’s integration must be reviewed separately. | Microphone/interface capture, duplex clock drift and latency loopback; MIDI hotplug, real device output/clock and controller profiles. |
| Plugin hosting | Windows CLAP desktop scan/add/native-range parameters/editors/state/automation/audio/export. VST3 backend processes real plugins, inactive state and Windows editors. Isolated scanner plus blocklist. `docs/plugins/{desktop-integration,vst3-hosting,host-evaluation}.md`, `plugin-host/tests`, `session/tests/{plugins,plugin_update,plugin_recording}.rs` | Desktop VST3 addition is intentionally gated at base. `containment.rs` scrubs bad samples/counts errors; **it cannot contain a crash or hang**. Runtime rejects non-Windows desktop use. Stereo-only engine adapter and no AU/bridge/modular rack. | Real compatibility corpus with versions/formats and native OS editors; device deadline runs. Synthetic fixture tests are essential but not substitutes. |
| Import/export | WAV/AIFF/FLAC/MP3/OGG decode; WAV/FLAC/OGG/MP3 encode, cancellation and mixer stems; reviewed MIDI/FLP workflows. `codec/tests/it`, `session/tests/{import_formats,export_formats,export,midi,flp}.rs` | Playlist stems/freeze missing; FLP sound mapping approximate, unsupported state retained, not restored; later meters/markers/arrangements/expression are dropped or reported. | `docs/flp/coverage.md` has two real FL 20.8.4 reader projects; historical/modern generated fixtures do not verify other real versions or matching sound. |
| Built-ins | Five effect kinds and one synth in `dsp/src/{effect,instrument}.rs`, shared blocks, measured tests, descriptors and generic editor fallback | No second synth family or remaining processor family exists merely because a descriptor editor can draw controls. ParamSet is Copy; large graphs/samples/tables need a separate immutable asset contract. | Independent signal measurements and listening per implemented behavior; licensed own samples/IRs/models before distribution. |
| Delivery | Tauri/React/Base UI/shadcn, GPL app/MIT audio kit/CC0 factory; three-OS strict CI definitions; portable binding comparison. `Cargo.toml`, desktop manifests, `.github/workflows/ci.yml`, `docs/ci-portability.md` | CI builds with `--no-bundle`; no release workflow, updater or crash-report transport in this snapshot. One main Tauri window; CSP is null. No localization architecture/tutorial workflow. | Signed installer/update on actual Windows/macOS/Linux, trusted signing identities and update hosting, OS UI/driver validation. Existing `v0.1.0-alpha.1` stays immutable. |

### Evidence reconciliation before declaring completion

The parent should reconcile these concrete mismatches without silently changing this document into a second matrix:

- `win-playlist-audio-clip-properties` still says independent stretch is absent, while `win-playlist-audio-stretch`, `ClipStretch` and the inspector establish its implementation. Normalize the note after integrated validation; normalization is still a separate missing behavior.
- `fx-fruity-parametric-eq2` is done, but its summary includes a live spectrum. `features/effects/eq/eq-display.tsx` computes the **filter response from parameters**; no audio FFT feed was found in engine/IPC/effect UI. T8 analyzer delivery must prove a live spectrum or the row must clearly state the chosen equivalent’s narrower scope.
- `fx-fruity-limiter` is done for the core lookahead limiter. `LimiterParams` has ceiling/input gain/release/lookahead and the editor has gain reduction, not the summary’s compressor/gate/scrolling history. Implement the extra behaviors through dynamics/analyzer infrastructure or explicitly review the equivalence criterion; do not credit them from the name.
- The core delay has sync/filter/saturation/stereo offset, but no modulation setting in `DelayParams`. Audit the modulation/degradation language of its row when implementing modulation effects.
- `win-mixer-tracks` is done while its note mentions 500 inserts and a current-track utility. `MAX_MIXER_TRACKS` is 128 including master and no current-track facility was found. T3 should explicitly test/raise the limit and implement the utility or present a scoped decision for review.
- Some done rows summarize several gestures, such as pattern split-by-channel or custom key labels. Use row-specific workflow tests to confirm every promised subbehavior before closing phase umbrellas; an existing panel is insufficient evidence.

These are accounting/acceptance findings, not source fixes in this survey. A row can map to one superior Windfall design, but every listed behavior still needs a test or an explicit scope decision. Identical proprietary sound is already outside the stated goal.

## 3. Contract for every implementation brief

Each worker receives this entire document or its brief plus this section, the accepted prerequisite hashes, the current parity rows, and explicit ownership. Start with `git status --short --branch`, `git rev-parse HEAD` and `git diff --stat c37ae0b9..HEAD -- <owned paths>`. Read changed seams before editing. If ownership overlaps an active worker, obtain an integration edit window from the parent; do not revert their work.

Path shorthand in this document is rooted at the repository: `project/`, `engine/`, `dsp/`, `plugin-host/`, `stretch/` and `codec/` mean `crates/windfall-<name>/`; `session/` means `apps/desktop/src-tauri/src/session/`; `features/` and `lib/` mean `apps/desktop/src/features/` and `apps/desktop/src/lib/`. New paths are explicitly proposals. Paths written as `windfall-project/src/...` similarly resolve under `crates/`. Test-directory module additions also require registration in that integration target's `main.rs`; adding a file without registering its tests is not verification.

**Realtime and ownership:** no callback allocations, frees, locks, waits or IO, including reset/parameter/note changes and failure paths. Build processors, FFT plans, samples, shared memory and routing plans away from audio/document locks. Keep heap objects alive through immutable plans and retire them through control-side queues. A Mutex in `SamplePool` is a control-side cache, never permission to access it from note-on. Preserve record-before-State acquisition for mutations, existing save/configuration ordering, generation/edit/request/source guards, and rejected-command atomicity. A stale worker result must leave project/history/pool unchanged. Every introduced bound needs observable overflow/failure behavior.

**Plugin ownership:** preserve provider identity/revision, selected playback token, exact binding fingerprint and separate render role. Speculative processors do not become state/editor owners. Missing bindings retain opaque state; effects pass through and instruments are silent with a reason. Capture native state off the document lock. VST3 cannot be enabled by skipping capture. Never introduce FFI destruction on the callback or make an editor request wait there.

**Persistence:** new optional fields have legacy serde defaults and tests loading old v1 projects; removed/reordered descriptor indices require explicit migration because automation stores indices. Non-Copy assets belong in immutable runtime storage, with persisted IDs/references, not huge parameter structs. All edits go through checked Rust commands with undo/redo and proper patch sections; browser mode uses the same Rust document. Keep session transport/view state separate where it is not musical data.

**UI:** follow existing shadcn/Base UI patterns, semantic theme tokens, `components/audio`, `features/params` and scoped `lib/actions` commands/menus. Use the shadcn skill for UI implementation. `effect-editor.tsx` already offers GenericParamEditor for a new real effect. Use that for small utilities; a table/curve/zone editor needs a purpose-built view. Native capability errors must be shown, not simulated as successful audio. Keep realtime displays outside React state and clear ID caches on project replacement. Future dialogs use existing Field/Alert/Empty components and accessible titles, not copied markup from another UI library.

**Verification environment and generation:** Windows Cargo runs in Git Bash after `source scripts/msvc-env.sh`. Put `C:/Program Files/nodejs` and `C:/Users/ewhee/AppData/Roaming/npm` on PATH; pnpm is `C:/Users/ewhee/AppData/Roaming/npm/pnpm.cmd`. Before ordinary tests, set `TS_RS_EXPORT_DIR` to a task-specific temporary native path. For example, in Git Bash:

```bash
source scripts/msvc-env.sh
task_bindings="$(mktemp -d)"
export TS_RS_EXPORT_DIR="$(cygpath -w "$task_bindings")"
cargo test -p windfall-project --lib --tests
```

Use the relevant package/test commands below, `cargo fmt --all --check`, package-scoped strict Clippy, and UI `typecheck`/`lint` when touched. Every listed command must exit 0 with all applicable assertions passing. Newly specified test names/files are delivery requirements, not tests claimed to exist today. UI test workers are bounded: `pnpm --dir apps/desktop test <paths> --maxWorkers=4`. Mock-backed UI tests for changed Rust require fresh local artifacts, even when the old checked-in WASM still passes unrelated tests.

Workers may run `scripts/gen-bindings.sh <task-output-directory>` and locally regenerate via `scripts/build-sim.sh` for validation; never hand-write generated TS. Exclude `src/bindings/*`, `windfall_sim.wasm` and its metadata from worker commits. Parent integrates source, runs the generators, reviews the complete binding inventory with `scripts/check-bindings.mjs`, and checks `node scripts/check-sim.mjs`. DSP changes also affect descriptors and the WASM dependency tree. No installs/dependency upgrades solely to refresh the scaffold.

**Commit/publication:** only owned source/docs and meaningful tests in feature commits, exact SHA in handoff, no push/PR/tag/release/visibility change without parent scope. Keep the old alpha tag and release assets unchanged. Report actual checks, intentional ignores, native/browser distinction, external requirements and the remaining rows.

## 4. Four briefs for the next implementation batch

### N1 — Independent sampler duration and pitch with prepared key variants

**Priority P1; effort L; risk high; confidence high.** Prerequisites: `c37ae0b9` loops, accepted A-EDITOR/A-SLICE/A-MIDI/A-PIANO model/audio seams, and integration’s regenerated mock. Completion targets `win-rack-sampler-stretch`; `inst-channel-sampler` still waits for filters/envelopes. This assignment does not reimplement playlist stretch, slicing or audio editing.

**Current seam:** `SamplerSettings` stores trim/root/tune/loops; `PlanSampler` stores one `AudioBuffer` plus `key_offset`. `voice.rs` resamples it at note-key speed. `ClipAudioCache` retains 32 variants/256 MiB with an oversized exception, and `session/clip_processing.rs::prepare_clip_command` snapshots, renders unlocked, then checks generation/edits/source. That policy alone cannot support every live key, overlapping old voices or all sampler loops.

**Exclusive ownership:** new `crates/windfall-engine/src/sampler_processing.rs`; sampler portions of `plan.rs`, `voice.rs`, `pool.rs`; new engine sampler-preparation tests; new `session/sampler_processing.rs`; sampler inspector stretch controls/tests. Narrow integration edits: SamplerSettings/SamplerPatch/check/lowering, controller preparation, module declarations, new IPC preparation operation, backend adapters and inspector composition. Do not change plugin runtime, playlist processing policy, editor/slicer modules or MIDI-device code.

1. Define persisted tape-default/spectral settings and a complete key-variant policy before enabling controls. Start with an explicit prepared key range (default one playable octave around root, expandable up to the whole MIDI range subject to a strict aggregate byte budget). Pre-render every supported key before publication so live input and sequenced playback behave alike. Expose preparation/range/budget failures. A key outside the published range must be reported and silent, never secretly tape-resampled or rendered on note-on. Save the musical mode/range; derive caches at load.
2. Snapshot source identity/settings/document tickets, prepare trimmed source variants using existing stretch DSP off-lock, and map loop bounds in the rendered domain. Choose and document lead-in/release semantics; preserve existing forward/ping-pong/no-outro rules. Publish an indexed immutable table and prebuilt plan only after final recording/generation/edit/source checks. Budget refusals keep the old mode/plan unchanged. Do not adopt the playlist oversized exception for a bank of 128 keys.
3. Extend voices to select a prepared buffer in constant bounded work. Old voices retain old variants through source reload, changed key range and cache eviction. Playback and export use the same preparation policy and requested range; offline rendering may not silently sound notes that realtime would reject. Keep tape mode bit-compatible with base loops.
4. Add inspector mode/ratio/quality/formants/range/progress controls using ParamControl/audio-kit patterns and one checked undo step per committed preparation. Capability handling in mock preserves settings and history without claiming DSP. Open/undo/redo must also prepare target snapshots off-lock.

**Acceptance:** new tests in `engine/tests/engine/sampler_processing.rs` cover a 440 Hz source at ±12 semitones with invariant intended duration, ratios 0.5/1/2 at unchanged pitch, deterministic bit equality across block partitions/playback/export, empty/missing sources, reverse/trim, one-frame and fractional forward/ping-pong loops, release and cut/steal behavior. Measure frequency using an independent test estimator with the stretch suite’s sine bound (0.1 cent); explicitly document any transient/formant quality limits. Exercise first note, all range edges, unsupported keys, repeated settings, 320 notes, reload, eviction and retirement under an allocator guard: zero alloc/realloc/free calls. Test budget calculation including all retained/prepared variants, bounded cancellation, source-pointer reuse and stale apply. Project tests cover legacy omission/defaults, invalid ranges, undo/redo/clone/save/load. Session barrier tests mirror `session/tests/clip_recording.rs`, `import_recording.rs` and existing clip-preparation stale tests. UI tests select different channels during pending work, cancel/apply/undo and show unavailable mock DSP honestly.

**Commands:** `cargo test -p windfall-project --lib --tests`; `cargo test -p windfall-engine --test engine sampler`; `cargo test -p windfall-desktop --lib session::tests::sampler_processing` (new module); `cargo clippy -p windfall-project -p windfall-engine -p windfall-desktop --all-targets -- -D warnings`; UI sampler inspector/preparation tests plus typecheck/lint. All pass, no generated artifacts in commit. Stop and return a design issue if the chosen range needs unbounded memory or preparation under a document lock; do not ship a lazy callback cache. Real musical listening is a follow-up evidence gate, not grounds to discard the engineering requirement.

### N2 — Seven measured utility effects through the existing descriptor interface

**Priority P1; effort M; risk medium; confidence high.** Prerequisites: base DSP traits plus integrated A-VST3 so exhaustive binding/editor matches are reconciled. Targets: `fx-fruity-balance`, `fx-fruity-center`, `fx-fruity-mute-2`, `fx-fruity-phase-inverter`, `fx-fruity-stereo-shaper`, `fx-fruity-soft-clipper`, `fx-fruity-fast-dist`. Windfall uses its own descriptive names.

**Current seam:** `Effect` has prepare/reset/set_params/process and latency/tail/gap; AnyEffect/EffectParams are explicit five-kind unions. Shared `blocks/{dc,shaper,smooth,delay_line,math}.rs` already provide usable foundations. New small processors can use `ParamSet` and the existing GenericParamEditor; existing routed sends or a master fader do not satisfy independent chain-position utilities.

**Exclusive ownership:** new `dsp/src/{balance,dc_block,channel_mute,polarity,stereo_matrix,soft_clipper,distortion}.rs`; new `dsp/tests/dsp/utilities.rs`; `dsp/src/{effect,lib}.rs` and corresponding exhaustive test helpers/descriptors example; utility-specific UI tests and optional concise groups in `features/params/groups.ts`. Narrow caller exhaustiveness updates require an integration edit window. No sampler preparation, plugin bridge/runtime, native files, or new synth families in this brief.

1. Specify real, independently testable transfer behavior: smoothed gain/pan; DC blocking with a cutoff; independently selectable L/R/both mute and polarity; a 2×2 stereo/mid-side matrix with independently bounded channel delay; soft-knee clipping; a drive/shape distortion with oversampling/anti-alias handling or an explicitly measured quality bound. Neutral matrix/gain/mute settings must be unity and utility delay reported for PDC. DC blocking is inherently a filter, so its default is not described as bit-unity.
2. Implement processors, sanitized Copy parameters, append-only descriptors, aliases only for documented routing views of the **same tested matrix**, and union dispatch. Distortion and soft clipping must have distinct transfer/quality behavior; changing names around one tanh is insufficient.
3. Add rack selection/editor/automation coverage using generated descriptors and GenericParamEditor. Existing engine construction should discover the new kinds; update necessary exhaustive matches explicitly. Do not add fake plugin bindings or require a native plugin download.

**Acceptance:** impulse identity/latency; DC rejection and transient response at 44.1/48/96 kHz; signed L/R polarity and mute truth tables; matrix equations and mid/side round-trip; mono cancellation/width boundaries; settled soft-knee transfer/ceiling and symmetry; oversampled distortion alias measurements relative to a high-rate reference. Bound output for NaN/Inf/denormal/extreme controls. Bit equality across irregular block partitions. Rapid automation/reset/bypass/mix and every new effect in the existing DSP allocator suite observe no callback allocation/free. Engine test chains each effect before/after a limiter and verifies automatic PDC, moving/removing effect tails/state, automation and render parity. Rust document/WASM tests add/edit/undo/save/reopen each kind; UI controls produce one gesture history step and registry/menu discoverability.

**Commands:** `cargo test -p windfall-dsp --test dsp`; `cargo test -p windfall-project --test commands`; `cargo test -p windfall-engine --test engine effects`; `cargo clippy -p windfall-dsp -p windfall-project -p windfall-engine --all-targets -- -D warnings`; `pnpm --dir apps/desktop test src/features/effects src/features/params src/features/mixer/effects.test.tsx --maxWorkers=4`, typecheck/lint. Use fresh local descriptors/WASM, excluded from commit. No new dependency is expected. Stop for coordinator review if descriptor reordering or stereo-matrix delay requires a new host contract. Later stereo enhancement, modulation, filtering and mastering families remain distinct work.

### N3 — Portable project archive and collision-safe numbered saves

**Priority P1; effort M/L; risk medium; confidence high.** Prerequisites: accepted A-EDITOR/A-SLICE source attachment and A-VST3 capture semantics. Targets `fmt-project-zip` and `fmt-save-new-version`; foundations for templates/presets/cloud, without duplicating A-LIBRARY’s browser.

**Current seam:** `session/files.rs::project_save` serializes saves, clones under State, captures plugin state unlocked, carries/relinks samples and marks saved only for the correct generation/edit count. `file.rs` supplies `.windfall` JSON, path resolution and atomic writes. Opening uses a replacement ticket and `Session::install`, including staged plugin-factory revisions. A zip writer alone would bypass these guarantees.

**Exclusive ownership:** new `crates/windfall-project/src/archive.rs` (filesystem feature or a separate `windfall-archive` crate if it would pollute the WASM dependency tree), archive tests, new `session/archive.rs`, new `session/versions.rs`, file-flow UI tests. Integration-only edits to module declarations/Cargo workspace, file dialogs/backend/command registration and `files.rs` helpers. Do not edit plugin capture internals, browser index, DSP or sample editor.

1. Define a versioned archive manifest containing ordinary `.windfall` project/session JSON and deduplicated referenced audio. Include external/recorded/edited/factory sources needed for portability, preserving opaque plugin states and original source names as metadata. Missing sources produce a reviewable list and explicit policy; never silently call an incomplete package portable. Native plugin binaries are not bundled.
2. Package a captured immutable snapshot off document/audio locks, with bounded entries/expanded bytes/path lengths, streamed IO, cancellation and a staging target atomically published on success. Do not rewrite the live project’s paths/history just to create an archive. Own only staging files, never delete unrelated targets on rollback.
3. Open into a managed extraction destination using the ordinary reviewed replacement workflow. Reject absolute/traversal/UNC/drive paths, symlink entries, duplicate/case-colliding entries, zip bombs and unsupported schema before installing anything. Resolve `SamplePath::Project` under the extracted root, and clean only this request’s abandoned extraction. Active plugin state is captured by the existing owner path; saved state for missing plugins survives.
4. Save new version reserves an unused numbered filename without overwriting a competing save, delegates to the existing safe save/carry-sample flow, updates current path on success, and does not mark concurrent edits clean. Define padding and handling of an already numbered stem; preserve all older numbered files. New File actions use existing registry and cancellation/error UX.

**Acceptance:** headless session round-trip archive with factory/external/project/recorded edited sources, two equal basenames, non-ASCII/spaces and relative nested paths; source removal after packaging still permits correct playback from archive. Missing plugins retain state and parameters. Malformed/traversal/case/symlink/size-limit archives leave document/destination unchanged. Inject write/rename/read failures and cancel at each stage. Barrier tests replace/edit project, start recording, reload a sample or race two numbered saves while work is pending; no stale install, overwritten version, or dirty-state loss. Test native plugin capture/export round-trip after A-VST3. Mock UI tests cover native-only archive capability, dialog cancellation/errors and new-version semantics without inventing a filesystem.

**Commands:** `cargo test -p windfall-project --test archive` (new, or equivalent new crate target); `cargo test -p windfall-desktop --lib session::tests::archive`; `cargo test -p windfall-desktop --lib session::tests::versions`; existing `session::tests::files` and plugin-save tests; scoped Clippy; new `lib/flows` file-action tests with `--maxWorkers=4`, typecheck/lint. Dependency strategy: prefer a pinned Rust archive implementation with only ZIP/deflate features; inspect the exact crate/transitive license files before selecting, record notices and feature bounds. Do not pull an archive runtime into `windfall-sim` unnecessarily or invoke a user-installed zip utility. Stop if safe extraction/publication requires expanding beyond owned file lifecycle; coordinate the design before changing it.

### N4 — First audio-plugin process bridge, with bounded failure behavior

**Priority P1; effort XL; risk high; confidence high.** Prerequisites: accepted A-VST3 capture/current-owner tests and stable plugin factory, but transport/fixture design can proceed in a new module while that integration lands. First delivery is Windows CLAP/VST3 **audio and state** isolation with visible editor limitations; the full bridge row remains partial until native editor/lifecycle parity and other supported OSes are verified.

**Current seam:** `PluginFactory` creates independent HostedEffect/HostedInstrument instances and has render_factory/provider_identity/revision. HostEvent/Transport are pointer-free values, but Rust enum layout is not a cross-process ABI. `plugin-host/src/containment.rs` explicitly says in-process crash/hang cannot be stopped. The existing scanner protocol is discovery IO, not a realtime audio transport.

**Exclusive ownership:** new `plugin-host/src/bridge/*`, new audio-helper binary mode, bridge tests/fixture behaviors in `plugin-host/test-plugins`, new desktop `plugins/bridge.rs`. Narrow factory/manager/helper-mode wiring after A-VST3 under an exclusive window. Do not rewrite A-VST3’s runtime or engine graph; keep `HostedEffect` facade so engine DSP tasks need no bridge knowledge. Any changed persisted wrapper options are a separately assigned shared-model patch. No AU or multithreading in this first delivery.

1. Specify a versioned explicit wire/shared-memory ABI: fixed capacities, little-endian fields, sequence/owner/generation tokens, negotiated rate/block/latency, checked parameter/note/transport data, no Rust pointers/Vec layouts. Separate control messages (load/state/gui/restart) from the audio rings. Validate offsets/sizes before native loads. Own helper lifecycle/resources off the audio thread.
2. Use a fixed extra-block pipeline with preallocated shared slots: callback submits one block and reads only a completed matching earlier sequence. It **never waits for the helper**. Missing/late/corrupt output takes a defined latency-matched dry path for effects or silence for instruments, with click-free transitions and telemetry. Select pipeline block size independently of irregular host callbacks and test adapter buffering. Count extra latency in graph PDC and dry/wet paths. No assumption that a plugin finishes before the callback deadline.
3. Worker supervises process death/hangs and finite retry/blocklist behavior. Killing/restarting an instance must leave other plugins and unsaved document alive. Retain last valid state and owner identity; stale process replies cannot update a reused numeric target. Offline render uses independent helpers and may wait **off realtime**, with cancellation/deadlines; report that streaming and offline paths differ in scheduling while testing identical healthy DSP output after latency alignment.
4. Capture and restore real native state through the accepted A-VST3/CLAP paths. Never send a native pointer across processes or capture a speculative/render owner. Expose bridge selection/health/retry through plugin manager and explicit unsupported editor states. Follow up with native helper/editor window parenting/scaling/lifecycle before calling the full bridge/editor wrapper rows done.

**Acceptance:** synthetic CLAP/VST3 fixtures crash, hang, emit NaN/Inf, saturate event queues, report bad latency, write malformed state, exit while capturing, and deliver stale sequence/token/revision responses. Parent remains alive, dry/silent fallback is bounded, no recording/doc mutation occurs, resources are reclaimed by supervisor. Zero callback alloc/free/lock/IO/wait across startup, reset, late output, crash and removal; measure maximum callback time separately from average CPU. Test variable 1/7/64/480/512-frame callbacks, PDC, routing, automation, seeks/loops, independent export/stem helpers and save/undo/replacement races. Healthy bridge audio after documented latency matches in-process reference at deterministic fixture precision. Test helper path discovery in a **packaged installer**, not just Cargo target paths. Then use the existing licensed Surge/OB-Xf corpus on actual Windows to record formats/versions/settings/output, with no macOS/Linux claim.

**Commands:** `cargo test -p windfall-plugin-host --test bridge` (new process integration target), `cargo test -p windfall-plugin-host --test realtime`; `cargo test -p windfall-desktop --lib session::tests::plugin`; `cargo test -p windfall-engine --test engine effects`; host/desktop strict Clippy; plugin manager/control UI tests with four workers, typecheck/lint. Use existing rtrb/serde and platform bindings where possible; any new shared-memory dependency needs exact-version license review and a portable implementation plan. Stop for design review if a callback wait or in-process fallback is proposed as crash isolation. Separate process is crash containment, not a security sandbox, and that distinction must remain visible.

## 5. Shared infrastructure and native workflows after N1–N4

These are executable portfolio boundaries. Dispatch one packet at a time per shared seam; expand its test files and actual prerequisite hashes in the brief. Every packet inherits section 3 and the exact verification commands of its owning crates/UI paths. Rows assigned in the inventory identify the complete scope, including umbrella rows; a packet’s first subset does not close all of them.

### T0 — Rack, sample preparation and everyday transport tools

After A-PIANO/A-EDITOR/N1, own rack-specific UI/commands and narrowly scoped sampler/note-timing DSP. Implement step graph editing of the actual supported note properties, channel groups/filter, per-channel swing mix/gate/shift, preview thumbnails and the steps-to-piano workflow (steps already are notes; the action opens/selects the right lane rather than creating duplicates). Advanced fill needs rhythm rules, preview and checked one-step Apply. Sampler preprocessing reuses the accepted editor job/asset attachment contract for normalization, reverse and loop seam crossfades; do not treat reversible playback flags as destructive processed output. Add sampler filter and pan/pitch/modulation envelopes/LFOs through T2's bounded control contract, arpeggio/note echo/polyphony/glide as distinct note/voice processing, and automatic clip crossfades/normalize with explicit per-instance versus source semantics. Audio properties UI must retain independent playlist stretch already present at base.

Tempo tap uses a bounded timestamp history with reset/outlier policy and one command; master toolbar memory must measure a defined native process value, not a random mock. Tools-menu macros operate on checked selections and one undo batch, preserving plugin states and automation deletion warnings. Tests cover graph versus steps/notes identity, groups/clone/undo, seeded fills, timing at bounds/tempo changes, sample normalization/loop seam rendering, stale processed-source attachment, typed/tapped tempo and memory errors. Run project commands/file tests, engine sampler/sequencing/realtime, session editor-preparation tests, rack/transport/palette UI tests and typecheck/lint. Retain useful empty states and action registry discoverability. Typing keyboard/metronome/count-in belong with T4's live clock/note pipeline.

### T1 — Musical meter map, markers, playback/export regions and arrangements

Own new `project/src/timeline.rs`, checked model/command edits, `engine/src/{sequencer,tempo,render}.rs`, playlist rulers/region controls and IPC transport/export options after A-SLICE. Current ProjectSettings has one TimeSignature; Playlist has one tracks/clips list and transport loops the whole song. Persist ordered meter/marker/arrangement data with stable identities; distinguish musical tick position from bars/beats and seconds. Build pure tick↔bar conversion first, then named/loop/skip/pause markers, selection loop/export range, track/clip groups and make-unique operations, alternative arrangements and linked instrument/audio tracks. Group hierarchy must reject cycles and preserve deletion/undo. Gate core-time-signature-changes and per-pattern signatures on actual map support, including MIDI/FLP conversion improvements.

Complete playlist region-zoom and playback/scrub tools against the shared canvas viewport and native transport: zoom fits the dragged bounds, scrubbing follows tempo/PDC and releases auditions on cancellation/focus loss. This work follows the slicer's accepted pointer/selection semantics and does not replace its split/slip implementation.

Tests: changes 4/4→7/8→3/4 at nonzero ticks with tempo ramps, seek/loop/end inside clipped sources, unaligned markers, range-boundary tails/PDC, all clip kinds/offsets, cancellation and exact undo; legacy files still play unchanged. Shared Rust meter fixtures drive UI ruler labels. Commands: project tests, engine `sequencing`, `rendering`, `audio_clips`, `automation`; playlist/`lib/time`/tempo tests, typecheck/lint. Named skip/pause must affect real transport, not just labels. Separate basic region delivery from full arrangements to keep review bounded.

### T2 — Identified note events, modulation/control ports and sidechain buses

Own a versioned event/port contract in `dsp/src/{instrument,effect}.rs`, `engine/src/{plan,rack,plugins,automation}.rs` and project control links after A-MIDI/A-PIANO/N4. At base Instrument is key/velocity based, key releases cannot identify overlapping notes, HostedEffect accepts only two channels, and automation targets are scalar parameters. Preserve legacy key-note behavior behind adapters while adding note identity/channel/expression and explicit audio/note/control port descriptions. Compile topologically ordered, bounded note transforms/control sources; detect cycles and define feedback only via explicit delayed nodes. Sidechain audio must not reach the audible sum and must have independently aligned latency. Control formulas use a bounded expression evaluator, not arbitrary code on audio. Parameters remain stable by IDs/descriptor mapping.

Tests: two overlapping same-key notes, sustain/voice stealing/slide/glide/color routing, transposition/chord/split transforms, deterministic event order and overflow; sidechain compressor ducking with silent detector bus, auxiliary bus latency/PDC; controller envelope/peak/formula smoothing and cycles; deletion/replacement unlinks atomically with undo. Host fixtures test real auxiliary input negotiation, not a renamed stereo input. Run DSP/engine/project/plugin-host targeted event, automation and realtime suites plus controls UI tests. This contract unlocks many long-tail rows; it is not completed by adding enum tags alone.

### T3 — Complete mixer signal/workflow model

After T2 and N2, own mixer-specific project commands, engine graph/utilities and `features/mixer/*` in a separate dispatch. Deliver actual sidechain routing, integrated post-slot three-band EQ/utilities, manual PDC offsets, track states, multi-selection, docks/layouts, waveform view and current-track utility. Explicitly resolve 128-versus-500 track capacity through memory/CPU measurements, fixed meter/retirement bounds and tests; do not just increase a constant. Manual offsets need defined negative/positive semantics, maximums and export alignment. External audio input/output and track arms depend on T4 device/capture routing.

Tests: sparse 500-track graph if adopted, cycles/refused routing, changing PDC with bypass/removed/moved effects, sample-aligned bus/sidechain impulses, grouped editing undo, presets containing missing native plugins, UI focus/menus at compact sizes. Run engine `mixer/effects/stems/realtime`, project command/file tests and mixer UI suites. Existing automatic PDC and sends remain regression requirements.

### T4 — Native recording, monitoring, metronome and takes

After A-MIDI plus T1/T2, own engine device/recording clock/routing and `session/recording.rs` plus recording transport/mixer UI; do not duplicate the first MIDI device implementation. Add a sample-clock timestamp contract, measured driver input/output latency with explicit manual calibration, bounded drift correction/resampling for independent devices, safe input monitoring, count-in/metronome aligned to tempo/meter map, multitrack arms/output ports, loop takes and automation/score/audio logger buffers. Loggers are bounded ring histories and flush through workers; no continuous IO in callbacks. Retain clear feedback prevention, channel mapping and whole-take error ownership. Typing-keyboard and step entry can use the accepted live note API.

Tests now: injected clocks/drift/latency, resampling frame counts, 7/8 count-in, loop split/final partial take, automation pickup/multilink logs, queue overflow/device loss/cancel/stale stop. Run engine recording/device/realtime and session recording/plugin-recording suites. External gate: actual mic/interface loopback measuring alignment/drift, monitor feedback/latency listening, MIDI device clock jitter/hotplug. A synthetic clock cannot close the hardware verification column. Discrete logger/count-in/take subdeliveries are engineering work, not reasons to mark rows won't-do.

### T5 — Presets, templates, scores and project metadata

After N3/A-LIBRARY and accepted native capture, own new `project/src/preset.rs`, `session/presets.rs`, project-info/settings and preset UI, with shared commands coordinated. Use Windfall formats, not FL bundled presets. Define schema/kind/version/asset manifests for channel, plugin, mixer track and modular rack states; preserve unsupported plugin state and relative audio, remap IDs/controller links/automations transactionally. Score files can use native note/event schema and SMF interoperability rather than pretending to parse `.fsc` already. User templates create fresh identity/path/history. Add author/genre/comments and supported export metadata. Global snap and panning law/timebase choices require explicit playback/import/export semantics; 960 PPQ is the current base, not a setting that can change by relabeling.

Tests: save/open/apply each preset, missing assets/plugins, stale native capture, renamed effect parameter migrations, duplicate IDs and incompatible newer schemas, immutable factory templates and clean undo; author tags independently decoded from exported files. Project/session tests plus `features/browser`, preset dialogs and action registry. A-LIBRARY owns catalog display; this packet owns file semantics.

### T6 — Playlist stems, consolidation/freeze and destructive-free render jobs

After T1/A-SLICE/A-EDITOR, own new engine playlist-render selectors and `session/consolidate.rs`, export selection and consolidate UI. Current stems select mixer TrackId; playlist filtering requires different musical-source semantics. Define whether a stem includes shared bus/master processing and how nonlinear/detector inputs are handled. Freeze renders a reviewed snapshot to owned audio, replaces/mutes selected sources as one command, preserves originals for unfreeze/undo, and never loads/replaces a live native owner with an export instance.

Tests: offset/loop/automation/range fades, shared patterns/channels across lanes, sidechain and nonlinear bus behavior, PDC/tails, failure/cancel/disk cleanup, stale review and reloaded source. Run engine `stems/rendering/audio_clips`, session `export` plus new consolidation tests, export/playlist UI. `win-mixer-render-tracks` also needs its selection workflow; do not credit it solely because mixer stems already exist.

### T7 — Modular rack, layer channels and reusable note/control modules

After T2/N4/T5, own new `project/src/rack.rs`, `engine/src/modular.rs`, `features/modular/*` and graph tests. Existing mixer routing is not an instrument/effect/note/control modular rack. Store node/port identities, assets, macros and checked edges; compile immutable graph and latency outside audio. Nested graphs require bounded depth/node/event count, cycle rules and stable parameter mapping. Embed native/built-in processors using their real lifecycle; save rack preset/state through existing capture roles. Layer channels fan out note events without duplicating ghost audio channels. The mobile-rack equivalent uses Windfall modules, not mobile-app code.

Tests: synth→FX graph, parallel paths/PDC, note splitter/color mapper/envelope/level/key map/sequencer, nested rack preset round-trip, atomic cycle refusal, mute/solo/automation, removed target ownership, max graph and allocator guard. Run project/DSP/engine graph suites, plugin session capture tests and modular browser/native UI flows. A graph editor with inert wires closes no processor row.

### T8 — Analyzer taps and reusable visual data

Own new bounded engine tap queues/atomics and analysis workers plus `features/analyzers/*`; narrow meter IPC additions are coordinated. Feed clock/meter/spectrum/spectrogram/oscilloscope/vectorscope/waveform/scrolling gain-reduction from real audio with fixed callback copying or decimated data, FFT on a worker, bounded drop counts and explicit selected source. Reuse renderer/theme/realtime subscription infrastructure. This also supplies the EQ live spectrum and mixer waveform view. GPU work and screenshots never run in the audio callback.

Tests: calibrated sine/RMS/peak, impulse/time history, FFT bin/sweep/DC/noise, left/right phase vector, queued overrun with UI stalled, latency labels and project-replacement cleanup. Compare computed magnitudes with an independent FFT/reference; no spectrum inferred from filter settings. Engine realtime tests plus analyzer/eq/mixer UI tests and a native source-selection smoke test. Big clock/large meter can share the actual data path; their distinct views still need interaction tests.

### F1 — FL project fidelity and public fixture corpus

Own `crates/windfall-flp/src/convert/*`, reader/report tests, `session/flp.rs` and import review tests after the related T1/T2/T3/new processor schemas land. At base the reader preserves markers/arrangements but conversion reports their loss; expression, unsupported controllers and native sound states are retained/reported rather than implemented. Extend conversion only for genuinely supported targets: meters/markers/arrangements, note expression, sampler loop/stretch mappings, controller/sidechain routes and newly implemented effects/instruments. Keep exact-versus-approximate-versus-omitted counts honest; preserve every unsupported opaque state under existing bounds. Third-party plugin state restoration requires a public understood wrapper/state decoder and correct host binding, not loading retained bytes blindly.

Add an ethically sourced, license/provenance-tracked real-project corpus by FL version, with small projects whose notes/clip offsets/routing/automation/meter and sample references can be independently inspected. Public manual/open parser evidence and user-authorized files are permissible; no reverse engineering/decompilation of proprietary binaries, content redistribution or invented acoustic equivalence. Existing two FL 20.8.4 reader files remain the verified real-version limit until new runs exist. Tests include hostile/oversized events, unknown versions, conflicting offset units, exact ticks and documented approximation; native/browser review cancellation/unsaved/replacement/source guards remain. Commands: `cargo test -p windfall-flp`, strict crate Clippy, `cargo test -p windfall-desktop --lib session::tests::flp`, FLP import/shared-WASM tests with four UI workers. Private copyrighted projects can be external evidence with sanitized metadata rather than committed fixtures. Missing corpus access is an external gate, not a reason to drop import engineering.

## 6. Built-in instrument/effect delivery families

Every family needs: real DSP/event behavior; persistence/default/migration tests; stable descriptors/macros; control automation; healthy output/block invariance/allocator tests; shared realtime/offline processing; useful native UI; own licensed example content; and documented limits. Small processors use existing interfaces directly. Complex processors first add immutable assets/port contracts; `ParamSet: Copy` is not a place to embed sample maps, arbitrary envelopes or FFT tables. Third-party hosting offers users a temporary option but never counts as a Windfall built-in equivalent.

The row inventory below assigns **all 41 instrument and 80 effect rows** to these families or the existing baseline. FL names are plain documentation references; new product names and content must be Windfall’s own. Each family lists distinct behaviors needed before sharing can justify multiple row closures.

| Family | Infrastructure, ownership and order | Distinct behavior and acceptance |
| --- | --- | --- |
| I1 Analog/subtractive and macro hybrids | Existing `dsp/src/synth.rs`, oscillator/SVF/envelope/unison blocks; new analog/hybrid modules, instrument-union owner, `features/channel-rack/synth`. After T2/T5. Covers compact analog, expanded analog, acid bass, groove synth and macro instruments. | Classical analog oscillator/filter/voice behavior, acid accents/slides and internal sequencer, lightweight mobile controls, three-oscillator architecture, hybrid wavetable/FM/sample modes and macro/preset browsing. A different subtractive preset alone cannot establish Kepler/Exo/Sawer/Poizone/SimSynth/MiniSynth/GMS/FLEX equivalents. Test tuning/aliasing/filter sweeps/envelopes/glide/unison/tempo; macro modes each exercise their actual synthesis path. Own packs, no proprietary FLEX content. |
| I2 FM/ring/modulation matrix | New `dsp/src/fm/*`, graph/patch assets, shared engine instrument hosting and matrix editor. After T2 and asset contract. Evaluate pinned Dexed engine extraction versus new bounded FM engine. | Light FM, deeper FM+ring/subtractive matrix and hybrid sequencer require different operators/routes/features; operator algorithms/feedback phase, key scaling, release, pitch behavior and patch mapping measured. Do not claim Sytrus from a DX-style preset loader. Native UI must edit the matrix and modulation, not only offer preset names. |
| I3 Additive/resynthesis/image | New additive spectral assets/worker prep and additive DSP; after M1 asset jobs/T2. Shared STFT/FFT from stretch where suitable. | Harmonic snapshots/morph, metallic partials, subtractive-style harmonic controls, sample/image resynthesis and image scan synthesis, deterministic number-derived patches. Harmor/Morphine/Ogun/Harmless/Autogun/BeepMap each needs its own observable synthesis/editing contract. Test partial spectra/morph interpolation/image mapping/reconstruction/error bounds and phase/CPU ceilings; worker loads tables, callback uses prepared partial arrays. |
| I4 Physical/acoustic/drum synthesis | New `dsp/src/physical/*` and `drums/*`, one instrument registry owner. Existing oscillator/noise/filter/delay blocks; own small excitation content. | Drum membrane/body model with both single and 16-pad workflows; kick pitch envelope plus optional sample layer; per-key drum patches; plucked/string resonator; acoustic string control and bass-guitar tonal response. Karplus–Strong helps plucked strings but does not prove every modeled percussion/string row. Test decay/tuning/damping/strike position, stable feedback at all controls, deterministic seeded noise and bounded polyphony. |
| I5 Multisample, SoundFont, layered pads and piano | New sample-zone/asset loader and engines; after N1/T2/T5 and A-SLICE. Keep native SoundFont parsing separate from WASM if needed. | Zone/velocity/key maps, layering/round-robin/chokes, editable full sampler versus player, SoundFont bank/preset semantics, layered drum-pad workflow and own piano/keyboard sources. Test crossfade/key/velocity boundaries, missing zones, voice/asset retirement and actual SF2 interoperability. A single sampler with a keyboard graphic is insufficient. FluidSynth or another parser/engine remains a candidate pending exact LGPL terms/transitives/content review. |
| I6 Slices, grains and turntable playback | Extend accepted A-SLICE APIs only after its commit; new granular/scratching modules, slice map/zone UI. | Basic slice trigger, advanced per-slice edit/two-deck-style workflow as required by row summary, transient beat detection, grain cloud controls, curve-driven scratch playback. Share original source/markers and interpolation but test granular scheduling and drawn playback curves independently. Slicex is not done when phase-3 slicing alone works. Test marker boundaries/reverse/overlaps/tempo, seeded grains and seek/release/export consistency. |
| I7 Speech and special hardware instruments | New worker speech rendering and peripheral adapters after M1/A-MIDI; never put text synthesis or gamepad IO on audio. | Own voice/text rendering with pitch/duration and saved output; configurable dashboard internal/MIDI controls; gamepad force-feedback events synchronized off callback. Test text errors/cancel/source attach and mock peripheral protocol, then actual Windows hardware/driver. Platform-specific rows remain possible engineering work pending SDK/content terms. |
| E1 Utility/filters/EQ | N2 then new filter/EQ modules and analyzer T8; current biquad/SVF/DC/smoothers are reusable. | Chain-position balance/mute/polarity/DC/matrix, stereo width/phase delay/balance, graphic fixed-band EQ versus freely placed EQ versus snapshot-morph EQ, resonant automated filter, low-CPU lowpass, selectable LP/BP/HP/notch/shelves and bass-boost shelving. Measure frequency response and CPU; a neutral filter preset does not establish several distinct EQ interfaces. Existing EQ remains baseline; live-spectrum acceptance is explicit. |
| E2 Dynamics/transient/mastering | New crossover/detector/transfer blocks and DSP, after T2 sidechain and T8 meters. Existing compressor/limiter provide single-band foundations. | True multiband compression/limiting with recombining crossover, envelope/transient attack-sustain shaping/split outputs, multistage limiter/saturator, two-stage simplification, one-knob macro enhancer and bass harmonic generation. Shared Maximus/Soundgoodizer-like engine is acceptable only if both editable multiband and intentionally constrained macro workflows work. Test independent band detectors, sum/phase, intersample peaks where promised, distortion/pumping/latency/tails and calibrated loudness. |
| E3 Delay/modulation/reverb | Existing delay lines/LFO/reverb; new modulated delay/allpass/FDN modules, after N2/T8. | Basic echo incl. inverted/ping-pong; stereo echo/offset; serial/parallel delay bank; 16 independently controlled frequency delays; chorus (basic/vintage/many-voice), flanger/stacked flanger, phaser/vintage phaser; legacy room reverb and expanded pitch-modulated spaces. Each vintage/model claim needs a public behavior specification and measurements, not branded presets. Test impulse timing/decay, frequency notches, modulation rate/depth/tempo, stable feedback, block and bypass continuity. |
| E4 Nonlinear/guitar/lo-fi | N2 shapers/oversampling, new drive/quantization/cabinet modules and rack presets after T7. | Overdrive, fast distortion, soft clipping, drawn waveshaping, bit/sample reduction+filter, modular distortion chain and eleven actual guitar pedal/cabinet behaviors. CPU/aliasing/headroom and cabinet IR provenance measured. A rack containing eleven copies of the same distortion is not Hardcore-equivalent. FIR/IR assets prepared off-lock. |
| E5 Spectral/convolution/pitch/vocoder | Shared FFT/STFT/stretch worker and streaming infrastructure; new DSP, after T2 multi-input and M3 pitch analysis. | Partitioned convolution IR/linear-phase EQ, frequency shift (Hz, distinct from pitch ratio), realtime pitch/formant shift, monophonic scale correction/MIDI harmonies, many-band and classic vocoders with real carrier/modulator routing. Measure latency, pitch cents, spectral/formant behavior, stereo balance and convolution versus direct FIR reference. RNNoise/Basic Pitch/offline stretch do not automatically provide realtime vocal correction. |
| E6 Buffered performance/time processors | New bounded replay rings/clock maps and curve editors, after T1/T2/T7. | Live transient retrigger/reloop, drawn time/volume envelopes, loaded-source scratching and twelve independent XY-steered performance effects. Test input-history wrapping, exact trigger/tempo sync, discontinuity crossfades, max memory, MIDI triggering and offline capture. Timeline slip/slice editing does not satisfy Gross Beat/Transporter. |
| E7 Note/control/rack modules | T2/T7 foundations, new bounded control sources and compiled transform nodes plus controls UI. | Note-triggered envelope, keyboard pitch/velocity source, audio peak/LFO source, formula, XY/XYZ, custom control surface; color routing, note envelopes, property scaling, key zones/remaps, sequencer/arpeggiator and layer fanout. Pan/volume LFO must actually process audio; the eight-unit envelope/filter bank needs E1 filters in real series/parallel audio routes, not only a control display. Share evaluator/UI but test each transformation/port type, linking ranges/smoothing/cycles and removal/undo. Effect-send taps require actual chain-position audio routing, not only post-fader sends. |
| E8 Notes, system instruments and peripheral effects | Project metadata/document viewers, device adapters after T5/A-MIDI/T8. | Multiple page-based notepad layouts with playback following; safe HTML project notebook without privileged script/IPC; OS General MIDI synth path on relevant Windows platform; lighting SDK output; calibrated tuner; modular CV on actual audio outputs. Reuse data but test distinguishing workflows. OS MIDI availability, CV voltage range/DC-coupled interfaces and lighting hardware are external requirements. Never claim voltage safety/calibration from float samples alone. |

**Rules for shared equivalents:** a single better engine can satisfy several rows if it exposes and tests all their distinct behaviors; record the exact mapping. It must not multiply the accounting score from superficial aliases. Core DSP behaviors may be complete before a specialist UI; record that as partial. Legacy processors can share a successor engine when the legacy feature contract is demonstrated; difficulty alone is not a won't-do reason.

## 7. Machine learning and audio-analysis workflows

### M1 — Job/asset/model contract

Own a new native analysis crate and `session/analysis_jobs.rs`, IPC job/review types and `features/analysis/*` after A-EDITOR; no live engine changes initially. Define input source fingerprint, document generation/edit revision, immutable audio selection/range, model/version/checksum/license, cancellation/progress, output artifacts and a review ticket. Background jobs run off audio/State, bounded concurrency/memory/disk, and return a proposal. Applying it rechecks tickets/source and dispatches one checked batch. Failed/cancelled/stale work removes only its own staging data. Successful external sources remain referenced through undo/save/archives; no speculative source GC. Model download/import is explicit and atomic, with checksum validation and offline failure UX. Mock mode tests the job state/review contract and reports that native inference is unavailable.

Exact gates: new native `analysis_jobs` integration tests with deterministic fake inference for state races/cancel/disk errors, project source/history tests, UI review tests, scoped Clippy/typecheck/lint. Fake inference proves lifecycle, not any ML row. Define a fixed-version CPU reference fixture before real inference adapters; do not require CUDA or a user Python installation for core native workflows. A bundled helper may be chosen only after size/platform/license/build review.

### M2 — Stem separation, denoising and remix import

After M1, implement separation yielding drum/bass/vocal/other sources of aligned length and sample origin; preview/select stems and one-step playlist/mixer import, then remix wizard that actually lays them out. Add RNNoise-style denoising as an editor job with wet/dry preview and source preservation, later an optional realtime processor only after deadline/latency tests. Noise profiling/tonal cleaning is separate from a vocal denoiser; A-EDITOR’s ordinary processing is not duplicated.

`win-playlist-deverb` additionally requires a trained reverb-removal model with distinct general-audio and vocal modes. No candidate or implementation is present at base. Select a reproducible licensed model/runtime against authored dry→reverberant references, using M1's jobs/review attachment. Measure tail reduction, speech/music damage and stereo coherence against an unchanged-input baseline; demonstrate both modes on appropriate real fixtures. A noise gate, denoiser or inverse EQ is not a substitute. Model availability/provenance and musical evaluation are explicit gates, with engineering investigation proceeding rather than an automatic won't-do designation.

Tests: authored separated sources and licensed mixed fixtures with reference outputs, alignment/resampling/channel preservation, reconstruction residual/SI-SDR or other explicitly chosen quality metrics, silence/clipping, short/long files, noisy speech and music loss, CPU/memory budgets, model absent/bad checksum, cancellation and stale Apply. Publish methodology and listening examples; no claim of universal separation quality. Run real CPU model fixtures in opt-in integration CI if artifact size prevents ordinary tests; ordinary CI still checks lifecycle deterministically. Both engine/runtime and weight redistribution terms must be established.

### M3 — Pitch detection, pitch/warp editors and realtime correction

After M1/A-EDITOR/N1, implement measured monophonic pitch/voicing/confidence/segmentation (classical detection is permissible), then a persistent editable pitch/timing region proposal with audition and non-destructive render, plus transient warp marker editor using existing stretch. Separate detected estimates from user overrides. Add realtime pitch correction/harmony effect only after dedicated prepared pitch-tracking/streaming synthesis latency work; current stretch latency/approximate formants must be shown rather than called zero latency.

Tests: calibrated notes/glides/vibrato/noise/silence and octave errors, reference F0 cents/voicing metrics, boundary continuity, actual corrected duration/pitch, warp marker ordering/limits, undo/save/reopen, audition/export match and stale source. Polyphonic pitch estimates do not automatically yield a vocal note editor. Tuner reuses analysis but needs a live selected source and calibrated display. Run analysis/stretch targeted suites, new pitch/warp UI tests and explicit musical listening evidence.

### M4 — Audio-to-MIDI

After M1/A-PIANO/T2, implement real inference/threshold/onset/offset conversion and review detected notes before inserting a pattern or score. Basic Pitch supplies a possible polyphonic reference; preserve conversion of seconds through tempo/meter maps and reject bounds rather than saturating silently. It is not a substitute for exact monophonic F0 in correction. Compare converted outputs to pinned upstream reference fixtures; test onset/offset/pitch/velocity, overlap, polyphony, held/repeated keys, silence and selected ranges. Apply is one undo step, cancelled/stale review changes nothing; native save/reopen and shared document/mock review tests pass. Report model quality separately from MIDI command correctness.

### License and dependency strategy

This survey checked upstream primary license pages on 2026-10-07. These are **candidates, not approved dependencies or approved model packs**:

- [Demucs code license](https://github.com/facebookresearch/demucs/blob/main/LICENSE) is MIT. Its [README](https://github.com/facebookresearch/demucs/blob/main/README.md) says the original repository is no longer maintained and points to a fork. Select a pinned model/runtime/maintenance strategy, verify weight terms and transitive binaries, and benchmark packaged CPU execution before committing to it.
- [RNNoise COPYING](https://github.com/xiph/rnnoise/blob/main/COPYING) contains BSD-style redistribution/notice/no-endorsement terms. Pin source/model and inspect generated weights, wrappers and rate/frame requirements. Native FFI resources are built/prepared/dropped on workers.
- [Basic Pitch license](https://github.com/spotify/basic-pitch/blob/main/LICENSE) is Apache-2.0; its [README](https://github.com/spotify/basic-pitch/blob/main/README.md) documents an ONNX model, mono analysis and 22,050 Hz preprocessing. Port exact preprocessing/postprocessing and measure against its reference, then inspect the selected ONNX runtime’s exact-version license, platform providers and redistributable binaries. Choosing a neural label without compatible shipped inference does not implement the feature.
- [Surge](https://github.com/surge-synthesizer/surge/blob/main/LICENSE), [Dexed](https://github.com/asb2m10/dexed/blob/master/LICENSE) and [Vital](https://github.com/mtytel/vital/blob/main/LICENSE) publish GPLv3 license texts. Extracting engines requires an audit of the chosen commit, submodules/FFI/build tools, per-file licenses and separate samples/presets/fonts/trademarks; do not bundle their entire UI or assume code license covers every asset. A short adapter/build feasibility delivery precedes an engine adoption. No need to replace the proven Windfall subtractive synth just to reuse a larger engine.
- [VST3 SDK license](https://github.com/steinbergmedia/vst3sdk/blob/master/LICENSE.txt) is MIT. The repo already pins `vst3=0.3.0` and clack 0.2.0 with manifest license notes. Keep these pins for the bridge first; a new format/SDK version needs its own reviewed license/source notice. ASIO is a separate build feature/SDK prerequisite, not automatically enabled by VST3 licensing.

Follow GPL-3.0-or-later app policy in `docs/ARCHITECTURE.md`; keep the MIT audio kit free of GPL-only implementation/data dependencies and own factory assets CC0. Record SPDX/license texts/source URL/exact commit or crate version/selected features/transitive notices/model provenance with every addition. Preserve codec LGPL obligations (the current LAME bindings/native encoder are described in `windfall-codec/Cargo.toml`), and distribute corresponding source/build information as required. Review optional AAC/REX/vendor SDKs independently; a separate process is not a license workaround. Compatible code-license candidates permit engineering investigation, not automatic distribution approval.

## 8. Extras, platform work and release

### X1 — Assistant and safe command/scripting integration

After T2/T7/M1, own `features/assistant/*`, bounded command proposal API and native script worker(s). `Command`/action registry is already the intended shared surface; never give model/script code direct mutable project pointers. Assistant answers local help and produces validated, reviewable command batches; opaque plugin actions honor native parameter capabilities and recording locks. Choose local/BYOK/service operation as a product decision; build provider-independent proposal/cancel/undo now. Python piano/device/modular scripts are separate compatibility targets, executed away from audio in bounded workers/processes with declared IO policy and time/memory limits. Realtime VFX scripting requires compiling/scheduling bounded events, not running Python inside the callback.

Tests: malformed/stale/overlarge proposals, failed batch rollback, recording exclusions, script timeout/process death and no external messages without user action; deterministic piano/device fixtures and saved script/version references. Command correctness can be synthetic; quality, service credentials/privacy and Python compatibility need separate evidence. No fake assistant panel completion.

### X2 — Performance mode, chord/generative tools, notation and touch

After T1/T2/A-MIDI/A-PIANO, own a live launch scheduler and new performance UI. Trigger/stop/queue clips on beat/bar boundaries with voice/release/tempo ownership and deterministic offline capture; do not repurpose song seek as live launching. Touch pads/keyboard use the accepted note API; chord detection and progression/riff generators use pure, seeded note transforms with review/one-step undo. Score sheet export requires a notation/layout artifact pipeline, pagination and readable accidentals/rhythm; MIDI download does not satisfy printable notation. Test launch races/tempo/meter transitions and note-off safety, chord inversions, seeded generator limits, multi-touch releases/focus and actual rendered score pages.

### X3 — Visualizer, synchronized video and own animation art

After T1/T8, own native media clock/decoding workers and `features/visualizer/*`, `features/video/*`. Video follows audio presentation time through seeks/loops/PDC, not a wall-clock setTimeout; verify drift and frame dropping. Visualizer consumes analyzer data and exports actual synchronized video through a reviewed codec/license pipeline. Animated note-driven character uses newly authored licensed art. Tests now: fake media clocks/drift/seeks, deterministic audio-reactive fixture/frame hashes and output media metadata; external GPU/webview/codec playback and native synchronization checks. An embedded video widget with no transport coupling is partial only.

### X4 — Phone remote, cloud backup and local mastering

Remote uses a paired/authenticated local protocol, read-only meter feed and the same named command validation; stale generations/recording guards apply. Test protocol authentication/replay/limits and loopback command undo before real phone/network/OS checks. Cloud backup follows N3 archives with checksum/version/conflict/cancel/retention semantics; provider/account/auth/hosting is a user scope choice, while portable archive/export and provider-neutral job plumbing proceed autonomously. Do not upload projects by default. Mastering is an actual local loudness/true-peak/dynamics render chain after E2, with target controls/review/reference metrics, not Image-Line’s service or a renamed limiter preset.

### X5 — Own loop/content library and legacy formats

After A-LIBRARY/T5, create provenance-checked own genre/key/tempo loop packs with audition/drop/fit and useful tutorial projects. Factory drum generation is reusable but not a full loop starter. Legacy DrumSynth/SimSynth/speech presets and ZGR need public format evidence, bounded readers/converters and licensed fixtures. WAV loop/slice/note metadata shares accepted marker/slice schemas; verify bytes independently. WavPack and AAC/M4A are distinct codec integrations with exact-version licensing and independently decoded export tests. REX remains an investigation/decision row until a GPL-compatible public implementation or legally usable SDK is established; do not mark impossible from age or complexity alone.

### P1 — Native OS/audio/plugin/editor support

Own platform-specific host/window adapters and packaged tests after A-VST3/N4, while retaining portable engine/model code. Current desktop runtime has a Windows-only gate; compile success on macOS/Linux does not remove it. Implement main-thread Cocoa/AppKit and Linux GUI-loop ownership, format paths/module lifecycle, native CLAP/VST3 editor parenting/floating/scaling, packaged helper launch and bridge IPC. AU is a macOS-native format adapter with actual audio/events/state/parameter/editor/latency tests, not a scanner label. Extend host ports/transport through T2 for sidechain/multi-output plugin capability. Wrapper smart-disable, fixed buffers, scaling/detached windows and threaded options need explicit DSP/lifecycle semantics and tests, not toggles that do nothing.

Audio backend inventory must follow the **actual cpal build features/hosts available**: Windows WASAPI/optional ASIO; macOS CoreAudio; Linux ALSA/JACK plus demonstrated PipeWire route/backend. Do not infer direct PipeWire hosting from the plan alone. Verify sample formats/channel maps/device loss/reconfigure/rate/buffer behavior and callback deadlines on actual OS/device combinations. Use `windfall-soak` and record actual frames, not requested frames. ASIO SDK/build distribution terms and working interface evidence are separate gates.

Multithreading follows a dedicated immutable graph scheduler delivery after T2/T7: precreated workers, bounded synchronization, known scheduling budget and deterministic accumulation order. No general Rayon/task spawn from the callback. Measure callback worst case and external low-buffer soak; preserve single-thread fallback. Running Windfall itself as CLAP/VST3/AU plugin is a separate X6 host-integration packet: host transport/sample rate/multiple outputs/state/GUI and reentrancy, tested inside actual other DAWs. It is not automatically supplied by hosting plugins in Windfall.

### R1 — Installers, updater, recovery, languages and first-use polish

Own a new release workflow, Tauri packaging/update configuration, explicit crash-report consent/recovery transport, translation catalog and help/tutorial UI. Parent alone publishes. Preserve `v0.1.0-alpha.1`; choose a new version for every later release, with matching Cargo/package/Tauri metadata and a reproducible artifact manifest. CI currently builds apps without bundling and has no release pipeline; add reproducible signed Windows/macOS and verified Linux packages, bundled bridge/model/content resources, GPL corresponding source/notices and checksum/SBOM outputs. Native uninstall/upgrade must preserve user projects/settings. Updater verifies signatures and channels, handles offline/corrupt/downgrade cases, rolls back safely, and never changes project format silently. Signing keys/certificates, notarization account and update host require the operator; implementation/test fixtures do not.

Recover from engine/plugin/helper crashes using existing autosaves plus bounded diagnostics without sample contents, private paths or plugin state bytes by default. Test fatal/nonfatal distinctions, user consent, retention/offline queue and restart recovery. A packaged security pass must replace null CSP with an appropriate tested policy before privileged HTML notebooks/remote/content are added; document any required local asset exceptions.

Native UI workflows include detached panels with one document owner and ordered revision patches across windows, DPI/plugin scaling, saved monitor layouts with offscreen recovery, focus/shortcut/menu isolation, high-contrast/readable themes, keyboard navigation and accessibility, localization of messages/numbers/plurals and guided build-beat/record/import/export tutorials. Existing docked panels and FL key preset are foundations, not detached-window support. Test two windows racing edits/close/reopen/device loss/project replacement and stale realtime subscriptions. Measure WKWebView/WebKitGTK renderer performance and fallback behavior before considering a shell change; Tauri/MIT registry/private repo choices are already implemented, not questions to reopen by default.

## 9. Phase acceptance gates and evidence collection

A phase ends with both its end-to-end story and correct row accounting. The following gates are proposed work; this survey does not claim they were executed. Parent runs combined workspace/strict Clippy/UI/generation gates once after each accepted batch, not every advisor or every unchanged task.

| Phase | Synthetic/native-headless gate available now or to add | Actual workflow / external evidence required |
| --- | --- | --- |
| 0 Spike | Existing device-supervisor/realtime/allocator tests and canvas renderer parity. CI must complete fmt/strict Clippy/workspace tests/app compile on Windows/macOS/Linux, plus binding/sim freshness. | `cargo run --release -p windfall-engine --bin windfall-soak -- --seconds 600 --buffer 128 --sample-rate 48000` on each supported low-latency route; zero xruns and documented actual buffer/cpu/device/OS. Native Tauri window controls playback and displays meters. Existing 480-frame Windows soak is useful partial evidence. Measure 10k-note editor in actual OS webviews. |
| 1 Make a beat | `session::tests::beat/files/playback`, rack/browser/document UI tests; archive/version tests as delivered; WAV independently decoded. | Fresh installer/user creates drum loop from own/user samples, previews, saves/reopens (audio still found), edits/history and exports correct WAV. Close residual graph/groups/fill/swing/preprocessing/metronome/tap/typing/scaling/backups rows according to their actual summaries. Preserve two copyright won't-do reasons. |
| 2 Write a song | Project/engine note/clip/automation/mixer/effects/render suites; piano/playlist/mixer/parameter/action UI plus meter/region/sidechain tests. | Create complete song with melody/drums/audio/automation/effects and routing, seek/loop/undo/save/reopen/export; verify export matches heard output within declared plugin determinism. Specialist tools, signatures/arrangements/presets/detached/multicore workflows need their own gates. Umbrella completion waits for selected child criteria. |
| 3 Record/edit | Capture format/allocator/fault and session ownership tests, clock injection, editor/slicer/stretcher quality, SMF/codec/stem independent decoding. | Real vocal over beat, count-in/monitor/latency calibration where promised, edit/slice/stretch, export mixer and playlist stems, reopen source attachments. Real MIDI input/output/clock/controller mapping/pickup and device loss. Record remaining monitoring/drift/listening limits honestly. |
| 4 Plugins/files | CLAP/VST3 fixtures process/state/editors/latency; scanner crash tests; **audio** helper kill/hang tests; modular graph tests; FLP/MIDI generated hostile/round-trip corpus and stale review tests. | Licensed popular free instrument/effect corpus (e.g. already evaluated Surge/OB-Xf) with actual versions, formats, editor/state/save/export on target OS; real FL projects from several available versions compare notes/clips/routing/automation/sample references and report approximation. AU only on macOS. No binary sound-equivalence promise for retained proprietary states. |
| 5 Long tail | Every instrument/effect/editor row links its family tests, spectral/transfer/F0/error measurements, asset/preset history/persistence, UI action and allocator proof; real model reference fixtures run. | Musical listening/usability on representative instruments/voices/mixes, external SDK/audio/CV/peripheral tests where relevant. All phase-5 rows accounted for individually; hosting an external synth or a common generic panel closes none by itself. |
| 6 Extras | Deterministic script/assistant proposal/performance/media/remote/cloud/content/notation/legacy-format tests and faults. Independent video/audio/file output inspection. | Real controller/phone/network/video/device/other-DAW flows; chosen service/user credentials only for explicitly configured optional services. Own art/loop packs and provenance. Resolve licensing/scope decisions before changing remaining exceptional rows. |
| 7 Release | Packaged build/signature/update-fixture tests, old-project compatibility/migrations, crash-recovery/localization/help/tutorial coverage, immutable release-manifest checks. | A stranger with private-repo access downloads, installs, launches and updates a **new** signed version on Windows/macOS/Linux; trust/notarization, audio/MIDI/plugins/webview/DPI/accessibility and uninstall tested. Signing/account/hosting evidence cannot be mocked into completion. |

Record each external run with commit, build/artifact hash, UTC date, OS/arch/runtime/GPU/device/driver, plugin/model version and license source, requested **and actual** rate/buffer, scenario/command, observed result and raw sanitized evidence path. Record skipped/failed tests as such. A requested 128 buffer, a Linux cross-compile, a fake MIDI device and a successful browser click each establish different things.

Combined parent verification, from the integrated root (Windows Git Bash/MSVC; ordinary tests use temporary TS_RS_EXPORT_DIR):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir apps/desktop typecheck
pnpm --dir apps/desktop lint
pnpm --dir apps/desktop test --maxWorkers=4
pnpm --dir apps/desktop build
node scripts/parity.mjs --check
node scripts/check-sim.mjs
node --test scripts/check-bindings.test.mjs
```

For freshness, parent generates to a temporary comparison directory with `scripts/gen-bindings.sh <directory>` and runs `node scripts/check-bindings.mjs <directory>` after final regeneration. Run actual platform CI/package gates when publishing, retaining portability tolerances limited to the documented computed automation-fixture values. No weakened checks to declare a platform finished.

## 10. Parity accounting, decisions and audit limits

`docs/parity/parity.json` is authoritative; `PARITY.md` is generated with `node scripts/parity.mjs`. This document's inventory freezes base statuses for drift review. Parent alone updates live status/evidence. Per accepted change, list exact row IDs, concrete behaviors, persisted schema/defaults, tested native/shared/mock path, checks/evidence, unverified external requirements and limitations. Prefer a machine-checkable row→test/evidence index in a future parent-owned improvement, with validation that all done rows have reachable evidence and all won't-do rows have a nonempty rationale. Do not force synthetic and external verification into the same Boolean.

Close a row only after its summary’s user-visible behavior works with save/reopen/undo where relevant, realtime safety, offline parity and meaningful failure handling. For umbrella rows, state required child rows and remaining exceptions; do not multiply a subsystem’s tests into blanket completion. For a shared equivalent, name each distinct capability and its acceptance evidence. Preserve unsupported imported data and warnings even after adding an approximate mapping. Added rows should have stable IDs; no deleting/rephasing a difficult row to make a phase green.

Existing justified restrictions stay: `core-sound-content` and `win-browser-fl-cloud-content` cannot ship Image-Line content. Own factory/loop/preset content remains required engineering work. Proprietary names/art/sounds and identical playback of every FL project are prohibited/out of scope by the plan; their exclusion is not permission to discard note/clip/controller fidelity.

Decisions requiring actual user/operator input, with independent work that can continue:

- VST2: `fmt-host-vst2` is still todo, outside the stated plan’s CLAP/VST3/AU selection, with an unresolved licensing path. Investigate compatible headers/host implementation and actual distribution terms, then propose a reasoned inclusion/restriction; do not automatically convert it to won't-do. Modern hosting/bridge proceeds.
- REX/vendor lighting/gamepad/speech dependencies: validate SDK/runtime/assets/licenses and relevant platforms. If an essential incompatible dependency has no lawful alternative, present evidence and a specific scope decision. Public-format/parser/mock-adapter work can proceed where permitted.
- Remote/assistant/cloud providers, data policy and service costs; model/own-content redistribution, signing credentials/update host, native OS/hardware availability. Build provider-neutral local contracts and deterministic fixtures while access decisions are pending; never invent service/hardware success.
- Exact replacement-quality promises and any capacity/platform exclusions must be explicit. Windows-first sequencing is practical given existing evidence, but the goal still includes macOS/Linux. A shell change requires measured failure, not a Linux warning in an old note.

Audit limits: source seams were inspected, not every source line or dependency subtree; no full security audit, new build/test execution, hardware/native install, Mac/Linux UI run, musical listening, proprietary project corpus download or model-weight redistribution audit was performed. Primary candidate license pages were checked, but exact adoption commits/transitives/assets still need review. No secrets were read or reproduced. No extra agents, external messages, pushes, PRs, tags or releases were created. The only deliverable of this branch is this roadmap.

## 11. Complete base-row dispatch inventory

The following table is an exhaustive snapshot from base `docs/parity/parity.json`, grouped by its existing areas. Route labels refer to this document; active reservations require their accepted commits before follow-on work. `B` means retain/retest baseline evidence, `Q` means retain the existing copyright restriction, and `D` means evidence-backed scope/SDK decision investigation. A route can still have missing subbehaviors described above. **Status is the base matrix status, not a new completion judgment.**

### Core features (23 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `core-lifetime-free-updates` | Lifetime Free Updates | 7 | todo | R1 |
| `core-stem-separation` | Stem Separation | 5 | todo | M2 |
| `core-audio-recording` | Audio Recording | 3 | in-progress | T4 |
| `core-audio-clips` | Audio Clips | 2 | done | B |
| `core-loop-starter` | Loop Starter | 6 | todo | X5 |
| `core-fl-studio-mobile-rack-fx` | FL Studio Mobile Rack + FX | 5 | todo | T7 |
| `core-audio-logger` | Audio Logger | 3 | todo | T4 |
| `core-chord-generator` | Chord Generator | 6 | todo | X2 |
| `core-gopher` | Gopher | 6 | todo | X1 |
| `core-denoising` | Denoising | 5 | todo | M2 |
| `core-sound-content` | Sound Content | 1 | wont-do | Q |
| `core-piano-roll` | Piano Roll | 2 | in-progress | A-PIANO / T2 |
| `core-mixer` | Mixer | 2 | in-progress | T3 |
| `core-full-song-arrangement` | Full Song Arrangement | 2 | done | B |
| `core-automation-clips` | Automation Clips | 2 | in-progress | T1 / T2 |
| `core-time-signature-changes` | Time signature changes | 2 | todo | T1 |
| `core-midi-support` | MIDI Support | 3 | todo | A-MIDI / T4 |
| `core-midi-out` | MIDI Out | 3 | todo | A-MIDI / T4 |
| `core-vst2-vst3-audio-unit-and-clap-support` | VST2, VST3, Audio Unit and CLAP support | 4 | in-progress | A-VST3 / N4 / P1 |
| `core-fl-studio-remote` | FL Studio Remote | 6 | todo | X4 |
| `core-fruity-envelope-controller` | Fruity Envelope Controller | 5 | todo | E7 / T2 |
| `core-fruity-keyboard-controller` | Fruity Keyboard Controller | 5 | todo | E7 / T2 |
| `core-fruity-voltage-controller` | Fruity Voltage Controller | 5 | todo | E8 / T4 |

### Main windows (113 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `win-rack-channel-rack` | Channel Rack | 1 | done | B |
| `win-rack-step-sequencer` | Step sequencer | 1 | done | B |
| `win-rack-graph-editor` | Graph editor | 1 | todo | T0 |
| `win-rack-channel-controls` | Channel mute, solo, pan and volume | 1 | done | B |
| `win-rack-mixer-track-selector` | Channel target mixer track selector | 1 | done | B |
| `win-rack-patterns` | Patterns and pattern selector | 1 | done | B |
| `win-rack-swing` | Swing (global and per channel) | 1 | in-progress | T0 |
| `win-rack-channel-groups` | Channel groups and filter | 1 | todo | T0 |
| `win-rack-channel-menu` | Channel button menu (clone, replace, insert, delete, rename, color) | 1 | done | B |
| `win-rack-fill-tools` | Fill each N steps and Advanced Fill tool | 1 | in-progress | T0 |
| `win-rack-cut-groups` | Cut and Cut-by groups | 1 | done | B |
| `win-rack-sampler-looping` | Channel sampler: loop points and ping-pong loop | 1 | done | B |
| `win-rack-sampler-precomputed` | Channel sampler: precomputed effects | 1 | todo | A-EDITOR / T0 |
| `win-rack-piano-roll-preview` | Mini piano roll preview | 2 | todo | T0 |
| `win-rack-send-to-piano-roll` | Send to Piano roll | 2 | todo | T0 |
| `win-rack-channel-envelopes` | Channel settings: envelopes, LFOs and filter | 2 | in-progress | T0 / T2 |
| `win-rack-arpeggiator` | Channel settings: arpeggiator | 2 | todo | T0 / T2 |
| `win-rack-echo-delay` | Channel settings: echo delay | 2 | todo | T0 / T2 |
| `win-rack-polyphony` | Channel settings: polyphony and portamento | 2 | todo | T0 / T2 |
| `win-rack-note-timing` | Channel settings: gate, shift and swing mix | 2 | todo | T0 |
| `win-rack-sampler-stretch` | Channel sampler: time-stretch and pitch modes | 3 | in-progress | N1 |
| `win-rack-layer-channel` | Layer channel (Fruity Layer) | 5 | todo | T7 |
| `win-piano-draw` | Piano roll: Draw tool | 2 | done | B |
| `win-piano-paint` | Piano roll: Paint tool and drum sequencer mode | 2 | in-progress | A-PIANO |
| `win-piano-delete` | Piano roll: Delete tool | 2 | done | B |
| `win-piano-mute` | Piano roll: Mute tool | 2 | todo | A-PIANO |
| `win-piano-slice` | Piano roll: Slice tool | 2 | todo | A-PIANO |
| `win-piano-select` | Piano roll: Select tool | 2 | done | B |
| `win-piano-zoom` | Piano roll: Zoom tool | 2 | todo | A-PIANO |
| `win-piano-playback` | Piano roll: Playback (scrub) tool | 2 | todo | A-PIANO |
| `win-piano-stamp` | Piano roll: Chord stamp | 2 | todo | A-PIANO |
| `win-piano-keyboard` | Piano roll: preview keyboard and key labels | 2 | done | B |
| `win-piano-event-editor` | Piano roll: event editor lane and note properties | 2 | in-progress | A-PIANO / T2 |
| `win-piano-slide-porta` | Piano roll: slide and portamento notes | 2 | in-progress | A-PIANO / T2; source pass in NOTE-ARTICULATION.md, platform transport pending |
| `win-piano-note-colors` | Piano roll: note colors (16 color groups) | 2 | in-progress | A-PIANO / T2; source pass in NOTE-COLORS.md, artifact/QA deferred |
| `win-piano-ghost-notes` | Piano roll: ghost notes | 2 | in-progress | A-PIANO; editable source pass in EDITABLE-GHOSTS.md, QA deferred |
| `win-piano-scale-highlighting` | Piano roll: scale highlighting and snap to scale | 2 | todo | A-PIANO |
| `win-piano-snap` | Piano roll: snap to grid | 2 | done | B |
| `win-piano-time-markers` | Piano roll: time markers and per-pattern time signatures | 2 | todo | T1 |
| `win-piano-waveform-helper` | Piano roll: waveform helper view | 2 | in-progress | A-PIANO / A-EDITOR; source pass in PIANO-WAVEFORM-HELPER.md, QA deferred |
| `win-piano-quantize` | Piano roll: Quantizer tool | 2 | in-progress | A-PIANO |
| `win-piano-articulate` | Piano roll: Articulator tool and Quick legato | 2 | todo | A-PIANO |
| `win-piano-chop` | Piano roll: Chopper tool and Quick chop | 2 | todo | A-PIANO |
| `win-piano-glue` | Piano roll: Glue | 2 | todo | A-PIANO |
| `win-piano-arpeggiate` | Piano roll: Arpeggiator tool | 2 | todo | A-PIANO |
| `win-piano-strum` | Piano roll: Strum tool | 2 | todo | A-PIANO |
| `win-piano-flam` | Piano roll: Flam tool | 2 | todo | A-PIANO |
| `win-piano-claw` | Piano roll: Claw machine tool | 2 | todo | A-PIANO |
| `win-piano-limit` | Piano roll: Key limiter tool | 2 | todo | A-PIANO |
| `win-piano-flip` | Piano roll: Flip tool | 2 | todo | A-PIANO |
| `win-piano-randomize` | Piano roll: Randomizer tool | 2 | in-progress | A-PIANO; source pass in PIANO-RANDOMIZER.md, artifact/QA deferred |
| `win-piano-scale-levels` | Piano roll: Scale levels tool | 2 | todo | A-PIANO |
| `win-piano-lfo` | Piano roll: LFO tool | 2 | in-progress | A-PIANO / T2; shared event/automation source pass in CURVE-LFO.md, artifact/QA deferred |
| `win-piano-riff-machine` | Piano roll: Riff machine | 6 | todo | X2 |
| `win-piano-scripting` | Piano roll scripting (Python) | 6 | todo | X1 |
| `win-piano-score-sheet` | Piano roll: export as score sheet | 6 | todo | X2 |
| `win-playlist-pattern-clips` | Playlist: pattern clips | 2 | done | B |
| `win-playlist-draw` | Playlist: Draw tool | 2 | done | B |
| `win-playlist-paint` | Playlist: Paint tool | 2 | done | B |
| `win-playlist-delete` | Playlist: Delete tool | 2 | done | B |
| `win-playlist-mute` | Playlist: Mute tool | 2 | done | B |
| `win-playlist-slip` | Playlist: Slip edit tool | 2 | todo | A-SLICE |
| `win-playlist-slice` | Playlist: Slice tool | 2 | todo | A-SLICE |
| `win-playlist-select` | Playlist: Select tool | 2 | done | B |
| `win-playlist-zoom` | Playlist: Zoom tool | 2 | todo | T1 / native tools |
| `win-playlist-playback` | Playlist: Playback tool | 2 | todo | T1 / native tools |
| `win-playlist-tracks` | Playlist tracks (name, color, mute, solo, resize) | 2 | in-progress | T1 |
| `win-playlist-track-groups` | Playlist: track grouping | 2 | todo | T1 |
| `win-playlist-instrument-audio-tracks` | Playlist: instrument tracks and audio tracks | 2 | todo | T1 |
| `win-playlist-time-markers` | Playlist: time markers | 2 | todo | T1 |
| `win-playlist-arrangements` | Playlist: arrangements | 2 | todo | T1 |
| `win-playlist-picker` | Playlist: clip source menu and picker panel | 2 | done | B |
| `win-playlist-clip-groups` | Playlist: clip grouping | 2 | todo | T1 |
| `win-playlist-make-unique` | Playlist: make unique | 2 | todo | T1 |
| `win-playlist-snap` | Playlist: snap | 2 | done | B |
| `win-playlist-selection-loop` | Playlist: timeline selection and loop region | 2 | todo | T1 |
| `win-playlist-audio-clip-fades` | Playlist: audio clip fades, crossfades and gain handles | 2 | in-progress | A-EDITOR / T0 |
| `win-playlist-audio-clip-properties` | Playlist: audio clip properties (gain, pan, pitch, reverse, normalize) | 2 | in-progress | A-EDITOR / T0 |
| `win-playlist-automation-editing` | Playlist: automation clip editing (curve shapes, step mode, LFO mode) | 2 | in-progress | T1 / T2 |
| `win-event-editor` | Event editor | 2 | todo | T1 / T2 |
| `win-playlist-audio-stretch` | Playlist: audio clip stretch and pitch-shift | 3 | done | B |
| `win-playlist-tempo-detect` | Playlist: detect tempo and fit to tempo | 3 | done | B |
| `win-playlist-consolidate` | Playlist: consolidate (freeze) tracks | 3 | todo | T6 |
| `win-playlist-deverb` | Playlist: Deverb | 5 | todo | M2 |
| `win-mixer-tracks` | Mixer: insert tracks, master track and current track | 1 | done | B / T3 capacity audit |
| `win-mixer-fader-pan` | Mixer: track fader, pan, mute and solo | 1 | done | B |
| `win-mixer-meters` | Mixer: peak meters | 1 | done | B |
| `win-mixer-effect-slots` | Mixer: 10 effect slots per track | 2 | done | B |
| `win-mixer-routing` | Mixer: track routing and send levels | 2 | done | B |
| `win-mixer-sidechain` | Mixer: sidechain routing | 2 | todo | T2 / T3 |
| `win-mixer-track-eq` | Mixer: integrated 3-band track EQ | 2 | todo | T3 |
| `win-mixer-track-utilities` | Mixer: phase invert, swap left/right and stereo separation | 2 | todo | T3 |
| `win-mixer-pdc` | Mixer: plugin delay compensation (automatic and manual) | 2 | in-progress | T3 |
| `win-mixer-docks-layouts` | Mixer: track docks and layout views | 2 | todo | T3 |
| `win-mixer-track-presets` | Mixer: track states (presets) | 2 | todo | T5 / T3 |
| `win-mixer-waveform-view` | Mixer: waveform meter view | 2 | todo | T8 / T3 |
| `win-mixer-multi-select` | Mixer: multi-track selection | 2 | todo | T3 |
| `win-mixer-audio-io` | Mixer: external audio input and output per track | 3 | todo | T4 |
| `win-mixer-record-arm` | Mixer: track record arm and disk recording | 3 | todo | T4 |
| `win-mixer-render-tracks` | Mixer: render tracks to wave files | 3 | todo | T6 |
| `win-browser-folders` | Browser: folder tree | 1 | done | B |
| `win-browser-preview` | Browser: sample preview | 1 | done | B |
| `win-browser-waveform` | Browser: waveform preview | 1 | done | B |
| `win-browser-search` | Browser: search | 1 | in-progress | A-LIBRARY |
| `win-browser-drag-drop` | Browser: drag and drop | 1 | done | B |
| `win-browser-backups` | Browser: project backups folder | 1 | todo | A-LIBRARY |
| `win-browser-tags` | Browser: tags | 2 | todo | A-LIBRARY |
| `win-browser-starred` | Browser: starred items | 2 | todo | A-LIBRARY |
| `win-browser-current-project` | Browser: current project tab | 2 | todo | A-LIBRARY |
| `win-browser-project-picker` | Project picker | 2 | todo | A-LIBRARY |
| `win-browser-plugin-database` | Browser: plugin database | 4 | todo | A-LIBRARY / A-VST3 / T5 |
| `win-browser-plugin-picker` | Plugin picker | 4 | todo | A-LIBRARY / A-VST3 / T5 |
| `win-browser-fl-cloud-content` | Browser: Library and Sounds tabs (FL Cloud content) | 1 | wont-do | Q |

### Instruments (41 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `inst-drumaxx` | Drumaxx | 5 | todo | I4 |
| `inst-harmor` | Harmor | 5 | todo | I3 |
| `inst-kepler-exo` | Kepler Exo | 5 | todo | I1 |
| `inst-morphine` | Morphine | 5 | todo | I3 |
| `inst-ogun` | Ogun | 5 | todo | I3 |
| `inst-poizone` | Poizone | 5 | todo | I1 |
| `inst-sakura` | Sakura | 5 | todo | I4 |
| `inst-sawer` | Sawer | 5 | todo | I1 |
| `inst-toxic-biohazard` | Toxic Biohazard | 5 | todo | I2 |
| `inst-transistor-bass` | Transistor Bass | 5 | todo | I1 |
| `inst-directwave-full` | DirectWave Full | 5 | todo | I5 |
| `inst-harmless` | Harmless | 5 | todo | I3 |
| `inst-kepler` | Kepler | 5 | todo | I1 |
| `inst-slicex` | Slicex | 5 | todo | I6 / A-SLICE |
| `inst-soundfont-player` | SoundFont Player | 5 | todo | I5 |
| `inst-sytrus` | Sytrus | 5 | todo | I2 |
| `inst-3x-osc` | 3x OSC | 2 | done | B |
| `inst-autogun` | Autogun | 5 | todo | I3 |
| `inst-bassdrum` | BassDrum | 5 | todo | I4 |
| `inst-beepmap` | BeepMap | 5 | todo | I3 |
| `inst-boobass` | BooBass | 5 | todo | I4 |
| `inst-channel-sampler` | Channel Sampler | 1 | in-progress | N1 / T0 |
| `inst-directwave-player` | DirectWave Player | 5 | todo | I5 |
| `inst-drumpad` | Drumpad | 5 | todo | I4 |
| `inst-flex` | FLEX | 5 | todo | I1 |
| `inst-fruity-drumsynth-live` | Fruity DrumSynth Live | 5 | todo | I4 |
| `inst-fruity-dx10` | Fruity DX10 | 5 | todo | I2 |
| `inst-fruity-granulizer` | Fruity Granulizer | 5 | todo | I6 / A-SLICE |
| `inst-fruity-kick` | Fruity Kick | 5 | todo | I4 |
| `inst-fruity-pad-controller-fpc` | Fruity Pad Controller (FPC) | 5 | todo | I5 |
| `inst-fruity-slicer` | Fruity Slicer | 5 | todo | I6 / A-SLICE |
| `inst-fruity-slicer-2` | Fruity Slicer 2 | 3 | todo | A-SLICE |
| `inst-gms` | GMS | 5 | todo | I1 |
| `inst-minisynth` | MiniSynth | 5 | todo | I1 |
| `inst-plucked` | Plucked! | 5 | todo | I4 |
| `inst-simsynth` | SimSynth | 5 | todo | I1 |
| `inst-speech-synthesizer` | Speech Synthesizer | 5 | todo | I7 / D |
| `inst-wave-traveller` | Wave Traveller | 5 | todo | I6 / A-SLICE |
| `inst-fl-keys` | FL Keys | 5 | todo | I5 |
| `inst-dashboard` | Dashboard | 5 | todo | I7 / D |
| `inst-fruity-vibrator` | Fruity Vibrator | 5 | todo | I7 / D |

### Effects (80 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `fx-transmitter` | Transmitter | 5 | todo | E2 |
| `fx-luxeverb` | LuxeVerb | 5 | todo | E3 |
| `fx-pitch-shifter` | Pitch Shifter | 5 | todo | E5 / M3 |
| `fx-transient-processor` | Transient Processor | 5 | todo | E2 |
| `fx-emphasis` | Emphasis | 5 | todo | E2 |
| `fx-transporter` | Transporter | 5 | todo | E6 |
| `fx-gross-beat` | Gross Beat | 5 | todo | E6 |
| `fx-hardcore-11-guitar-fx` | Hardcore (11 Guitar FX) | 5 | todo | E4 |
| `fx-low-lifter` | Low Lifter | 5 | todo | E2 |
| `fx-pitcher` | Pitcher | 5 | todo | E5 / M3 |
| `fx-vintage-chorus` | Vintage Chorus | 5 | todo | E3 |
| `fx-vintage-phaser` | Vintage Phaser | 5 | todo | E3 |
| `fx-frequency-shifter` | Frequency Shifter | 5 | todo | E5 / M3 |
| `fx-hyper-chorus` | Hyper Chorus | 5 | todo | E3 |
| `fx-maximus` | Maximus | 5 | todo | E2 |
| `fx-multiband-delay` | Multiband Delay | 5 | todo | E3 |
| `fx-spreader` | Spreader | 5 | todo | E1 |
| `fx-vocodex` | Vocodex | 5 | todo | E5 / M3 |
| `fx-control-surface` | Control Surface | 5 | todo | E7 / T2 / T7 |
| `fx-distructor` | Distructor | 5 | todo | E4 |
| `fx-effector-12-fx` | Effector (12 FX) | 5 | todo | E6 |
| `fx-equo` | EQUO | 5 | todo | E1 |
| `fx-frequency-splitter` | Frequency Splitter | 5 | todo | E2 |
| `fx-fruity-balance` | Fruity Balance | 5 | todo | N2 |
| `fx-fruity-blood-overdrive` | Fruity Blood Overdrive | 5 | todo | E4 |
| `fx-fruity-chorus` | Fruity Chorus | 5 | todo | E3 |
| `fx-fruity-compressor` | Fruity Compressor | 2 | done | B |
| `fx-fruity-convolver` | Fruity Convolver | 5 | todo | E5 / M3 |
| `fx-fruity-delay-2` | Fruity Delay 2 | 5 | todo | E3 |
| `fx-fruity-delay-3` | Fruity Delay 3 | 2 | done | B / E3 audit |
| `fx-fruity-delay-bank` | Fruity Delay Bank | 5 | todo | E3 |
| `fx-fruity-fast-dist` | Fruity Fast Dist | 5 | todo | N2 |
| `fx-fruity-filter` | Fruity Filter | 5 | todo | E1 |
| `fx-fruity-flanger` | Fruity Flanger | 5 | todo | E3 |
| `fx-fruity-flangus` | Fruity Flangus | 5 | todo | E3 |
| `fx-fruity-formula-controller` | Fruity Formula Controller | 5 | todo | E7 / T2 / T7 |
| `fx-fruity-html-notebook` | Fruity HTML NoteBook | 5 | todo | E8 |
| `fx-fruity-limiter` | Fruity Limiter | 2 | done | B / E2 / T8 audit |
| `fx-fruity-love-philter` | Fruity Love Philter | 5 | todo | E7 / T2 / T7 |
| `fx-fruity-lsd` | Fruity LSD | 5 | todo | E8 |
| `fx-fruity-multiband-compressor` | Fruity Multiband Compressor | 5 | todo | E2 |
| `fx-fruity-notebook` | Fruity NoteBook | 5 | todo | E8 |
| `fx-fruity-notebook-2` | Fruity NoteBook 2 | 5 | todo | E8 |
| `fx-fruity-panomatic` | Fruity PanOMatic | 5 | todo | E7 / T2 / T7 |
| `fx-fruity-parametric-eq` | Fruity Parametric EQ | 5 | todo | E1 |
| `fx-fruity-parametric-eq2` | Fruity Parametric EQ2 | 2 | done | B / T8 audit |
| `fx-fruity-phaser` | Fruity Phaser | 5 | todo | E3 |
| `fx-fruity-reeverb-2` | Fruity Reeverb 2 | 2 | done | B |
| `fx-fruity-scratcher` | Fruity Scratcher | 5 | todo | E6 |
| `fx-fruity-send` | Fruity Send | 5 | todo | E7 / T2 / T7 |
| `fx-fruity-soft-clipper` | Fruity Soft Clipper | 5 | todo | N2 |
| `fx-fruity-squeeze` | Fruity Squeeze | 5 | todo | E4 |
| `fx-fruity-stereo-enhancer` | Fruity Stereo Enhancer | 5 | todo | E1 |
| `fx-fruity-stereo-shaper` | Fruity Stereo Shaper | 5 | todo | N2 |
| `fx-fruity-vocoder` | Fruity Vocoder | 5 | todo | E5 / M3 |
| `fx-fruity-waveshaper` | Fruity WaveShaper | 5 | todo | E4 |
| `fx-fruity-x-y-controller` | Fruity X-Y Controller | 5 | todo | E7 / T2 / T7 |
| `fx-fruity-x-y-z-controller` | Fruity X-Y-Z Controller | 5 | todo | E7 / T2 / T7 |
| `fx-patcher` | Patcher | 4 | todo | T7 |
| `fx-fruity-peak-controller` | Fruity Peak Controller | 5 | todo | E7 / T2 / T7 |
| `fx-razer-chroma` | Razer Chroma | 5 | todo | E8 |
| `fx-soundgoodizer` | Soundgoodizer | 5 | todo | E2 |
| `fx-tuner` | Tuner | 5 | todo | E8 |
| `fx-vfx-color-mapper` | VFX Color Mapper | 5 | todo | E7 / T2 / T7 |
| `fx-vfx-envelope` | VFX Envelope | 5 | todo | E7 / T2 / T7 |
| `fx-vfx-level-scaler` | VFX Level Scaler | 5 | todo | E7 / T2 / T7 |
| `fx-vfx-keyboard-splitter` | VFX Keyboard Splitter | 5 | todo | E7 / T2 / T7 |
| `fx-vfx-key-mapper` | VFX Key Mapper | 5 | todo | E7 / T2 / T7 |
| `fx-vfx-sequencer` | VFX Sequencer | 5 | todo | E7 / T2 / T7 |
| `fx-emphasizer` | Emphasizer | 5 | todo | E2 |
| `fx-vfx-script` | VFX Script | 6 | todo | X1 |
| `fx-fruity-7-band-eq` | Fruity 7 Band EQ | 5 | todo | E1 |
| `fx-fruity-bass-boost` | Fruity Bass Boost | 5 | todo | E1 |
| `fx-fruity-center` | Fruity Center | 5 | todo | N2 |
| `fx-fruity-delay` | Fruity Delay | 5 | todo | E3 |
| `fx-fruity-fast-lp` | Fruity Fast LP | 5 | todo | E1 |
| `fx-fruity-free-filter` | Fruity Free Filter | 5 | todo | E1 |
| `fx-fruity-mute-2` | Fruity Mute 2 | 5 | todo | N2 |
| `fx-fruity-phase-inverter` | Fruity Phase Inverter | 5 | todo | N2 |
| `fx-fruity-reeverb` | Fruity Reeverb | 5 | todo | E3 |

### Visual and video (7 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `vis-fruity-video-player` | Fruity Video Player | 6 | todo | X3 |
| `vis-fruity-big-clock` | Fruity Big Clock | 5 | todo | T8 |
| `vis-fruity-db-meter` | Fruity dB Meter | 5 | todo | T8 |
| `vis-fruity-spectroman` | Fruity Spectroman | 5 | todo | T8 |
| `vis-video-visualizer-zgameeditor` | Video Visualizer (ZGameEditor) | 6 | todo | X3 |
| `vis-wave-candy` | Wave Candy | 5 | todo | T8 |
| `vis-fruity-dance` | Fruity Dance | 5 | todo | X3 |

### Audio editors (3 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `editor-newtone` | Newtone | 5 | todo | M3 |
| `editor-edison` | Edison | 3 | todo | A-EDITOR |
| `editor-newtime` | Newtime | 5 | todo | M3 |

### Formats and hosting (36 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `fmt-project-native` | Project file save and open (.flp) | 1 | done | B |
| `fmt-project-zip` | Zipped project (.zip) | 1 | todo | N3 |
| `fmt-save-new-version` | Save new version | 1 | todo | N3 |
| `fmt-autosave-backup` | Autosave and backups | 1 | done | B |
| `fmt-project-templates` | New from template and Save as template | 2 | todo | T5 |
| `fmt-flp-import` | FL Studio project (.flp) as an import source | 4 | in-progress | F1 |
| `fmt-cloud-backup` | Back up to FL Cloud | 6 | todo | X4 |
| `fmt-state-fst` | State file (.fst) | 2 | todo | T5 |
| `fmt-score-fsc` | Score file (.fsc) | 2 | todo | T5 |
| `fmt-import-wav` | Sample import: WAV | 1 | done | B |
| `fmt-import-mp3` | Sample import: MP3 | 3 | done | B |
| `fmt-import-ogg` | Sample import: OGG | 3 | done | B |
| `fmt-import-flac` | Sample import: FLAC | 3 | done | B |
| `fmt-import-midi` | MIDI file import | 4 | done | B |
| `fmt-import-rex` | Sample import: ReCycle loops (.rex, .rx2, .rcy) | 6 | todo | D |
| `fmt-import-legacy-presets` | Sampler sources: DrumSynth (.ds), SimSynth (.syn) and speech (.speech) presets | 6 | todo | X5 |
| `fmt-import-zgr` | BeatCreator/BeatSlicer grid file (.zgr) | 6 | todo | X5 |
| `fmt-export-wav` | Export: WAV | 1 | done | B |
| `fmt-export-mp3` | Export: MP3 | 3 | done | B |
| `fmt-export-ogg` | Export: OGG | 3 | done | B |
| `fmt-export-flac` | Export: FLAC | 3 | done | B |
| `fmt-export-midi` | Export: MIDI file | 3 | done | B |
| `fmt-export-stems-mixer` | Export: split mixer tracks (stems) | 3 | done | B |
| `fmt-export-playlist-tracks` | Export: all playlist tracks | 3 | todo | T6 |
| `fmt-export-options` | Export options (song or pattern, tail, bit depth, dithering, resampling) | 3 | done | B |
| `fmt-export-m4a` | Export: M4A (AAC) | 6 | todo | X5 |
| `fmt-export-wav-markers` | Export: loop, slice and note markers in WAV files | 6 | todo | X5 |
| `fmt-wavpack` | WavPack compressed audio | 6 | todo | X5 |
| `fmt-host-vst3` | Plugin hosting: VST3 | 4 | in-progress | A-VST3 / N4 / P1 |
| `fmt-host-clap` | Plugin hosting: CLAP | 4 | in-progress | A-VST3 / N4 / P1 |
| `fmt-host-au` | Plugin hosting: Audio Unit | 4 | todo | P1 |
| `fmt-host-vst2` | Plugin hosting: VST2 | 4 | todo | D |
| `fmt-host-plugin-manager` | Plugin manager (scan and verify) | 4 | in-progress | A-VST3 / N4 / P1 |
| `fmt-host-bridging` | Bridged plugins (separate process) | 4 | todo | N4 / P1 |
| `fmt-host-wrapper-options` | Plugin wrapper options (smart disable, fixed-size buffers, threaded processing, scaling, detached window) | 4 | todo | N4 / P1 |
| `fmt-host-fl-as-plugin` | FL Studio as a VST or AU plugin | 6 | todo | X6 / P1 |

### Workflow, MIDI and settings (39 rows)

| Row ID | FL reference | Phase | Base status | Route |
| --- | --- | --- | --- | --- |
| `wf-audio-settings` | Audio settings (driver, device, sample rate, buffer length) | 0 | in-progress | P1 |
| `wf-transport` | Transport (play, stop, record, pattern/song mode, song position) | 1 | in-progress | T4 |
| `wf-tempo` | Tempo and tempo tapper | 1 | in-progress | T0 |
| `wf-metronome` | Metronome | 1 | todo | T4 |
| `wf-typing-keyboard` | Typing keyboard to piano keyboard | 1 | todo | T4 |
| `wf-themes` | Themes | 1 | done | B |
| `wf-ui-scaling` | Interface scaling | 1 | todo | R1 |
| `wf-hint-bar` | Hint bar | 1 | done | B |
| `wf-toolbar-meters` | Output meter and CPU/memory panels | 1 | in-progress | T0 |
| `wf-undo-history` | Undo and edit history | 2 | done | B |
| `wf-multithreading` | Multithreaded processing | 2 | todo | P1 |
| `wf-global-snap` | Global snap | 2 | todo | T5 |
| `wf-detached-windows` | Detached windows | 2 | todo | R1 |
| `wf-keyboard-shortcuts` | Keyboard and mouse shortcuts | 2 | done | B |
| `wf-macros` | Tools menu macros | 2 | todo | T0 |
| `wf-project-info` | Project info (title, author, genre, comments) | 2 | todo | T5 |
| `wf-project-settings` | Project settings (time signature, timebase, panning law) | 2 | in-progress | T5 |
| `wf-midi-settings` | MIDI settings (input and output devices, ports, controller type) | 3 | todo | A-MIDI / T4 |
| `wf-midi-recording` | Note recording from MIDI input | 3 | todo | A-MIDI / T4 |
| `wf-step-edit` | Step editing (step entry) | 3 | todo | T4 |
| `wf-score-logger` | Score logger (dump score log to pattern) | 3 | todo | T4 |
| `wf-count-in` | Recording count-in | 3 | todo | T4 |
| `wf-loop-recording` | Loop recording (takes) | 3 | todo | T4 |
| `wf-automation-recording` | Automation recording | 3 | todo | T4 |
| `wf-controller-linking` | Link to controller (remote control settings, mapping formula, smoothing) | 3 | todo | A-MIDI / T4 |
| `wf-multilink` | Multilink to controllers | 3 | todo | A-MIDI / T4 |
| `wf-pickup` | Pickup (takeover) mode | 3 | todo | A-MIDI / T4 |
| `wf-midi-sync` | MIDI clock output (send master sync) | 3 | todo | A-MIDI / T4 |
| `wf-internal-controllers` | Internal controller linking | 5 | todo | E7 / T2 |
| `wf-audio-to-midi` | Audio to notes (pitch editor 'Send score') | 5 | todo | M4 |
| `wf-remix-a-song` | Remix a song wizard | 5 | todo | M2 |
| `wf-performance-mode` | Performance mode | 6 | todo | X2 |
| `wf-midi-scripting` | MIDI scripting (Python device scripts) | 6 | todo | X1 |
| `wf-preconfigured-controllers` | Preconfigured controller support | 6 | todo | X1 |
| `wf-touch-controllers` | Touch controllers (virtual keyboard and drum pads) | 6 | todo | X2 |
| `wf-chord-detection` | Chord detection panel | 6 | todo | X2 |
| `wf-cloud-mastering` | Mastering on export (FL Cloud) | 6 | todo | X4 |
| `wf-languages` | Interface languages | 7 | todo | R1 |
| `wf-help-tutorials` | Help menu and guided tutorials | 7 | todo | R1 |
