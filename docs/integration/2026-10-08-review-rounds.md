# Fixed-source review rounds, 2026-10-08

These reports keep standards and specification findings separate. They are
independent reviews of fixed source before root integration; the status after
each round records subsequent composition. Owners received all findings and
their full prior briefs; the full project remains active.

# Timeline R3

Fixed source: `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, parent
`e0809d0e1933e650667f379dbaf4687196d30865`, in
`gpt/t3-timeline-regions`.

## Standards

**Standards-only R3: three documented-standard findings.**

Reviewed fixed `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, directly atop `e0809d0e1933e650667f379dbaf4687196d30865`.

1. **Hydration changes do not notify the action registry.** `apps/desktop/src/features/playlist/timeline-actions.ts:124` now reads `hydrated` for Clear’s enabled state, but `apps/desktop/src/features/playlist/actions.ts:508` watches only tool, selection and active. Hydration returning no region changes none of those values, leaving open registry consumers stale. The `docs/ARCHITECTURE.md:437` requires notification. Include `hydrated` in the selector.

2. **The Timeline dropdown duplicates registered actions.** `apps/desktop/src/features/playlist/timeline-controls.tsx:373` supplies separate titles, disabled predicates and handlers for Play/Loop/Zoom/Export/Clear. This bypasses the `docs/ARCHITECTURE.md:430`, omitting registry shortcuts and disabled reasons. Render these static entries through `ActionMenuItem`. Inherited from first delivery.

3. **Timeline entity IDs omit required newtypes.** `crates/windfall-project/src/timeline.rs:36` and its marker type use `pub id: u32`; update/remove commands likewise accept raw integers. The `docs/ARCHITECTURE.md:67` requires `u32` newtypes. Use serde-transparent identity types, preserving numeric JSON. Inherited from first delivery.

No additional recording/State, reserved-seam or callback-standard breach found. No separate baseline-smell findings.

Independent verification was fixed-blob source inspection; no tests, builds, edits or delegation. The owner’s 248 UI/77 native-engine passes and strict checks remain provenance. Worktree is clean at the fixed hash. Combined-root/artifact integration and physical/external-plugin/non-Windows verification remain gates; staged follow-ups remain open.

## Spec

**Spec verdict: changes requested — two P2 findings.**

Reviewed `e0809d0e1933e650667f379dbaf4687196d30865` → `59c8f7f3adc0c552fca972b515e5219ddaf24d06`.

1. **Chained pending Plays lose the live cancellation target.** `apps/desktop/src/features/playlist/timeline-store.ts:44` prefers the successor’s unpublished `guard` over its inherited `cancel`. Start Play A, start Play B while A’s Play is pending, hold B’s region publication, let A commit, then Clear before B commits. Clear targets B although native still owns A. The actual fixed-function harness ended with `playing: true`, owner request 1, watermark 3, and both native region and UI selection null. This violates the approved requirement that cancellation “must reconcile any transport it commits during invalidation.” Retain cancellation authority across uncommitted successors. Native consequences are source-traced.

2. **Song meters leak into pattern-mode plugin transport.** `crates/windfall-engine/src/processor.rs:712` consults the song meter map without checking playback mode. With scalar 4/4 and a song 7/8 change at tick zero, pattern playback sends hosted plugins 7/8 while pattern labels remain 4/4. The contract says “per-pattern signatures remain scalar.” Restrict song-map lookup to song mode. **Source-traced**, including forwarding through the native plugin adapter.

Independent verification: **10 cached native timeline tests passed**, single-threaded, after dependency-path, fingerprint, timestamp and inventory checks. Other R1/R2 closures were source-traced. The owner’s 248 shared-WASM UI passes, 77 native/engine passes and strict checks remain provenance; restored legacy WASM was not tested.

Worktree remains clean. No edits, builds, regeneration or delegation occurred. Combined-root and physical/external-plugin/non-Windows behavior remain unverified.

Standards: three documented findings; specification: two P2 findings.
All five are assigned to the same implementation owner. New identity types
must preserve numeric JSON. Registry notification/menu hooks and song-only
meter lookup have narrow authorized source windows. No finding is waived
because the owner's earlier tests passed.

# Release foundation R1

Fixed source: `671910694f4be9397e5bb63f9da730e920143f5d`, parent
`f7102e7c793c0153433edc4b7125d55d2fd0e8b7`, in
`gpt/t3-release-engineering-r1`.

## Standards

Standards review of `f7102e7c...67191069`: **0 documented-standard breaches; 2 nonblocking judgment-call smells.**

- **Possible Duplicated Code / Shotgun Surgery** — `scripts/release/cli.mjs:12`. The common options `"source", "commit", "version", "channel", "mode"` recur in three lists; command names also duplicate the `specific` map’s keys. A CLI contract change requires synchronized edits. Define common options once and derive accepted options and command names from the command map.

- **Possible Duplicated Code** — `scripts/release/source.mjs:375`. Two adjacent `git/matching-refs/tags/...` loops repeat the same normalization and `"release tag identity is already reserved"` check, differing only by the `v` prefix. Iterate over the two tag prefixes using one lookup/check body.

Workflow contract validation and syntax checks for all eight Node modules passed. Worktree remains clean. The reported 38 tests were not rerun because their fixtures invoke Cargo metadata, prohibited for this review.

## Spec

SPEC review of `f7102e7c…67191069`: **two findings; changes requested.**

- **P1 — Resource admission misses supported platform overrides.** `scripts/release/source.mjs:241` rejects only `tauri.PLATFORM.conf.json`. A future source commit can contain `tauri.windows.conf.json5` or `Tauri.windows.toml` adding model resources or helpers; `resources()` still reports `models/helpers: not-admitted` and inventories only factory content. The pinned Tauri CLI [enables these formats](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-cli/Cargo.toml#L59) and [merges platform configuration](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-cli/src/helpers/config.rs#L145). A bounded Node fixture confirmed the JSON5 override passes the resource gate. This violates “Future bridge/model resources must be an explicit validated packaging contract.” Reject every supported override format until admitted, or validate the effective merged configuration.

- **P2 — Independent verification does not bind SBOM declarations and edges to source metadata.** `scripts/release/dependencies.mjs:401` accepts arbitrary nonempty license declarations; its graph checks validate identities and node coverage, not expected edges. `scripts/release/candidate.mjs:282` relies on this validator. Starting with genuine fixture evidence, changing workspace licenses from `GPL-3.0-or-later` to `MIT` and deleting the desktop→core edge still passed validation. Updated inventory hashes/checksums would not repair the missing source binding. This falls short of evidence “from pinned Cargo/npm metadata/lockfiles” and rejection of “truncated metadata.” Recheck declarations and graph edges against authoritative metadata.

Workflow contract and actual lock readers passed; two selected tests passed. The reported 38-test suite was not rerun because it invokes Cargo. Probes used owned temporary directories and were cleaned up. The fixed worktree remains clean; no native or publication actions ran.

Standards: no documented breach and two nonblocking judgment calls;
specification: one P1 and one P2 finding. The resource-override and
source-bound SBOM repairs are assigned to the same owner, alongside pinned
upload-action provenance. Existing Tauri/version/updater metadata and
publication remain closed for this foundation. No release, upload, native
bundle, signature or installation was performed by either review.

# Filter family downstream R4

Fixed composed source `f7ae4583737e743d4109a451de95f4af0fabcd33`, comparison `caeef7794bf6ec79e4f8cc34aa30e2fb74ba991c`. The parent test-only clock correction is part of the review; DSP/engine production code was unchanged.

## Standards

Clean scoped R4 **standards verdict** for fixed `f7ae4583737e743d4109a451de95f4af0fabcd33` against `caeef779`: **no actionable documented-standard violations or baseline-smell findings**.

The downstream tests reuse existing document, engine, mixer and generic-control paths. Independent signal references avoid product coefficient builders; allocations remain outside guarded callbacks, and WASM document lifetimes are explicitly cleaned up.

The restoration correction preserves the departing-share, first-frame and `2e-6` bounds. It checks every serial-handover frame, full wet from frame 400 and settling from frame 768 with an independent bass-pole justification.

Independently verified:

- All previous 12 descriptors and 141 automation-range fixtures are preserved.
- Existing parent-generated comparison output matches all 174 fixed binding files.
- Fixed WASM bytes, SHA-256 and transitive source-input hash match committed metadata.
- Handwritten-source `git diff --check` passes. The unfiltered check flags eight generated ts-rs whitespace lines, excluded from standards findings as instructed.

**No tests were executed by this reviewer.** Parent logs corroborate the corrected six-test engine pass and earlier failure; owner/parent test, lint, build and throughput results retain their original provenance.

No files changed, builds, regeneration or delegation occurred. Utility acceptance was treated as context. The separate Spec verdict and requested hardware/listening/platform gates still control parity closure.

## Spec

Clean scoped **R4 spec verdict** for fixed `f7ae4583737e743d4109a451de95f4af0fabcd33` against `caeef7794bf6ec79e4f8cc34aa30e2fb74ba991c`: **no actionable spec findings**.

Persistence, defaults, indexed controls, all seven modes, automation/history, generic mixer UI and native rendering follow the E1 contract. The restore correction preserves the departing-share and first-frame assertions and the `2e-6` bound. Its independent cold-filter recurrence checks all 1,024 frames: incoming audio begins at frame 161, reaches full wet at 400, and the bass pole justifies the always-wet endpoint at 768.

Independently verified:

- Exact cached executables attributed through source hashes, linked PDBs, dependency fingerprints, timestamps and test inventories.
- **Four project and six engine tests passed**, single-threaded; no failures or ignores.
- Nine restore paths: maximum serial-reference error `6.519258e-9`. Guarded callbacks recorded zero allocator calls; live/offline/both stem modes agree.
- All **174** fixed generated files match the cached comparison inventory. Fixed WASM bytes, SHA-256 and dependency-source checksum match its metadata: **1,881,168 bytes**.

Verification caveat: full fixed `git diff --check` reports eight trailing-whitespace lines in generated parameter types. The nongenerated scope passes; classification belongs to the standards axis.

The 299 DSP passes, 70 UI passes, Clippy, formatting and CPU measurements remain owner/parent provenance. I performed no edits, builds, regeneration, UI execution or delegation.

This supports the bounded headless E1 delivery. Device deadlines, musical listening/usability and macOS/Linux evidence remain unverified; this review alone does not close the three parity rows.

Both axes are clean for bounded headless E1 delivery. All three rows retain their physical audio, listening/usability and other-platform gates.

# Analysis foundation R2

Fixed `f006ce1b6ad8a819be1ca58bb228b95d64bea1d5`, direct parent `89d5ffbd3cb426d62d36786636a1e0a6e032b8b8`.

## Standards

**Standards R2: no documented-standard violations; one nonblocking judgment call.**

Reviewed immutable `f006ce1b6ad8a819be1ca58bb228b95d64bea1d5` against direct parent `89d5ffbd3cb426d62d36786636a1e0a6e032b8b8`. All ten changed paths are authorized; the worktree remained clean.

- **[P3, judgment call] Possible Duplicated Code:** [jobs.rs:178](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-analysis/src/jobs.rs:178) adds the same capture/model/liveness eligibility predicate already implemented by `Review::check_eligibility` at line 162: `self.input.stamp() != current || self.model != *model || !current.source_identity.is_live()`. Future guard changes must update both paths. A shared borrowed validator would preserve the allocation-free seam while keeping eligibility rules together.

Independent source tracing found no remaining standards issue in the final eligibility/acknowledgement boundary: metadata is prebuilt, eligibility borrows, and acknowledgement defers destruction. Strict codec decoding shares the existing parser and preserves ordinary entry signatures and sanitation.

No files were written, builds performed, or test binaries executed. The reported 37 analysis tests, 145 codec tests and Clippy passes remain owner-provided evidence. Spec acceptance was outside this axis.

## Spec

**Spec R2 verdict: clean within the foundation scope. No new concrete findings.**

Reviewed fixed `f006ce1b6ad8a819be1ca58bb228b95d64bea1d5` directly against `89d5ffbd3cb426d62d36786636a1e0a6e032b8b8`. Exactly ten authorized paths changed; the worktree remained clean.

All five R1 repairs are supported by fixed-source tracing and independent analysis tests:

- **Cleanup:** “retain disk/file charge on cleanup failure.” [Tracked publication cleanup](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-analysis/src/jobs.rs:1006) blocks repeated retries and forget/admission cycles. The Windows refusal test retained 12,288 bytes/two slots until explicit cleanup, preserving competitor bytes.
- **Manifest capacity:** “charge actual retained manifest storage.” [Capacity accounting](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-analysis/src/model.rs:49) charged 12,583,031 retained manifest bytes; queued admission reached 21,243,263 bytes. A valid 128 MiB-capacity string was refused.
- **Final commit:** “no clones/allocations/large drops under final State protocol.” [Borrowed eligibility and acknowledgement](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-analysis/src/jobs.rs:1202) measured zero allocations and zero frees with one and sixteen outputs; retirement remained deferred.
- **Nonfinite output:** refusal occurs before ordinary sanitation at [strict decoding](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-codec/src/decode.rs:472). The real analysis worker rejected encoded Float32 nonfinite output. Float32/Float64 file/byte matrices were source-traced.
- **Case aliases:** “before any writes.” [Artifact checks](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/crates/windfall-analysis/src/artifacts.rs:127) and destination checks rejected aliases while preserving existing PCM, bytes and checksums.

**Independent execution:** all 37 cached analysis tests passed single-threaded after fixed-source inventory, timestamps and 87 dependency fingerprints matched. These also exercised capture/model validation, cancellation, deadlines, partial publication, stale eligibility and retained final assets.

The strict-codec executable links an older codec fingerprint, so its two tests were excluded from independent R2 execution evidence. The reported 145 codec tests, Clippy and formatting remain owner evidence. No Cargo/build commands or source edits occurred.

Session/IPC/UI atomic installation and actual inference remain explicitly deferred; foundation tests do not close those requirements.

The parent subsequently integrated both sources as `44cc4d65` and `2176fc31`. Fresh combined execution passed 37 analysis and 145 codec tests, with two existing ffmpeg-dependent ignores, strict analysis/codec/desktop all-target Clippy, formatting and simulator freshness. This closes the cached codec attribution gap above. The optional predicate duplication is nonblocking. Actual Session/IPC/UI attachment is the next exclusive implementation window; inference remains unavailable.

# Delay family foundation R1

Fixed `6326f883a434af507a1436532d497889fec400e2`, comparison `caeef7794bf6ec79e4f8cc34aa30e2fb74ba991c`.

## Standards

Standards review of `caeef779...6326f883` (one commit): **one contract finding and one design judgement.**

- **P2 — Recoverable preparation failure is missing.** [support.rs:33](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank/support.rs:33) allocates both processors’ histories through `self.data = vec![0.0; maximum + 1]`. Allocator exhaustion aborts rather than returning a preparation error. This conflicts with the supplied fallible-capacity requirement and [roadmap §3](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/docs/IMPLEMENTATION-ROADMAP.md:73): “Every introduced bound needs observable overflow/failure behavior.” The documented 614.4 MB bank makes this material; a byte estimate and proposed host budget cannot guarantee allocation succeeds. Add an off-thread fallible preparation path using fallible reservation, staging replacement histories before publishing them and preserving the previous prepared processor on failure. The existing `Effect` signature can remain unchanged.

- **P3 — Possible Mysterious Name / Primitive Obsession; judgement only.** [echo_bank.rs:230](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank.rs:230) expresses feedback as `back[side] * c[3] * global_fb * c[13]`; processing, readiness and tail queries depend on the positional `Ramps<14>` layout. Named indices or a named control snapshot would make those relationships reviewable without tracing the setter array. Existing filter processors use positional controls, so this is a maintainability suggestion, not a documented-standard breach.

The diff respects file ownership and contains only the two authorized `lib.rs` declarations. No callback allocation/free/lock/wait/IO violation was found.

I independently executed four attributed cached release cases, single-threaded: parameter/default contracts, block-partition/no-op/reset behavior, preparation accounting, and the all-rate allocator/extreme-domain guard. All passed; the guard observed zero callback allocator calls. No edits or builds occurred. The owner’s 128 library tests, full 25-case release run, CPU measurements, Clippy and formatting were not rerun.

This review does not establish registry, host, editor or full E3 parity acceptance.

## Spec

One P2 spec finding in `caeef7794bf6ec79e4f8cc34aa30e2fb74ba991c...6326f883a434af507a1436532d497889fec400e2`:

**Signed polarity controls use an incompatible descriptor unit.** [echo_bank/params.rs:150](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank/params.rs:150) assigns `Gain` to `outputGain` and `nextSend`, including their −1…1 ranges. The [contract](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/docs/DELAY-FAMILY.md:38) says “Signed sends and output levels permit phase inversion.”

Passing these descriptors through the existing parameter formatter/parser loses that meaning: −1 displays as `−∞ dB`, accepting that displayed text yields 0, and typing “−1” yields positive `0.891251`. This is a source-equation trace, not an executed UI failure. Use `None`, matching StereoMatrix’s signed coefficients, while preserving indices. This correction belongs in the owned parameter file; it does not require implementing the deferred editor.

No additional DSP spec findings.

I independently executed the provenance-checked cached release binary: **24 passed, 1 CPU test ignored**, including allocator guards and full-range impulses. Diff checks passed; the worktree remains clean. The 128 library tests, CPU observation, Clippy and formatting results remain owner-reported.

Host admission and saturating tail aggregation remain integration gates. This review does not close either parity row or the remaining E3 scope.

Both P2 repairs are assigned to the same owner: staged fallible preparation with recoverable refusal, and signed unit descriptors. Registry, host admission/tail aggregation and editor integration remain later windows. Neither row is closed.

# Process bridge leaf R4

Fixed `0741117f6a4acf4954ba71950d7e1dfa636a1f4a`, direct parent `e6696dd3720814d515cf7652a6d09a19cd0e7cf4`. All six R1, three R2 and two R3 counterexamples and responses accompanied the new review.

## Standards

Fixed review: `e6696dd3..0741117f`. **No actionable findings within the approved leaf scope.**

## Standards

Zero findings. The twelve changed files remain within the reserved scope. No documented-standard breach or actionable baseline smell was identified; tooling-enforced issues were excluded.

## Spec

Zero findings. Source inspection supports closure of both R3 issues:

- [Load reception](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-plugin-process-bridge/crates/windfall-plugin-host/src/bridge/helper.rs:424) drains bounded progress immediately, sleeps only on `WouldBlock`, and checks deadlines before and after each step. The versioned private bootstrap carries the caller’s remaining budget.
- [Wrapped capture validation](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-plugin-process-bridge/crates/windfall-plugin-host/src/bridge/helper.rs:26) checks the complete container before reply framing in both formats. Refusal follows the recoverable control-error path; VST3 recovery remains mandatory.

The six R1 and three R2 source closures remain supported: whole-owner watchdog aging, note reconciliation, distinct processed epoch, offline failure/cancellation guards, monotonic capture cache and authenticated disclosure. Atomic payload/CAS ownership and the documented **2B** schedule remain intact.

Independent checks confirmed the fixed SHA, direct parent, clean worktree and passing `git diff --check`. **No binaries, tests or builds were executed.** The owner’s 113 tests, RED/GREEN results, allocator guards and strict checks remain owner-reported evidence.

Production activation, engine/document integration, packaged discovery, licensed corpus, native editors, other platforms and hardware acceptance remain open.

Standards: **0 findings**; Spec: **0 findings**.

## Spec

**Spec-only R4: no actionable findings** in fixed `0741117f`, direct parent `e6696dd3`. Source inspection supports closure of all six R1, three R2, and two R3 triggers.

- Load reception now drains successful bounded reads immediately, sleeps only on `WouldBlock`, rejects EOF, and checks deadlines around each step. The private bootstrap carries the remaining startup budget; Hello2 rejects downgrade before Load disclosure.
- Both capture paths check the complete wrapped-state budget before framing. Refusal becomes a recoverable control error, and VST3 reactivation still runs. The native boundary fixtures stream actual payloads and check retained cache, PID, subsequent DSP, and capture.
- Separate `processed_epoch`, conservative acknowledgement gates, held-note recovery, whole-owner watchdog, monotonic cache publication, and offline failure/cancellation guards remain intact. Callback transport and the documented **2B** schedule are unchanged.

Independent evidence: fixed Git blobs, parent/diff/log and regression-source review; `git diff --check` passed. **No native binaries, tests, Cargo, or builds were executed.** The reported 113 passing tests and RED/GREEN results remain owner evidence.

Production routing, engine/document integration, packaged discovery, licensed corpus, native editors, other platforms, and hardware acceptance remain open. This verdict covers the reviewed leaf scope.

Both source axes are clean within the leaf scope. Parent integration uses only incremental commits `27dc8a4a`, `ba61a255`, `e6696dd3`, `0741117f`, never local import merge `02072c81`. Fresh combined native checks are required before the production routing window opens; no desktop isolation or full N4 closure is claimed from leaf review.

# Rack note preview R1

Fixed `ea2f36ddef21527b354581f9aeae007f4a299e2a`, comparison `f9a1d2ee735531feb805df6743800b1fd692e32b`.

## Standards

Reviewed only `f9a1d2ee...ea2f36dd` (one commit, eight files). Standards findings:

- **P2 — Row-view commands bypass the action registry.** [note-preview.tsx:284](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview.tsx:284) constructs labels and handlers directly. Automatic/Steps/Notes have no registered actions, so they lack palette/keymap discoverability; Open also omits the existing action’s configured shortcut. [ARCHITECTURE.md:430](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/docs/ARCHITECTURE.md:430) requires: “Every user action is a named entry in the action registry”; line 431 says every menu label “shows the key an action has in its own scope.” Register scoped view commands and render registry metadata, preserving the captured-target guards.

- **P2 — Thumbnail lacks its own context menu.** [note-preview.tsx:156](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview.tsx:156) adds an actionable thumbnail without an inner `ContextActions`; it inherits the rack’s generic menu, whose entries omit preview navigation/view choices. [ARCHITECTURE.md:440](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/docs/ARCHITECTURE.md:440) requires right-click menus “on every thing with actions of its own” and says “the innermost one opens.” Add a target-specific context menu using the same guarded actions.

- **Optional judgment call — possible Primitive Obsession.** [rack-store.ts:24](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/rack-store.ts:24) exposes `noteViews: Record<string, RackNoteView>` and later decodes `key.split(":").map(Number)`, while the component independently encodes lane identity. Consider accepting typed pattern/channel identity and keeping encoding private. No runtime defect is established here.

The documented findings are source-traced, not runtime reproductions. I executed the fixed diff/log, `diff --check`, and `check-sim`: clean worktree, current assigned 1,881,168-byte WASM, unchanged artifacts and 174 bindings. I did not run UI tests; the 187 passes and browser measurements remain **owner-reported evidence**.

Native desktop smoke/window-focus verification remains the known external gate. No spec-axis verdict or parity completion is asserted.

## Spec

Clean on the spec axis: no actionable defect found in fixed `ea2f36dd` against `f9a1d2ee`, using the requested three-dot diff and one-commit log.

The implementation satisfies “Shows a thumbnail of a channel’s piano roll notes in place of its step buttons.” Source traces confirm:

- [Geometry](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview-geometry.ts:55) preserves timing, duration and pitch, clips finite coordinates, and bounds dense output while including every visible note.
- [Navigation](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview.tsx:44) checks generation, current pattern and channel before selecting the lane and invoking existing `view.pianoRoll`; it performs no document/history edit.
- [View cleanup](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/rack-store.ts:75) clears choices on replacement and deletion. Lane-keyed remounting prevents stale presses and menus. Geometry is memoized; playback writes cursor styles directly.
- The approved existing rack test contains only ten added lines; every original assertion remains.

**My execution:** four existing test files—note-preview, geometry, rack and rack-render—passed **80/80 tests**, with `--maxWorkers=2 --no-cache --configLoader=runner`. These used the assigned actual WASM. Its hash/size, 174 bindings and `check-sim` verified; diff-check passed and the worktree remains clean.

The owner’s 187-test run, browser measurements, screenshots and type/lint checks remain **owner evidence**. My scale checks were jsdom tests, not fresh browser measurements. Native desktop/window-focus smoke and parent composition/acceptance remain open. Parity is not marked done.

The two documented standards findings are assigned to the same owner with a narrow action-registration and innermost-context-menu window. The optional identity-encoding judgment is nonblocking. The clean specification verdict and 80 independently executed cases do not waive the standards findings. Source stays outside root pending the next independent round and composition.

# Timeline R4

Fixed `a8e33f209c3d5d85e3ecd55b876d02bdd5e22119`, direct parent
`59c8f7f3adc0c552fca972b515e5219ddaf24d06`; inherited delivery since
`9b3001f47304167d2cced8dc09fcc8971ca7bf62` was also inspected.

## Standards

**Standards-only R4: changes requested — one inherited rule breach remains.**

Reviewed fixed `a8e33f209c3d5d85e3ecd55b876d02bdd5e22119`, directly above `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, including the original delivery since `9b3001f4`.

**Static Add menu entries still bypass the registry.** [timeline-controls.tsx:346](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/apps/desktop/src/features/playlist/timeline-controls.tsx:346) builds Add meter and four Add marker entries with separate titles and `onClick={() => choose(...)}` handlers. These duplicate actions already registered in [timeline-actions.ts:67](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/apps/desktop/src/features/playlist/timeline-actions.ts:67).

The documented rule says: “Menus, the command palette, context menus and the keymap all read the registry.” [ARCHITECTURE.md:430](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/docs/ARCHITECTURE.md:430). Consequently, these entries omit configured shortcut labels and bypass registry enablement and execution. Render all five through `ActionMenuItem`. The inline-entry exception concerns “the thing clicked”; these are static creation actions.

The prior hydration notification and ID-newtype findings are resolved. Play/Loop/Zoom/Export/Clear now use registry-backed entries. No separate actionable baseline-smell finding or additional recording/State, callback or reserved-seam breach found.

Independent verification was fixed-source inspection only. The owner’s **251 shared-WASM UI tests, 31 native/project/engine tests and strict checks** remain reported provenance. No tests, builds, edits, regeneration or delegation occurred; the worktree remains clean. Combined-root/artifact integration and physical/external-plugin/non-Windows verification remain outstanding.

## Spec

**SPEC R4: changes requested — one inherited P2 finding.**

Reviewed fixed `a8e33f209c3d5d85e3ecd55b876d02bdd5e22119`, parent `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, including inherited delivery paths since `9b3001f47304167d2cced8dc09fcc8971ca7bf62`.

**P2 — Hosted plugins receive incorrect bar origins after unaligned meter changes.** The contract says: “[A change starts a new bar](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/docs/TIMELINE-REGIONS.md:18).” [Processor transport](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-engine/src/processor.rs:712) forwards the current signature and absolute beat position, without the meter segment’s bar origin/index. [CLAP](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-plugin-host/src/clap/processor.rs:202) and [VST3](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-plugin-host/src/vst3/processor.rs:431) consequently calculate bars as though the current signature applied from tick zero.

In the shared fixture, 4/4→7/8 at tick **4001** begins bar **3**, with its downbeat at 4001. Both adapters instead report a bar origin at tick **3360**; CLAP reports zero-based bar number **1**, rather than **2**. Plugins using host bars for synchronization therefore disagree with the project. Carry the segment origin and cumulative bar count into native bar fields, preserving absolute beat/seconds positions, and add an adapter-level regression. **Source-traced; external-plugin behavior was not independently executed.**

The prior chained-cancellation and pattern-signature findings appear closed. R1/R2 guards, recovery, hydration and safe-request limits were source-traced; numeric ID compatibility was executed.

Independent verification: **28 focused tests passed**, single-threaded—10 engine, 11 session, 7 project—after binary hashes, dependency fingerprints, fixed-source hashes, timestamps and inventories matched. The hosted probe covers six engine transport fields, not the derived native bar fields above.

Owner-reported 251 shared-WASM UI passes and strict checks remain provenance. No stale-WASM UI tests, edits, builds, regeneration or delegation occurred. Worktree remains clean at the fixed hash. Combined-root integration and physical/external-plugin/non-Windows behavior remain unverified; explicit staged follow-ups remain open.

The static creation-menu repair is assigned immediately to the same owner.
Native bar-origin propagation has an approved narrow callback transport
contract: optional typed meter-segment origin and cumulative bar index, with
unchanged absolute beat/second positions and legacy scalar defaults. T1 owns
checked immutable meter records and native adapter bar fields; N4 owns runtime
forwarding and a versioned 14-word ABI3 transport codec with verified slot
layout. Neither may edit M1's central registration paths. Earlier guarded
cancellation and typed identity closures are retained; the 28 independent
passes do not waive the bar-origin finding. No timeline source is integrated.

# Delay family E3 R2

Fixed source: `42d58df48e499a9fc4c3997f0382310976bc4415`. Both reviewers received the original brief,
all earlier findings, responses and remaining objections.

## Standards

Standards R2 review of `caeef779…42d58df` and incremental `6326f883…42d58df`: **no documented-standard violations found; one unchanged P3 heuristic remains.**

- **Prior P2 preparation finding resolved.** [Roadmap §3](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/docs/IMPLEMENTATION-ROADMAP.md:73) requires observable overflow/failure behavior. [History staging](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank/support.rs:103) now checks capacity and budgets, reserves fallibly, and completes every history before either processor mutates live state. Refusal preserves audio state and latches a copyable typed status, including through legacy `Effect::prepare`. Staged and replaced histories retire on the calling control thread.

- **P3 — Possible Mysterious Name / Primitive Obsession; judgement only.** [echo_bank.rs:232](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank.rs:232) still expresses feedback as `back[side] * c[3] * global_fb * c[13]`. Processing, readiness and tail queries depend on the positional `Ramps<14>` layout. Named indices or a named snapshot would clarify those relationships. Existing filter processors use positional controls; this remains a nonblocking suggestion, not a documented-rule breach.

Ownership remains intact, with only the two authorized `lib.rs` declarations. Source inspection found no callback allocation/free/lock/wait/IO violation.

I independently executed **two provenance-checked cached release cases**, sequentially and single-threaded: signed-descriptor/JSON/audio regression and parameter/default contracts. Both passed; the signed-audio allocator guard observed zero calls. Preparation fault-injection tests were inspected, not executed. The owner’s 132-library/26-release runs, Clippy, formatting and earlier CPU measurements were not rerun. Diff checks passed; the worktree remains clean. No edits or builds occurred.

This review does not establish host admission, registry/editor acceptance or full E3 parity.

## Spec

**Spec review: no new findings.** Reviewed the full `caeef779…42d58df4` diff and incremental `6326f883…42d58df4` repair.

Both prior P2 findings are resolved:

- **Recoverable preparation:** [staging and reservation](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank/support.rs:103) use checked budgets and fallible allocation before publishing replacement histories. Refusal preserves live audio state and latches typed status, including through legacy `Effect::prepare`.
- **Signed descriptors:** all output/send coefficients now use [`None`](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-delay-family-e3/crates/windfall-dsp/src/echo_bank/params.rs:150), preserving indices, ranges, defaults and JSON.

The declared counts and chain controls agree with the primary [Delay Bank](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Delay%20Bank.htm) and [Multiband Delay](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Multiband%20Delay.htm) references. Deferred controls remain explicitly documented.

I independently ran **five provenance-checked cached release cases**, sequentially and single-threaded: parameter contracts, signed metadata/audio, block/no-op/reset invariance, serial/parallel routing, and sixteen-band impulse/control isolation. All passed; applicable callback allocator guards reported zero calls.

I inspected the private reservation-failure/rollback tests but did not execute them. The owner’s 132-library-test and full 26-release-test results, Clippy and formatting remain owner-reported. CPU measurements remain attributed to `6326f883`.

No edits or builds occurred; HEAD remains fixed and the worktree clean. Host admission, saturating tail aggregation, registry/project/engine integration, drawn editors and remaining E3 work remain open. This establishes no full parity acceptance.

Foundation review is accepted with no documented-standard or specification defect. The unchanged positional-control heuristic is nonblocking. Source remains outside root pending composition and fresh tests; host admission, saturating tails, registry/project/engine integration, drawn editors and the remaining E3 processors stay open.

# Release foundation R2

Fixed source: `477209ec1d3c5f2d7efe5e7147b5551a4bfbe0c1`. Both reviewers received the original brief,
all earlier findings, responses and remaining objections.

## Standards

Standards review of `f7102e7c…477209ec`, including incremental `67191069…477209ec`: **0 documented-standard breaches; 1 nonblocking judgment-call smell.** Both earlier duplication findings are resolved.

Applicable documented sources were [ARCHITECTURE.md](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/docs/ARCHITECTURE.md) and [roadmap R1](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/docs/IMPLEMENTATION-ROADMAP.md:327). No local AGENTS or dedicated standards file was found.

- **Possible Duplicated Code** — [action-provenance.mjs:287](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/scripts/release/action-provenance.mjs:287) and [line 437](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/scripts/release/action-provenance.mjs:437) repeat the workflow action extraction—`Object.values(...jobs).flatMap(...).filter(...).map(...)`—and identical audited-pin assertion. Changes to this normalization require synchronized edits. Extract a private helper used by collection and checking. This is a smell-baseline heuristic, not a documented-rule violation.

Executed: **8/8 offline provenance tests**, syntax checks for all ten Node modules, workflow contract check and local provenance check. The Cargo-dependent release suite and network reproduction were not rerun. The unresolved toolchain license closure remains explicit.

Worktree remains clean at the fixed commit; no edits, builds or publication actions occurred.

## Spec

SPEC review of `f7102e7c…477209ec`, including incremental `67191069…477209ec`: **0 new findings. Prior P1/P2 are resolved by the fixed implementation.**

- **P1:** The requirement “Future bridge/model resources must be an explicit validated packaging contract” is enforced by [source.mjs:225](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/scripts/release/source.mjs:225). A Cargo-free temporary fixture rejected all 15 platform filenames and both alternate base filenames, while preserving the valid base inventory. These match the [pinned Tauri filename/merge contract](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-utils/src/config/parse.rs).

- **P2:** “Untrusted metadata evidence cannot bootstrap ownproof” is now enforced: [dependencies.mjs:457](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-release-engineering-r1/scripts/release/dependencies.mjs:457) reconstructs and compares declarations, notices, edges, features, kinds and predicates. The offline collector also compares supplied Cargo metadata with a fresh locked read. This conclusion is from source tracing; Cargo-dependent regressions were not rerun under this review’s prohibition.

The unresolved `buffers@0.1.1` closure remains explicit and publication remains ineligible. Fresh primary-source reads reproduced four pinned upload-action files, tarball integrity and both distribution matches; the [maintainer’s MIT comment](https://github.com/actions/upload-artifact/blob/cf430e030ddbb5b0abf93d22962f4752f3646cd9/.licensed.yml) is not promoted to a verified grant.

Executed: **11 selected tests passed**, the resource probe, actual lock readers (**538 Cargo / 589 npm entries**), workflow contract and offline evidence checks. The full 56-fixture suite was not rerun. Worktree remains clean at `477209ec`; no edits, Cargo, builds, dispatch or publication occurred. Native/signing/install/updater gates remain open.

Source review is accepted with no documented-standard or specification defect; the action-extraction duplication heuristic is nonblocking. Source remains outside root. The historical buffers@0.1.1 license closure is unresolved and publication stays ineligible. Owner evidence is 55/56 in one full run plus the corrected fixture passing separately, not one fresh full 56-test pass. Native bundles, installation, signing and updater acceptance are unverified.

# Rack note preview R2

Fixed source: `fd62eb055810c31ab7f54d13bd831a48c3b77293`. Both reviewers received the original brief,
all earlier findings, responses and remaining objections.

## Standards

**Standards: clean.** No actionable documented-standard violations or additional smell findings in fixed `fd62eb055810c31ab7f54d13bd831a48c3b77293`. Reviewed both `f9a1d2ee...fd62eb0` and `ea2f36dd...fd62eb0`; the log contains the feature commit and its repair.

Both prior P2 findings are closed by source trace:

- **Registry and shortcuts:** [actions.ts:56](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/actions.ts:56) registers scoped view commands with canonical titles, shortcuts, checked state and disabled reasons. Menus reuse that metadata; the open alias reveals the existing piano action’s configured key. This satisfies [ARCHITECTURE.md:430](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/docs/ARCHITECTURE.md:430): “Every user action is a named entry in the action registry.”
- **Thumbnail context menu:** [note-preview.tsx:219](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview.tsx:219) wraps the actual thumbnail in `ContextActions`, deriving entries from the registry and retaining captured-target guards. This follows [ARCHITECTURE.md:440](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/docs/ARCHITECTURE.md:440): “They nest, and the innermost one opens.”

The optional Primitive Obsession concern is resolved: callers pass typed lane identity, and [rack-store.ts:77](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/rack-store.ts:77) keeps store-key encoding private.

**My execution:** the two preview test files passed **47/47**, using `--maxWorkers=2 --no-cache --configLoader=runner`. UI tests used the assigned actual WASM; geometry tests used pure fixtures. `check-sim`, hash/size, 174-binding inventory and both diff checks passed. Artifacts are unchanged and the worktree remains clean.

The owner’s 201-test run, static checks and browser measurements remain owner evidence. Native desktop/window-focus smoke and parent composition/acceptance remain open. No spec-axis verdict or parity completion is asserted.

## Spec

**Clean on the spec axis:** no actionable defect found in fixed `fd62eb055810c31ab7f54d13bd831a48c3b77293`. Reviewed both `f9a1d2ee...fd62eb05` and incremental `ea2f36dd...fd62eb05`; the repair commit directly follows immutable `ea2f36dd`.

The implementation satisfies “Shows a thumbnail of a channel’s piano roll notes in place of its step buttons.” [Geometry](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview-geometry.ts:56) preserves timing/duration/pitch, clips finite coordinates and bounds dense rendering. Explicit Steps retains the original detail-dot assertions and editing behavior.

Both R1 repairs meet their granted behavioral contract:

- [Registered view actions](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/actions.ts:56) provide canonical titles, shortcuts, checked state and disabled reasons. Palette/keymap execution targets the current selected lane.
- The thumbnail owns an [inner context menu](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview.tsx:219). Captured commands [validate generation, pattern and channel before selection/execution](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-rack-note-preview-t0/apps/desktop/src/features/channel-rack/note-preview-target.ts:98). Navigation/view changes preserve document, history and dirty state.

**My execution:** note-preview, geometry, rack and rack-render passed **94/94 tests**, using `--maxWorkers=2 --no-cache --configLoader=runner`. UI cases used the assigned actual WASM through `SimDocument`; geometry cases are supplemental pure tests. `check-sim`, both diff-checks and artifact verification passed: unchanged 174 bindings and 1,881,168-byte WASM, SHA-256 `9feb181f0a3d6aed64d9aabd26c61cedcf015b1437889bfa5ca7b309d18dde3a`. The worktree remains clean.

The owner’s 201-test run, browser measurements/screenshots and static checks remain **owner evidence**. Native desktop/window-focus smoke and parent composition/acceptance remain open. Parity is not marked done.

Both review axes are clean. Root imported the original feature and incremental repair as 1e90a885 and b4db603c; the original source commits remain immutable. Fresh root UI/static validation is being completed separately. Native desktop/window-focus smoke and the parent's parity decision remain open.

# Timeline R5

Fixed source: `c5448921960ead05d0f0d4846fc627fbcf035fc6`, above metadata prerequisite
`94e168ae` and prior `a8e33f20`. Both reviewers received the complete original
brief, prior findings, responses and unresolved composition gates.

## Standards

**Standards R5 verdict: approve with one P3 judgment-call smell.** No new documented-rule breach found in `a8e33f20...c5448921960ead05d0f0d4846fc627fbcf035fc6`, including relevant inherited delivery paths since `9b3001f4`.

**P3 — Possible Duplicated Code.** [timeline.rs:248](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-project/src/timeline.rs:248) repeats [check.rs:155](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-project/src/check.rs:155): `!(1..=16).contains(&signature.numerator)` and `![2, 4, 8, 16].contains(&signature.denominator)`. Their diagnostics are also duplicated in `MeterMapError::Display`.

Changing a supported signature constraint now requires synchronized edits to document validation and playback preparation. This matches the supplied heuristic, “the same logic shape appears in more than one hunk or file in the change”; it is a maintenance judgment, not a current behavioral defect. Narrow repair: share the typed signature validator and make `time_signature_problem` its string adapter.

The static Add entries now use `ActionMenuItem`. Hydration invalidation and serde-transparent ID newtypes remain repaired. The meter gate precedes the controller’s installation mutation; no distinct additional preparation/retirement breach was identified in this diff.

**Actual checks:** fixed-source, diff, test-source and relevant call-path inspection only. No tests or binaries executed; no builds, edits, regeneration or delegation occurred. The worktree remains clean at the full pinned hash above. Owner-reported **27 native/project/engine tests, 93 shared-WASM UI tests and strict checks** remain provenance.

The acknowledged **P1 whole-composition preparation/retirement-under-guards objection remains open**. Combined artifacts/runtime forwarding, physical/platform verification and explicitly staged follow-ups remain outstanding. Spec was not assessed or reranked.

## Spec

**SPEC R5: changes requested — one P2 finding** at `c5448921960ead05d0f0d4846fc627fbcf035fc6`.

**P2 — Exact downbeats can report the preceding native bar.** [events.rs:148](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-plugin-host/src/events.rs:148) subtracts independently converted beat positions before applying `floor`. In a song extending past tick 3845, use scalar 4/4, add 7/8 at tick **485**, then seek/play at **3845**. The quotient becomes `0.9999999999999999`, rather than 1. Both ABI builders consequently receive bar origin **485**, rather than **3845**; CLAP receives zero-based bar **1**, rather than **2**.

This disagrees with the contract that native bar indices use the “[same shortened-bar conversion as the ruler](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/docs/TIMELINE-REGIONS.md:590).” Use tick-based or rounding-aware boundary arithmetic that preserves genuinely pre-boundary positions, and add concrete adapter regressions using independently converted absolute ticks. **Source-traced with independently evaluated IEEE-754 arithmetic; this trigger was not executed natively.**

**Actual checks:** 27 focused cached tests passed single-threaded: seven host, ten engine timeline, one controller refusal and nine project timeline. Binary/source hashes, dependency fingerprints, timestamps and inventories matched before execution. Inherited cancellation guards and recovery were source-traced.

The owner’s 93 shared-WASM UI passes and strict checks remain provenance; restored artifacts were not executed. The worktree remains clean. No edits, builds, regeneration or delegation occurred.

The supplied whole-composition P1 preparation/retirement objection and deferred N4 forwarding/render-refusal integration remain open. Physical/external-plugin/non-Windows verification and explicitly staged follow-ups remain outstanding.

The standards heuristic is nonblocking. The exact-downbeat P2 is assigned to
the same timeline owner, with native arithmetic and concrete ABI regressions;
public anchor/ABI changes require separate N4 coordination. The fixed producer
is not imported or declared accepted on both axes. The inherited whole-state
preparation/retirement gate remains with the new P1 owner.

# Analysis M1 app R1

Fixed source: `e3cdbb3f7d04f5024350e10e5044b9ba09249890`, directly above reviewed
foundation `f006ce1b`. Both reviewers received the full foundation and app briefs,
all previous findings and responses, and the known native-preparation gate.

## Standards

**Standards R1 verdict: changes required.** One documented violation and one nonblocking smell.

Reviewed pinned `e3cdbb3f7d04f5024350e10e5044b9ba09249890` directly against `f006ce1b6ad8a819be1ca58bb228b95d64bea1d5`.

- **[P2, documented violation] Analysis actions bypass the registry.** [index.tsx:35](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/index.tsx:35) opens Analysis through `DialogTrigger`; import and job operations similarly use direct `onClick` handlers. Selecting an audio clip exposes these operations only through panel buttons: no Analysis entries exist in the fixed registry or startup registration. [ARCHITECTURE.md:430](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/docs/ARCHITECTURE.md:430) requires: “Every user action is a named entry in the action registry.” Register the Analysis entry and commands, use registry-backed controls, and invalidate their enabled state when panel context changes.

- **[P3, judgment call: possible Duplicated Code] Terminal-status policy is repeated.** [index.tsx:164](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/index.tsx:164) and [retire.ts:3](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/retire.ts:3) independently declare `["cancelled", "failed", "consumed"]`. The supplied smell baseline describes “the same logic shape” appearing in multiple changed files. A policy update could make button eligibility and retirement diverge. Share a typed terminal-status predicate.

**Actual checks:** Read-only fixed-source inspection covered all 22 changed paths and relevant Session/controller/store/registry call paths. HEAD and direct parent matched; the worktree remained clean. Fixed diff checking passed. Lockfile bytes matched the parent plus exactly the authorized desktop dependency reference; the model fixture SHA matched.

No native binaries, UI, tests or Cargo commands were executed. Owner-reported passes remain provenance. The known whole-composition native-preparation gate remains unresolved; this Standards review does not clear it or assess Spec acceptance.

## Spec

**Spec R1 verdict: changes required.** Reviewed pinned `e3cdbb3f7d04f5024350e10e5044b9ba09249890` against direct parent `f006ce1b`. Two new app defects are source-traced:

- **[P2] Windows junction retargeting bypasses final source validation.** [analysis_jobs.rs:952](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src-tauri/src/session/analysis_jobs.rs:952). Retarget a source-path junction after the hash/identity check but before final commit. `source_file` protects the followed file, while the final predicate checks unchanged path strings and loaded audio identity. Apply can therefore succeed although the stored source path now resolves to different content. This violates the required “source key/path/bindingdigest/AudioIdentity/content” eligibility. Reject reparse components or retain their namespace authority through commit.

- **[P2] Dismissing a panel loses recovery controls after cleanup refusal.** [retire.ts:14](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/retire.ts:14), [index.tsx:107](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/index.tsx:107). If cancellation leaves refused cleanup, `analysisForget` rejects immediately. Unmount only logs that rejection; reopening starts without the retained job or retry controls. The rejection also bypasses the identity-bearing timeout message. This contradicts “A cleanup refusal … reports the retained job identity.” Preserve bounded recovery controls across dismissal and include the job ID in every retirement refusal.

**The known P1 composition gate remains open.** [analysis_jobs.rs:995](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src-tauri/src/session/analysis_jobs.rs:995) reaches `Controller::set_plan → PlanState::build → factory.effect/instrument` under final guards, after Document dispatch. The requirement is “fallible Controller preparation OFF-State.” Prepare and validate a guaranteed installation before dispatch, with native retirement deferred outside guards.

**Actual checks:** all **14 cached Session tests passed**, single-threaded, after 259 pinned workspace source hashes, source timestamps, 435 dependency fingerprints and the selected test inventory matched. Diff checking passed; the worktree remained clean.

The two new failure cases were **not executed**. UI/WASM, IPC exports, Clippy and frontend passes remain owner evidence. Unavailable production inference is explicitly deferred and is not a finding.

All three new documented/spec defects are assigned to the same implementation
owner. The registry repair has a narrow canonical startup import/registration;
namespace authority and recovery repairs remain in the owned analysis modules
and tests. Static Plan/sampler preparation is explicitly insufficient for the
unresolved native-ready installation gate. No production inference or app/root
composition acceptance is claimed.

# Subsequent root composition and CI

E3 and release foundations are imported through `7f8c3112` and `6d077380`.
Fresh parent checks pass 132 DSP library tests, 26 release-profile delay cases,
strict DSP Clippy, complete 174-binding comparison and an actual rebuilt
1,880,070-byte simulator. The matching simulator passes 199 effect/document UI
cases and nine automation cases. The release suite passes all 56 cases in one
parent invocation; workflow/provenance checks pass while unresolved historical
tool license closure keeps publication ineligible. No workflow was dispatched.

Rack CI follow-up `87ebcb34` is imported as `d332801b`. It awaits asynchronous
menu readiness and selects explicit Steps in the existing synth case after
asserting the actual automatic thumbnail. All original behavioral assertions
remain active. The owner executed 235 cases on its assigned artifact; fresh
parent validation also passed all 235 cases across eight files on the newly
rebuilt root simulator. App typechecking and scoped test lint passed. CI at
`43e42983` passed Windows Rust, all bindings, parity and freshness, but failed
those two UI cases, eight macOS VST3 unsupported-entry cases and Ubuntu's native
realtime process with SIGSEGV. No app job succeeded. Native portability is
assigned to a separate isolated owner; tests are not disabled to claim success.

T8 foundation `421bf5eb` passed separate R1 source reviews and is imported as
`a0ec6372`. Fresh parent process 40292 passed all 16 analyzer cases, with the
existing optional throughput case ignored, strict engine all-target Clippy,
workspace formatting, complete 174-binding comparison, simulator freshness
and whitespace checks. The native command log is
`C:/Temp/windfall-root-t8-foundation-verification.log`. Actual engine tap hooks,
native subscriptions and analyzer/EQ/mixer views remain open.

The Linux portability owner reproduced the unchanged realtime SIGSEGV natively
and traced it to a Rust TLS cleanup callback in an already-unloaded fixture.
Diagnostic code retention passes all original assertions but is not the fix.
N4 froze fixture classes 0–20 at `71dbec11` and released only creator identity,
assignment, two owner comparisons and a private module declaration to the
portability owner. Native identity, wrong-owner and unload-before-thread-exit
regressions are being implemented in that serialized window. Original Ubuntu
CI confirmation and macOS CFBundle support remain open.

Timeline repair `3d2475c7` directly follows `c5448921`. Owner-compiled RED/GREEN
covers both concrete native ABI downbeats at independent anchor 485/960 and
position 3845/960; 33 focused host/engine/project tests passed. Fresh independent
R6 Standards and Spec tasks receive all earlier findings and responses. The
public anchor interface and bridge layout are unchanged by this repair.
Preparation proposal `07cdb24b` is doc-only and has a new independent design
audit. Existing preparation source has not been granted or repaired. M1's
registry, namespace authority and retained-job recovery repairs are underway
with the original owner; the global native-ready installation gate stays open.

# Analyzer tap foundation R1

Fixed source: `421bf5ebf2406d660751eb8cbd670527e09b13d7`, direct parent
`9c909879d8f93022085addf68f069d52cfe945b2`. Both reviewers received the original
bounded foundation brief and explicit product integration gates.

## Standards

**Standards verdict:** no documented-rule breaches found; one P3 smell heuristic. No standards blocker identified in this bounded foundation.

**P3 — possible Mysterious Name:** [analyzer_taps.rs:1043](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analyzer-taps-t8/crates/windfall-engine/tests/analyzer_taps.rs:1043) prints `audio_seconds_per_cpu_second`, although its denominator is `Instant` wall-clock elapsed time. Baseline: “name doesn't reveal what it does or holds.” Running the ignored release case produces a misleading CPU-time label when the log is read independently of the documentation. Rename it to `audio_seconds_per_elapsed_second` and update the corresponding documentation sentence. This is a heuristic, not a hard standards violation.

**Actual checks:** resolved full base `9c909879d8f93022085addf68f069d52cfe945b2`; confirmed checkpoint `421bf5ebf2406d660751eb8cbd670527e09b13d7` directly descends from it. Reviewed the nonempty eight-path diff and commit list; all eight working-file blob hashes match the checkpoint. Fixed-diff whitespace check passed; worktree remains clean.

Inspected queue ownership, staged replacement/retirement, cancellation/join paths, snapshot-pool conservation, calibration, and production-interface tests. Checked primary [rtrb](https://docs.rs/rtrb/0.4.0/rtrb/), [RustFFT](https://docs.rs/rustfft/6.4.1/rustfft/trait.Fft.html), and [NIST](https://tf.nist.gov/phase/Properties/ten.htm) documentation on 2026-10-08.

**Execution evidence:** no tests, Cargo, Clippy, builds, or cached binaries executed. The reported 16 passing tests and release measurement remain owner-reported evidence. Full T8 product acceptance remains open.

## Spec

**Spec findings: None.** I found no actionable defect within the promised foundation after inspecting queue ownership, replacement/retirement, cancellation, snapshot conservation, gap clearing, invalid PCM, clocks, accounting, and FFT normalization.

**Actual checks:** Resolved base `9c909879d8f93022085addf68f069d52cfe945b2` and fixed commit `421bf5ebf2406d660751eb8cbd670527e09b13d7`; confirmed direct ancestry, one commit, eight permitted paths, 2,976 additions, clean worktree, and passing `git diff --check`. The lockfile change adds only the engine’s existing RustFFT dependency reference. Independently checked the official [RustFFT scratch contract](https://docs.rs/rustfft/6.4.1/rustfft/trait.Fft.html) and [rtrb ownership guarantees](https://docs.rs/rtrb/0.4.0/rtrb/).

**Execution evidence:** No builds, Cargo commands, or tests executed. The 16 passing tests, zero callback heap operations, Clippy/rustfmt results, and 584.586 ms throughput remain owner-reported evidence.

**Verdict:** Spec source review passes for this bounded foundation. Actual engine hooks, native subscriptions/source-selection verification, distinct views, and the EQ live-spectrum audit remain open.

Both source axes accept the bounded foundation. The optional measurement-label
heuristic is assigned to the same owner in a source-only follow-up. Root import
`a0ec6372` has separate fresh 16-case native verification and strict checks;
reviewers did not execute those tests. No product tap, view or parity closure is
inferred from either source verdict.
