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

# Timeline R6

Fixed source: `3d2475c74fba88fcb4314f7ebc614708e76e9e6f`, direct parent
`c5448921960ead05d0f0d4846fc627fbcf035fc6`. Fresh reviewers received the original
bounded timeline brief, all five prior rounds and the retained composition gates.

## Standards

Fixed HEAD: `3d2475c74fba88fcb4314f7ebc614708e76e9e6f`

Verified parent: `c5448921960ead05d0f0d4846fc627fbcf035fc6`

**Findings:** No new hard documented-rule breaches or actionable optional baseline smells in the fixed R6 diff. The shared host helper uses bounded arithmetic and observable refusal, with no source-visible allocation/free, lock, wait, or I/O. Both native ABI builders consume it; public anchor fields and legacy scalar behavior remain unchanged.

The acknowledged whole-composition P1 preparation/retirement-under-guards objection remains open. R6 neither changes nor resolves that path.

**Performed checks/provenance:** Verified HEAD, parent, single-commit log, complete six-file diff, clean worktree before and after, and passing `git diff --check`. Read architecture, plan, relevant roadmap contracts, timeline evidence, checked project meter conversion, engine forwarding, native builders, and added assertions. All five changed Rust file hashes match the documented source hashes.

Source review only: no tests, binaries, Cargo/build/generation, or UI runs. The documented 33 passing tests remain owner-reported evidence. Worktree unchanged.

## Spec

Reviewed fixed HEAD `3d2475c74fba88fcb4314f7ebc614708e76e9e6f`, parent `c5448921960ead05d0f0d4846fc627fbcf035fc6`.

**R6 findings:** No new spec failures found in the fixed diff. Source tracing supports the R5 repair: canonical absolute tick boundaries distinguish `next_down`, exact and `next_up`; noncanonical anchors use fused boundaries with a final half-open check. Tick/index overflow and collapsed boundaries refuse. Scalar negative bars and `Transport::advance` retain their prior behavior. Engine Song anchors, Pattern scalar transport, and concrete CLAP/VST3 field construction remain consistent.

**Inherited P1 remains unresolved:** With an attached stream, valid project publication still calls `PlanState::build` while holding the Controller guard at [controller.rs:260](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/crates/windfall-engine/src/controller.rs:260), reached through document-state publication at [edit.rs:317](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-timeline-regions/apps/desktop/src-tauri/src/session/edit.rs:317). This retains the objection against the roadmap requirement: “Build processors, FFT plans, samples, shared memory and routing plans away from audio/document locks.” Meter refusal gates do not resolve it.

**Checks/provenance:** Verified HEAD, parent, commit log, fixed diff, architecture/plan/roadmap, review history, checked meter producer, host arithmetic and native adapters. `git diff --check` passed; the worktree remains clean and unchanged.

Source review only: no Cargo/build/generation/UI or binary execution. Inspected available executable/source hashes and fingerprint metadata, but did not establish the complete dependency provenance required for execution. Owner-reported tests are not reviewer-executed evidence. N4 and combined-root acceptance remain separate gates.

Both source axes are clean for the incremental native-bar repair. The owner’s
33 focused host/engine/project passes remain distinct from source-only reviewer
checks. T1 is still outside root; N4 ABI3/runtime forwarding and whole-state
preparation must compose before root native/artifact acceptance. No additional
parity or platform claim follows from the source verdicts.

# Project preparation P1 design R1

Proposal-only pin: `07cdb24b5628ae79ae56c7933df26a050abfe1ce`, direct parent
`6d077380`. The independent design audit received the original off-lock/native
readiness and deferred-retirement contract, T1/N4/M1/utility constraints, and
four explicit answers. It did not execute tests or grant source implementation.

**Recommendation: hold the shared source grant for `07cdb24b`.** The shared-row clarification is feasible in principle, but four concrete design/grant conditions remain unresolved. These concern the proposed implementation, not shipped failures.

1. **Separate physical storage from generation identities.** Ten restored slots can require 20 physical rows but 30 possible identities: outgoing `g0`, captured active `g1`, fresh `g2`. The clarification addresses that arithmetic; the immutable proposal still calls `U` the unique-generation union. Specify the row-to-alternative mapping and how subsequent publications resolve an earlier reservation without treating provisional choice as actual native evidence. See [proposal:406–448](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:406), [active indexing](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/plan.rs:425), and [Ledger identity](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/state.rs:366). **Proof needed:** queue successive revisions, advance through outgoing completion and first audible insertion after lease acceptance, preserve active indexes, and refuse any required third source before Document commit.

2. **Define selected-only history semantics, including inherited capacity.** Processing selected stages alone is insufficient. Existing history logic uses physical `stages.is_empty()`, suffix lengths, prefix order, and retained-stage scans. Inactive union entries could therefore suppress scalar promotion, choose the wrong predecessor, or double-count a suffix. A late `g0 → g1` choice during an unfinished tap transition must use an ordered selected view on both sides of history transfer. Each destination must also preserve inherited ring/tap capacity; [tap transfer currently clamps](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-dsp/src/blocks/tap_crossfade.rs:117). See [proposal:436–482](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:436) and [Compensation history](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/rack.rs:1012). **Proof needed:** explicit history-selection rules plus unchanged R3 short-history/reference-switch and R4/R5 cancellation/continuity cases, with adoption delayed after acceptance and zero callback allocation/free.

3. **The four-operation interface lacks publication intent.** `prepare(project, pool)` cannot distinguish an ordinary edit from New/Open or receive its saved transport patch, yet `install()` must publish the appropriate reserved envelope. Transport requests may change while preparation stalls. Specify one owned intent input, bind current sequence-dependent fields before issuing the lease, and preserve replacement ordering without recursive Controller calls. See [proposed interface](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:64), [message reservation](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:508), and [existing replacement publication](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/apps/desktop/src-tauri/src/session/files.rs:461). **Proof needed:** late play/stop/seek, pattern removal, full queues, and consumer interleaving between pushes; refusal leaves musical state unchanged.

4. **The requested grant omits the fallible device adapter.** Production still obtains a `Processor` through infallible [device.rs:724](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/device.rs:724). The proposal requires `try_attach`, but its [engine file window](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:674) omits `device.rs`. Add a serialized adapter window or name its owner. **Proof needed:** constructor error, failed CPAL attempt, same-rate retry, suspend/reopen, and full-ring/backlog retirement, including last-reference destructors checking that all relevant guards are free.

I found no additional design blocker in borrowed refusal ownership or the private `None → Unavailable` / `Some → Err` distinction. Poison/unwind and last-reference destruction still require integration evidence; the intended guarantees alone do not establish them.

Read-only audit completed at the fixed pin. No files changed, imports, builds, tests, or nested agents.

The same preparation owner has a doc-only incremental response window for all
four conditions. Publication preserves the existing serial callback semantics;
queue capacity or a batched tail does not guarantee one-callback transaction
processing. Source construction, device adapter and processor changes remain
ungranted until the revised interface and ownership proof are reviewed.

# M1 app repair R2 and new CI observation

Immutable app repair `9243f20f` directly follows `e3cdbb3f`. Owner execution
reports 20 desktop cases (16 Session, one namespace-cap and three existing
slicer), 11 IPC checks and 41 UI cases. The namespace repair was compiled
against a real Windows junction retarget, and its ordinary-parent test rejected
write/rename while handles lived and permitted rename after retirement. Retained
recovery controls survive dismissal, selection and actual project replacement;
no restored entry gains review/apply authority. Both fresh App R2 review tasks
have the original foundation/application briefs, all findings and responses.
These results are owner evidence pending independent review and root composition.

Standards R2 subsequently found one remaining documented lifecycle breach:
project replacement invalidates the registry but leaves the old dialog target
and source capture alive, reopening a stale panel and blocking a new open action.
The trigger is source-traced, not a proven stale native Apply. The same owner
has the report; `9243f20f` remains frozen while Spec R2 completes. Cleanup
recovery identities must survive independently of that discarded dialog target.

New CI at pushed `290f0313`, run 37756198700, failed macOS at analysis
`r1_high_capacity_valid_manifest_is_charged_queued_running_and_ready`:
`running_heap >= retained` failed, with 29 other analysis cases passing. This
run did not reach the earlier VST3 failure; those observations are separate.
The owner is diagnosing whole-process heap measurement versus concurrent test
lifetimes in excluded diagnostics while `9243f20f` stays frozen. No assertion
bound, test or source accounting policy is weakened to claim success. Raw log:
`C:/Temp/windfall-290-macos-ci.log`.

The same push also produced release-workflow validation failure 37756196886
with zero jobs/check-runs; no development candidate was dispatched or built.
The local workflow contract/full fixture suite had passed, so the release owner
is diagnosing missing GitHub parser/semantic validation coverage in its narrow
workflow/checker/test window. License, signing, resource and publication gates
remain intact. The root source batch remains committed and the full project
remains active.

# M1 app R2 independent axes

Pin `9243f20f4db79a78e9bdac75fb309d405d855ef7`, direct parent `e3cdbb3f`. Both reports are source-only; prior owner execution is separate. The same owner has a narrow incremental repair window for both triggers.

## Standards

Reviewed **`9243f20f4db79a78e9bdac75fb309d405d855ef7`**, direct parent **`e3cdbb3f7d04f5024350e10e5044b9ba09249890`**. Standards axis, source-only.

**One P2 documented breach:** project replacement retains the old dialog capture. In [actions.ts:284](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src/features/analysis/actions.ts:284), `registerAnalysisActions` handles replacement only with `registry.invalidate()`. It leaves `useAnalysisDialog.target` holding the previous generation, clip ID and source capture. Consequently, inspector remount reopens the stale dialog, while `analysis.open` remains disabled as “Analysis is already open.”

This violates [ARCHITECTURE.md:449](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/docs/ARCHITECTURE.md:449): “anything kept by id outside the project … is dropped when another project takes the place of the open one.” Clear the dialog target/capture on replacement while preserving the intentionally process-local cleanup records. This trigger was source-traced, **not executed**; it does not demonstrate stale native Apply.

No additional documented breaches identified. Source inspection supports actual startup registration/disposal, registry invalidation, retained-ID recovery routing, removal only after native forget, and namespace-handle retirement outside final guards. No optional smell findings raised.

The inherited **global P1 engine-preparation/installation objection remains open**, explicitly documented and unchanged by this increment.

**Performed checks and provenance:** verified checkout, full HEAD/direct-parent SHAs, expected single-commit log, requested diff, architecture/plan/roadmap, full Analysis/AppR1 documentation, retained application seams and foundation89/f006 contracts. `git diff --check` passed; foundation and codec sources match f006. Worktree was clean before and after.

No source/git mutations, imports, builds, generation, UI tests, native executables, WASM execution or nested agents. Owner-reported **20 desktop / 11 IPC / 41 UI** checks remain owner evidence; none were independently rerun here.

## Spec

Reviewed **`9243f20f4db79a78e9bdac75fb309d405d855ef7`**, verified as the direct child of `e3cdbb3f7d04f5024350e10e5044b9ba09249890`. **Source-only review.**

**P2 — Remappable drive letters bypass the namespace guard.** [source_namespace.rs:93](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-analysis-jobs-m1/apps/desktop/src-tauri/src/session/analysis_jobs/source_namespace.rs:93) accepts every Disk/VerbatimDisk prefix and protects filesystem directories, without establishing authority over the drive mapping. This leaves the requirement to pin namespace “components from root to leaf” partial.

Concrete trigger: capture through a substituted `Z:\source.wav`, then remap `Z:` to another ordinary directory after apply’s hash/identity check, at `analysis:prepared`. Retained handles still protect the old objects; the final canonical check compares unchanged document metadata and loaded AudioIdentity, allowing dispatch despite the pathname resolving elsewhere. Windows drive mappings are mutable links in the object namespace, separate from filesystem sharing protection. [Microsoft’s DefineDosDevice documentation](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-definedosdevicew) supports this distinction. Refuse remappable aliases or establish authority over their resolved namespace. **This is a source/platform-contract inference; I did not execute the trigger.**

No additional concrete registry or recovery defect found by source tracing. Reservations precede submit; removal follows successful native forget; recovery commands do not restore apply authority. Documented conservative input refusals are not findings.

**Known global P1 remains open:** final publication still reaches `set_plan → PlanState::build` after Document dispatch under guards. This increment documents that limitation and does not repair it or establish full M1 closure.

**Performed checks/provenance:** read-only SHA/parent/log/diff verification, baseline/foundation contract inspection, requested documentation and changed-path tracing, `git diff --check`, and initial/final clean-worktree checks. Confirmed reserved foundation/codec/IPC/Controller/State/edit/dependency files are unchanged. No binaries, WASM, builds, generation, UI tests, imports, mutations, or nested agents ran. Owner execution counts remain owner evidence.

The dialog capture must be discarded on replacement while cleanup records remain recoverable. The drive-mapping trigger requires actual Windows reproduction and authoritative namespace policy; repeated path queries alone do not pin a mutable mapping. No Controller/native readiness authority follows from this review.

# Native fixture and ABI3 prerequisite review R1

Pin `1fa5a16d...0c1a1664`, exactly fixture increment `71dbec11` and ABI3/read-only metadata increment `0c1a1664`. Neither activates the production bridge. Both axes received the original N4 contract, all six R1, three R2 and two R3 findings/responses and unresolved preparation/T1 obligations.

## Standards

Source inspection only: reviewed exactly `1fa5a16d...0c1a1664` and its two commits. I ran no builds or tests; reported pass counts remain owner evidence.

**Documented standards:** No breaches found in the changed hunks against pinned `docs/ARCHITECTURE.md`, including realtime allocation/locking/blocking and ownership rules. No new material leaf regression identified.

**Optional baseline heuristics:**

- **P3 — Possible Data Clumps / Primitive Obsession**, `crates/windfall-plugin-host/src/bridge/adapter.rs:81,149–154`: both getters expose `(u64, u64, u64)`, although collection intent and completed DSP proof have different meanings. Named, allocation-free `Copy` structs would identify epoch, sequence and desired/processed generation and reduce accidental interchange. This concerns the local API; explicit primitive wire words remain appropriate.
- **P3 — Possible Duplicated Code**, `crates/windfall-plugin-host/test-plugins/src/vst3.rs:74–75,186–187`: the new class names are duplicated between `getClassInfo` and `getClassInfo2`, while class count `21` is repeated across admission checks. This extends an existing maintenance smell. A shared immutable descriptor table could supply names and count while preserving class IDs and behavior.

The acknowledged P1 concerning native construction/retirement under State/controller guards remains unresolved and outside this leaf. This review does not accept full N4 or its external verification gates.

## Spec

**Source inspection — SPEC:** No new concrete findings in `1fa5a16d...0c1a1664` (exactly `71dbec11` and `0c1a1664`).

The source matches the bounded leaf contract:

- ABI3 transport occupies words 16–29, epoch 30–31, and reply identity 32–39. Anchor validation and ABI1/2 rejection match `docs/plugins/process-bridge.md:550–560`.
- Desktop forwarding preserves the optional anchor and absolute transport fields.
- `collection_frontier()` is read-only. `completed_proof()` is populated after matching successful output collection, retains earlier proof across unknown output, and clears on normal reset, matching the rule at `docs/plugins/process-bridge.md:562–568`.
- Class19 returns native processing failure at gain ≥0.75. Class20 applies the 0.625→0.375 deactivation edit while retaining the implemented 37-frame delay. Earlier class predicates remain unchanged.

The acknowledged preparation/retirement-under-guards P1 remains unresolved. This review does not close full N4 or its production, editor, installer, device, licensed-corpus, or other-platform gates.

No reviewer builds or tests were run; the reported test results remain owner evidence.

Optional getter tuple and fixture-table smells do not block this leaf. Production draft `19f6ddc2` is separately frozen and under new independent review. It remains outside root; P1 and checked meter/render composition remain activation gates.

# Ordered timeline and prerequisite composition

Root now contains T1's immutable source sequence `3ba8b13c`, `401939d4`, `e9febc2c`, `ed4ad871`, `e0809d0e`, `59c8f7f3`, `a8e33f20`, `94e168ae`, `c5448921`, `3d2475c7`, followed by only N4 `71dbec11` and `0c1a1664`. Root source pin: `c0fcabea7b369bfcc86ca31146d198c8ffc1d496`. Duplicate metadata integration `1fa5a16d` and N4 local merge `02072c81` were not imported.

One VST3 conflict retained the entire existing R4 point-refusal/readback/recovery regression and added T1's separate concrete ProcessContext meter tests. Existing parameter/event clears and drop reporting remain. Other imports applied cleanly. Native checks and matching generated artifacts/UI remain required before push; no parity row or P1 gate closes through composition alone.

The first root native run passed all FLP targets, but MIDI property round trips failed two cases: an old single-signature oracle and a real second-trip end-duration divergence. The same timeline owner has a MIDI-only incremental repair window retaining all note, tempo, history and byte-idempotence checks. The generated failing seed was retained outside source in `C:/Temp/windfall-t1-root-midi-round-trip-regressions.txt`; the automatic source seed change was restored. Independent engine/host checks continue.

# Completed CI observation at pushed 290f0313

Actual [run 37756198700](https://github.com/erivgout/windfall/actions/runs/37756198700) is terminal. UI passed **165 files / 2577 tests**; all three binding jobs, WASM freshness and parity passed. The app job was skipped because Rust jobs failed.

- macOS: analysis high-capacity heap test failed at line 514, `running_heap >= retained`; 29 passed / one failed / zero ignored.
- Ubuntu: the same case failed its queued heap assertion at line 523, also 29 passed / one failed / zero ignored. Neither reached the previously observed VST3 failures.
- Windows: analyzer quota tests failed `InstanceLimit` at line 709 and installation count two instead of one at line 635; 14 passed / two failed / one existing optional timing ignore.

Raw logs: `C:/Temp/windfall-290-{macos,ubuntu,windows,ui}-ci.log`. These are distinct from earlier serial local passes. M1's controlled unrelated 16 MiB retirement reproduced a false global heap delta while manifest buffers and correct charges remained live. Its one-file test-only subprocess isolation checkpoint `760568ca` preserves every original numerical assertion and is under independent review. T8 has a diagnosis-only window for controlled concurrent quota contention; no quota/assertion/CI scheduling policy has been weakened.

Release validation [37756196886](https://github.com/erivgout/windfall/actions/runs/37756196886) had zero jobs. Job-level `env` uses `${{ runner.temp }}`, outside the [official context availability](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability) for that key. The exact remote parser message is unavailable: this is a concrete documented violation and likely cause, not a quoted GitHub diagnosis. The release owner has a narrow workflow/checker repair window. No candidate was dispatched or published.

# Analysis heap-test isolation review R1

Pin `760568ca210035364f4cf2e6143bec63de59d908`, direct parent `9243f20f`. One path, 31 additions. Both source reviewers received the original quota/retention/final-seam contract, remote failures, controlled diagnostics and exact narrow grant.

## Standards

**Standards: 0 findings.** No documented breach of `docs/ARCHITECTURE.md` or actionable code-smell finding.

**Spec: 0 findings.** The helper at `crates/windfall-analysis/tests/analysis_jobs.rs:486` and guards at lines 511 and 1494 satisfy the authorized test-local isolation repair:

- Both literal `--exact` filters match one unconditional, nonignored top-level test in the same executable, so the pinned source introduces no zero-test success path.
- The case-specific child marker terminates recursion. A wrong-case inherited marker triggers isolation with the correct marker.
- Child output is captured and forwarded before checking exit status. Assertion failure, abnormal termination or launch failure fails the parent.
- `current_exe()` and separate command arguments avoid shell quoting assumptions on Windows and Unix. Serialization applies only inside each child; native worker execution remains intact.

Performed source checks: verified `760568ca` has direct parent `9243f20f`; inspected the exact diff, allocator, fixture, test bodies, Cargo harness configuration, architecture contract and baseline `ANALYSIS-JOBS.md`. The diff contains one changed path and 31 additions. Removing precisely the helper and two guards **in memory reproduced the original 66,309-byte source exactly**, preserving every numerical assertion and production call, including the 24 MiB cap, manifest capacities, retirement and one/sixteen-output zero-allocation/free checks.

No tests or builds were executed. Reported suite, fault-injection, Clippy and formatting results remain owner evidence; this review does not establish macOS/Linux runtime behavior or close the acknowledged P1/App R2 issues.

## Spec

**Standards: 0 findings. Spec: 0 findings.** No actionable defect found in the pinned, source-only repair.

Performed checks against immutable `9243f20…760568ca`:

- Confirmed the direct parent and exact scope: one file, 31 additions, zero deletions.
- At `analysis_jobs.rs:486`, the child uses the current executable directly. Its case-specific environment marker prevents recursion and wrong-case bypass without changing the parent environment.
- At lines 511 and 1494, both exact filters match top-level, unignored tests using Cargo’s default harness. No zero-test success path was identified at this revision.
- The parent captures and prints both output streams, then requires child success. Spawn errors and unsuccessful exits fail the parent.
- Programmatically removing the helper and two guards restored **the exact base Git blob**, `6fbe5dc66e3325c8717ea7e8de89870fbd0e763a`. Every original assertion and production call remains unchanged, including capacities, the 24 MiB cap, retirement, and one/sixteen-output allocation/free guards.
- Reviewed `docs/ARCHITECTURE.md`, base `docs/ANALYSIS-JOBS.md`, fixture provenance, Cargo configuration, and CI. Isolation affects only the two child cases; native worker execution remains intact.

Portability and failure propagation are source conclusions. I ran no tests, builds, installs, edits, or nested agents. Owner-reported execution evidence was not independently reproduced, and exact macOS/Linux schedules remain unobserved. The acknowledged P1 and App R2 gates remain outside this repair.

No numerical assertion or production code changed. Owner default-parallel 30-case and deliberate child-failure execution is distinct from source-only reviewer evidence. Corrected remote macOS/Linux behavior remains pending fresh CI.

# Production bridge draft R1

Frozen `19f6ddc2f866ea3cdcdc19a106f802019796bb7a`, direct parent `0c1a1664`. Both axes received the original N4 and all repair context. This draft remains outside root, gated by P1 and full meter/render precheck composition.

## Standards

Standards review of `0c1a1664...19f6ddc2`: **0 new documented-rule violations; 2 optional P3 concerns.** Locations refer to the pinned revision.

- **P3 — possible Duplicated Code:** `apps/desktop/src-tauri/src/plugins/runtime.rs:1762,1888`. Effect and instrument factories repeat bridge construction, offline-error latching and manager-error recording. Extract that shared preparation/result handling; retain the distinct trait-object conversions.
- **P3 — possible Feature Envy:** `plugins/runtime.rs:104` and `plugins/bridge.rs:291,306`. The new bridge requires exposing eight `ParameterControl` fields and directly implements reset/acknowledgement atomic writes. Narrow methods on `ParameterControl` would keep its generation-publication rules together.

These are heuristic judgments, not breaches of `docs/ARCHITECTURE.md`. Inspection was read-only, using immutable Git objects; no builds or tests were executed.

The acknowledged guarded-construction/retirement P1 remains unaccepted. This review does not approve production activation, close full N4, or waive the separate T1 composition precondition.

## Spec

**SPEC: one new P1 finding** in frozen range `0c1a1664…19f6ddc2`.

**P1 — Capture can revert settled parameters to launch-time values.** In [bridge.rs:88](C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src-tauri/src/plugins/bridge.rs:88), capture clones the original parameter specs and updates values only for unsettled/pending controls.

Concrete trigger: launch at gain **0.5**, commit and receive COMPLETE acknowledgement for **0.625**, then queue an unprocessed note. Capture now has `desired_generation > processed_generation`, but the settled gain’s transmitted spec remains **0.5**. The helper’s reconciliation path consequently returns **0.5** as CLAP companion metadata ([helper.rs:310](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-plugin-host/src/bridge/helper.rs:310)); VST3 can additionally write **0.5** into native state ([helper.rs:330](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-plugin-host/src/bridge/helper.rs:330)). This can corrupt saved parameters/state despite gain having been acknowledged.

Requirement: [process-bridge.md:567](C:/Users/ewhee/.t3/projects/windfall/docs/plugins/process-bridge.md:567) states, “Capture retains committed pending intent separately from actual native bytes and inactive reconciliation.” Initialize every transmitted spec from its current control snapshot before applying pending-document overrides.

This is a source trace, not an executed reproduction. The acknowledged preparation/retirement P1 and missing T1 composition remain open; this review does not accept production activation or full N4.

The capture finding is source-traced, not reviewer-executed. The same owner has a narrow bridge-capture/appended-native-regression/docs repair window. Transmitted specs must use current control values before matching pending-document overrides. No fixture, state ABI, native-thread, engine or P1 source grant follows.

# Invalid-meter and owner-metadata composition

The unchanged new Controller refusal regression failed on root: provider-identity snapshots increased from three to five while constructors/process/drop counts stayed unchanged. Accepted utility R5/R6 snapshots native-owner metadata during Plan compilation; the isolated timeline branch lacks that hook. Root's three-line integration guard snapshots native metadata only when typed `Plan.meters` is valid. Invalid plans retain their exact error and publication refusal; valid owner/revision/generation behavior is unchanged. No assertion was weakened. Full engine suites passed after the guard; host/desktop checks continue.

# Preparation proposal R2

Doc-only `7519e5eddfc33cebdc498cbad263df1aee218b3a` directly follows `07cdb24b`. It addresses physical rows versus possible generations, ordered selected-history views/inherited capacities, owned edit/replacement intent with serial callback semantics, and fallible device lifecycle fences. A new independent design audit has the original brief, full R1 findings and revised response. No source grant or P1 acceptance follows from the document.

# Imported test-only repair

Root imported only M1 `760568ca` as `abd044dd`, after clean source verdicts on both axes. App `9243f20f`/`e3cdbb3f` were not imported. The invalid-meter identity guard is separately committed as `7354fc4b`. Matching source bytes in completed native binaries are unchanged by these Git metadata operations; default-parallel analysis execution was pending at this point and is recorded below when completed.

# Preparation design R2 acceptance and engine-only grant

**Recommendation: `7519e5ed` is design-ready for narrow, serialized implementation in the requested windows.** I found no additional blocking design objection. This grants an implementation attempt; P1’s production lock defect remains open until composed-source proof passes.

The four R1 responses resolve their respective design conditions:

1. **Physical rows and generation identities:** [§2a/2b](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:449) now distinguishes P20/G30, maps alternatives to destinations, flattens unresolved choices, and refuses a required third source before construction or Document mutation. The common Ledger predicate also couples generation retention with actual owner reuse, addressing the mismatch between [plan retention](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/plan.rs:337) and builder eligibility. **Required proof:** queue `{g0,g1}` plus active g2; retain g2 without construction; refuse removal requiring three sources; then advance completion/first hearing between lease acceptance and adoption. Check concrete ownership, active indexes and row bounds.

2. **Selected history and inherited capacity:** [§2c/2d](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:535) supplies ordered selected views, copy-before-retarget ordering, suffix/promotion/fallback rules, and conservative inherited ring/tap capacity. These directly address the physical-stage assumptions in [history transfer](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/rack.rs:1012). The checked payload calculation and visible memory refusal are acceptable; they establish neither measured feasibility nor a global legacy-backlog bound. **Required proof:** late choice during unfinished taps, larger inherited capacities without clamping, and unchanged R3–R5 signal/allocation assertions. Include adopted-None rows followed by fading-voice/fader edits: inactive storage must not incorrectly influence [shaped/ringing decisions](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/state.rs:802).

3. **Publication intent:** [§3a](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:637) supplies owned Edit/Replace intent, late transport binding, and reserved producer capacity. Its replacement order matches [existing publication](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/apps/desktop/src-tauri/src/session/files.rs:461), while retaining serial consumer semantics. **Required proof:** late play/stop/seek/pattern changes, full/one-short/exact-capacity queues, and consumer interleaving/headroom pauses. Refusal must precede musical mutation; accepted installation must require no fallible fallback.

4. **Device adapter and retirement:** The [attachment design](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:840) and [explicit device window](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:954) now cover the previously omitted [infallible production attachment](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/device.rs:724). Starting/Closing fences and retained control endpoints provide a feasible ownership order. **Required proof:** constructor Err, CPAL build/play failure, bounded stale exhaustion, same-rate reopen, and full-ring/backlog teardown. Destructors must verify guard freedom, including refusal, poison and unwind, and final endpoint release after Processor destruction.

Borrowed outer staging and private Unavailable-versus-Native handling introduce no further design blocker. T1 invalid-meter refusal/healthy retry, M1 borrowed eligibility-before-dispatch and acknowledgement-after-install, and N4’s independent render/error channel remain mandatory integration checks.

Read-only audit completed at the fixed pin. No edits, imports, builds, tests or nested agents.

Parent granted stage 1 engine implementation only after that audit. The same bound owner merged the sole authorized root prerequisite `abd044dd` as `937891439e4c9f268ddfa5cd91c1b7c6a9d4e8ff`, preserving both immutable proposal pins. Controller/State/Plan/Rack/private native preparation, message/export declarations and fallible device lifecycle are the narrow implementation window. Session publication, Processor/transport, N4 runtime/host ABI, M1/T8 integration and render/stem error-channel changes remain closed. No design verdict closes the P1 locking defect; actual engine source, retirement and queue/history proof must precede a separate Session grant.

# Invalid-meter snapshot guard source review

Fixed root guard `7354fc4b61e862ad74ab2c0d1ab4e9b9e23e2462` preserves accepted valid-plan native identity snapshots and skips them only for a typed rejected meter map. The unchanged Controller refusal test now passes in the full composed engine suite.

## Standards

Judgment: **accept this guard. No findings** in the pinned `c0fcabea…7354fc4b` diff.

- **Standards:** No documented-rule violation or actionable baseline smell. The condition adds no abstraction, ownership mechanism, or callback work.
- **Spec:** At [plan.rs:584](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-engine/src/plan.rs:584), `meters.is_ok()` prevents both factory revision and provider-identity reads for invalid raw plans while preserving their typed `MeterMapError`. Existing Controller refusal gates run before state mutation, native construction, publication, or selected-pattern changes. Valid plans execute the same snapshot, linking, automation, installation-refresh, and retention paths, preserving the documented R3–R6 identity/life contract.

Source-only checks: verified both commit objects and the one-file diff; inspected pinned compilation, snapshot, Controller gates, the invalid-meter regression assertions, and relevant architecture, timeline, roadmap, and utility docs.

No edits, builds, tests, imports, pushes, or nested agents. Reported passing suites remain parent execution evidence, not independently verified here. The known P1 preparation/retirement-under-guards objection remains open and unchanged; this judgment covers only the composition guard.

## Spec

**Approve for the bounded guard.** No actionable standards violations, spec failures, or judgment-call smells found in pinned `c0fcabea…7354fc4b`.

At [plan.rs:584](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-engine/src/plan.rs:584), the condition skips provider identity/revision snapshots for invalid meter maps while preserving the original typed `MeterMapError`. The existing Controller refusal gates precede state mutation, owner retention, native construction, and publication.

For valid maps, snapshot timing, linking, automation compilation, and utility R3–R6 identity/revision/lifetime retention remain unchanged. Raw compilation retains its existing tolerance of other invalid model values.

Checks were source-only: inspected the pinned diff, relevant documentation, Controller gates, and the named regression’s unchanged assertions. `git diff --check` passed. No edits, builds, tests, imports, pushes, or nested agents were performed; parent execution results remain separate evidence.

The acknowledged P1 preparation/retirement-under-guards issue remains open and outside this guard’s acceptance.

# Composed root verification before final MIDI import

At `abd044dd`, Windows serial MSVC/jobs=1 native checks passed: project 331; FLP 129; engine 410 plus one existing optional CPU ignore; host 186 ordinary cases plus the explicitly invoked sticky CLAP native-state case (187 total); desktop 43 focused native cases (timeline 11, plugin/session 11, playback 14, export 7). The 21-class native fixture was built separately. Host real-process tests exercised the authenticated subprocess role despite its ordinary-suite ignore. These are actual parent runs, distinct from child evidence and source-only reviewers. No installed external-plugin, native editor, physical device or other-OS claim follows.

Strict Clippy passed for project/MIDI/FLP/IPC/sim/engine all targets, host all features/all targets and desktop lib/tests; workspace formatting passed. After the one-file heap isolation import, root analysis ran with default parallel test scheduling: 30 passed, zero failed/ignored, followed by strict all-target Clippy. Remote corrected macOS/Linux runs remain pending.

Fresh generator output contains 183 bindings. `scripts/check-bindings.mjs` matched their exact bytes. `.gitattributes` disables only end-of-line whitespace warnings within generated bindings, preserving ts-rs output without hand editing it. The intermediate simulator was 1,982,006 bytes, SHA-256 `9b5a7e88ec863b901b2f7b0205cbcdb052c97b501fb82bba364d9ca1f6c9144a`, input digest `9cbcf6165f84c3ebac24cc763828cc2feb28646620fef13ce067f360a711896c`. On those matching intermediate artifacts, full desktop TypeScript/ESLint checks passed and the actual shared-WASM UI suite passed 172 files / 2,662 tests in 335.06 seconds (`C:/Temp/windfall-root-timeline-all-ui.log`). The final MIDI source import requires a further matching simulator rebuild; intermediate evidence is not relabelled as that final run.

# MIDI integration repair

Root imported only `aa6c55b6138d34257d73bb55a4e5c30cc6b6deb5` as `b801cc21`, retaining every earlier timeline/native/utility pin. The four-file repair uses the effective last-wins tick-zero meter for scalar pattern sizing, or 4/4 before a late first event. Ordered absolute meter changes and the existing last-clip export end authority remain unchanged. The complete metadata oracle is derived independently from source; all existing note/channel/mix/tempo/history/duration/byte assertions remain.

The real failing canonical trip grew by 1,920 ticks (28,800 to 30,720); the earlier parent shortening interpretation was incorrect. The authored fixed fixture stabilizes at 30,720, and the collision/no-tempo fixture at 30,240. New independent Standards/Spec reviews target exactly `3d2475c7...aa6c55b6` with the original task, all prior findings/responses and root failure context. Their source conclusions and final parent runtime/artifact results will be recorded separately.

# Production bridge capture repair R2

Frozen `6b228e1216d6e33757faa2d5970fba66f935039d`, direct parent `19f6ddc2`. Both new reviewers received original N4 scope, all earlier R1/R2/R3 findings/responses, prerequisite and production verdicts, the new capture P1 and its exact response.

## Standards

Standards review of fixed `19f6ddc2…6b228e12`: **0 new findings**—no documented-rule violations against pinned `docs/ARCHITECTURE.md`, and no new optional heuristic concerns.

The [capture fix](C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src-tauri/src/plugins/bridge.rs:90) initializes every transmitted value from its current control snapshot before pending-document overrides. It adds no callback work or generation/proof mutation.

The added test source covers settled COMPLETE followed by an unprocessed note, unchanged DSP proof, distinct CLAP/VST3 reconciliation, capture/save/reopen, opaque-state-only restoration, and newer pending-document precedence. Reported execution results remain owner evidence; I ran no builds or tests.

Prior optional P3 concerns remain deferred. Guarded construction/retirement P1 and the full T1 producer/pre-refusal composition gate remain material and open. This review does not authorize production integration or activation, close full N4, or accept editor, installer, device, licensed-corpus, or non-Windows gates.

Inspection used immutable Git objects only; no edits or nested delegation.

## Spec

**SPEC: 0 new findings** in frozen `19f6ddc2…6b228e12`.

The prior capture P1 is repaired at source: [bridge.rs:94](C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src-tauri/src/plugins/bridge.rs:94) initializes every transmitted value from its current control snapshot before pending document overrides. Settled gain therefore survives note-only pending intent, while newer committed values retain precedence.

The added CLAP/VST3 facade tests explicitly require COMPLETE settlement before queuing the note, then assert unchanged DSP proof and distinct inactive reconciliation. The Session cases cover capture/save/reopen, restoring opaque state without parameter overrides, pending `0.75` precedence, and unchanged live document/ownership.

Inspection used immutable Git objects; no builds or tests were executed. Reported RED/GREEN results remain owner evidence.

The acknowledged guarded construction/retirement P1 and full T1 malformed-meter precheck remain open. This review does not accept production activation or full N4; installer, editor, device, licensed-plugin corpus and non-Windows gates remain unverified.

Parent retains the clean source verdict without importing production `19f6ddc2` or `6b228e12`. Their actual native tests remain owner evidence. P1 prepared installation/retirement and checked full meter/render composition still gate any production integration or activation.

# Final MIDI native and artifact checks

At root `b801cc21`, all 116 MIDI targets passed with `PROPTEST_RNG_SEED=20261008`, including existing persisted seeds and all four 300-case properties. Strict MIDI all-target Clippy and workspace fmt passed. No regression seed changed. Final matching simulator: 1,982,055 bytes, SHA-256 `d1a3aa6b1d4b908b160ed77fc88c4ee82fa1ec8bad7206ee0e715e5ed27e51a3`; input digest `732414a3bad9f213c17a4791f5bb3301caa4e5f6e1b8beb04f8e5fc3c180ea59`. Freshness, exact 183-binding comparison and parity checks passed. Native/build log: `C:/Temp/windfall-root-midi-aa6-final.log`.

The final full UI run on that exact artifact also passed **172 files / 2,662 tests**, in 292.90 seconds (`C:/Temp/windfall-root-timeline-final-all-ui.log`), without mocks replacing the shared Rust document. No code or assertion changed between the intermediate and final UI runs. Previous TypeScript and ESLint passes apply to the identical UI/generated type sources. Parity remains 68 done / 60 in progress / 212 todo / two won't-do; no row was closed through this integration batch. Published alpha, private-repository access and collaborator permissions are unchanged.

# MIDI integration repair source review R1

Both new independent reviewers inspected immutable `3d2475c7...aa6c55b6` with the original timeline scope, complete prior findings/responses and parent integration failure context.

## Standards

**Standards: 0 new findings.** No documented `docs/ARCHITECTURE.md` breach or actionable Fowler smell in this increment.

**Spec: 0 new findings.** Stable normalization preserves collision order, so selecting the last tick-zero event agrees with map retention. The 4/4 fallback fixes late-first-meter sizing while preserving later absolute ticks and disabled-signature behavior. The replacement oracle derives metadata from normalized source, independently of import/export results. Original non-metadata assertions remain intact; authored fixtures check canonical duration, exact bytes, notes, and checked undo/redo.

Reviewed the exact `3d2475c7...aa6c55b6` diff/log—one commit, four paths—and complete pinned `TIMELINE-REGIONS.md`. Tests were not executed; RED/GREEN results remain owner-reported evidence.

Global construction/retirement P1, parent integration, combined artifacts/WASM, platform/device acceptance, and retained T1 follow-ups remain open. No parity-row closure or global-gate waiver.

## Spec

**0 new findings.**

**Standards — 0:** No new violations of the pinned `docs/ARCHITECTURE.md` contract or legal rules.

**Spec — 0:** The importer selects the effective last tick-zero meter after stable normalization and uses 4/4 for sizing before a late first event. Later map ticks, option gating, collision/clamp/bounds handling, and export’s clip-end authority remain intact. The metadata oracle derives expectations from normalized source data; existing content, history, duration, and byte assertions are retained. The authored fixtures pin 30,720 and 30,240 ticks.

Reviewed the exact `3d2475c7...aa6c55b6` diff/log and complete timeline record using read-only Git objects. No builds or tests were run; execution evidence is owner-reported.

The global native construction/retirement P1 remains **OPEN**. Combined source/artifact, platform/device, and remaining T1 acceptance gates are unchanged; this review closes no rows.

Both source verdicts are clean. The parent full 116-target execution and final artifact checks above are separate executed evidence; no global preparation, hardware/platform, remaining timeline or parity gate is waived.

# Additional CI repair windows

The analyzer owner reproduced default-parallel contention on unchanged source: nine pass/seven fail, then eleven pass/five fail, each with the existing optional timing ignore. Controlled unrelated admission reproduced the count discrepancy and legitimate `InstanceLimit`; concurrent probes still admitted exactly eight tap contenders, or four maximum-layout contenders with four `ByteLimit` refusals. No production quota defect was demonstrated. Parent granted only a test-local mutex held across all 17 analyzer cases and endpoint/native-worker destruction, with every quota/signal/allocator assertion and process-wide eight-tap/eight-slot/32 MiB policy unchanged. No global CI serialization or routing/source grant follows. New source and review remain pending.

The release owner froze `cbaeedac`, a four-file workflow/context checker repair after the pinned GitHub parser reproduced three invalid job-level `runner` references and zero after repair. The VST portability owner froze `5d3fe8e7`, a four-file creator-identity fixture repair for verified GNU Linux x86_64, preserving both owner assertions and class0–20 behavior. New independent Standards/Spec pairs inspected each exact increment. Neither was imported at this point; their subsequent source verdicts and parent execution are recorded below. Remote workflow/Ubuntu validation and macOS support remain distinct gates.

# Native fixture identity repair review R1

Immutable `5d3fe8e7c7f35cb1c0fd4538cfbc381796296387`, direct parent `1c22651e` (the separately applied equivalent `71dbec11` fixture prerequisite). Parent imported only the identity increment as `27db8b89`. Original classes0–20, both ownership assertion messages and R4 capacity/drop behavior remain intact.

## Standards

**0 standards findings.**

Reviewed `1c22651ef288888294ca2351d57edba9c16dde52...5d3fe8e7c7f35cb1c0fd4538cfbc381796296387`, its commit log, all four changed paths, `docs/ARCHITECTURE.md`, and the full pinned identity/provenance document.

The private `CreatorThread` type appropriately contains the platform identity. Its FFI signature and value equality match the documented Linux GNU x86_64 ABI verification, with the live-creator limitation explicit. Other targets retain the original `ThreadId` behavior. Both owner assertions and messages remain intact.

Tests retain creator/library ownership during foreign-thread comparisons, exercise both actual fixture guards, and unload the exact-helper cdylib before worker exit. Reversing only the authorized substitutions reproduces the baseline fixture exactly; all six prerequisite blobs match `71dbec11`. No actionable baseline smell was found.

Unresolved gates remain: original Ubuntu CI confirmation; macOS loader/bundle repair and native validation; other-platform ABI/unload verification; unavailable native Linux Clippy and the documented unchanged fixture warning at line 741. Full bridge activation, P1 construction acceptance, and the overall hosting goal remain open.

Read-only Git-object review only; no builds or tests executed.

## Spec

Spec: **0 findings** for `1c22651ef288888294ca2351d57edba9c16dde52...5d3fe8e7c7f35cb1c0fd4538cfbc381796296387` and its single-commit log.

The four-path diff stays within the grant. The private helper uses `pthread_self() -> c_ulong` with C linkage under the specified Linux/GNU/x86_64 cfg, consistent with the pinned ABI documentation. Other targets retain `ThreadId`. Both original owner assertions remain intact; tests keep creators alive during comparisons, exercise both actual component guards against another live owner, and drop the helper library before worker exit and join. Existing classes, DSP, R4 behavior, point arena, drop and scanner source remain unchanged.

Linux/Windows passes and baseline SIGSEGV reproduction are **owner-reported execution evidence** in the fully read pinned document. This review inspected Git objects only and ran no builds or tests.

Remaining gates: original Ubuntu CI confirmation, macOS loader repair and platform verification, and full native-hosting acceptance—including bridge activation and P1 construction under guard. Native Linux Clippy remains unavailable; the inherited standalone-fixture warning at `vst3.rs:741` remains outside the grant. Scoped lint passes do not waive broader checks or establish other-platform ABI correctness.

Parent Windows execution after rebuilding the current 21-class fixture passed the four new ownership/unload regressions, all 20 realtime cases with default parallel scheduling, VST3 host 12 and scanner two. Strict host all-feature/all-target Clippy, workspace fmt, simulator freshness and diff checks passed. Log: `C:/Temp/windfall-root-vst-5d-final.log`. Linux native results remain owner evidence; original Ubuntu CI confirmation, macOS loader and unavailable native Linux Clippy are not relabelled as parent passes.

# Candidate-workflow context repair review R3

Immutable `cbaeedac810450dd1439ba3572eb1de0648828cb`, direct parent `477209ec`. Parent imported only this increment as `3459d430`, with the manual workflow and publication gates retained.

## Standards

0 Standards findings.

Reviewed `477209ec1d3c5f2d7efe5e7147b5551a4bfbe0c1...cbaeedac810450dd1439ba3572eb1de0648828cb`. No documented architecture violation or actionable baseline smell found in the four-path diff.

Static inspection confirms step-scoped `runner.temp` values reach every downstream consumer through `GITHUB_ENV`, including the upload’s `env.CANDIDATE_OUTPUT`. Manual dispatch, read-only permissions, action pins, source identity, immutable-alpha protections, license/resource gates, signing restrictions and verification before upload remain preserved.

GitHub server acceptance, signing/notarization, native installer/install/update/uninstall validation, editor integration, helper/model admission and operator/license approval—including `buffers@0.1.1`—remain open. Parent alone publishes.

Read-only Git object inspection only; recorded parser and fixture executions remain the owner’s evidence.

## Spec

0 Spec findings.

Reviewed `477209ec1d3c5f2d7efe5e7147b5551a4bfbe0c1...cbaeedac810450dd1439ba3572eb1de0648828cb`, its single commit, all four changed paths, and the pinned release/architecture contracts.

Source inspection confirms the three runner paths are defined in step `env` and exported through `GITHUB_ENV` before capture, native build, collection, verification, and upload. The upload consumes `env.CANDIDATE_OUTPUT` in step `with`. Manual dispatch, read-only permissions, action pins, immutable source identity, alpha protection, resource admission, SBOM/notices verification, signing rejection, and verified no-clobber upload remain preserved.

The parser’s three-errors-to-zero comparison and 20 passing fixtures are owner-reported execution evidence; I did not rerun them.

Unresolved gates remain: GitHub service acceptance; native installer contents and install/update/uninstall; signing/notarization; editor verification; helper/model resources; operator configuration; corresponding-source/license review, including historical `buffers@0.1.1` provenance. Publication remains parent-owned and gated.

Root's static syntax/context/release-contract checker passed. The combined existing release and new workflow fixtures passed **68 tests**, zero failures/skips, in 163.79 seconds (`C:/Temp/windfall-root-release-cbae-final.log`). This is separate from the owner's narrow 20-fixture execution and pinned parser comparison. No candidate workflow was dispatched and no bundle, artifact upload or release was produced. Published alpha and future version/signing/operator/license requirements remain unchanged.

# Analyzer test-quota isolation review R1

Frozen `2a46e2aaa27390a25677e9ec0617f115f0b400c8`, direct parent `51c88f45`, imported only as root `dfe522b5`. Both new reviewers received the original foundation, all prior review findings/responses, actual Windows CI failure and controlled contention proof. The increment contains only the test file and evidence document.

## Standards

**0 new findings** for pinned `51c88f45212f9592b62349a9bc11ecf9d7343bc9...2a46e2aaa27390a25677e9ec0617f115f0b400c8`.

**Standards:** 0 documented breaches; 0 optional heuristic findings. The mutex is confined to integration-test orchestration, consistent with [ARCHITECTURE.md’s callback rule](C:/Users/ewhee/.t3/projects/windfall/docs/ARCHITECTURE.md:176).

**Spec:** The narrow repair is source-acceptable. All 17 cases acquire the same named guard as their first local, including the existing ignored CPU case. Explicit cleanup and reverse local destruction keep endpoint retirement and native joins inside the guard lifetime. Acquisition precedes allocator-watch and timing intervals. No recursive acquisition or worker dependency on this test mutex appears.

[Poison recovery](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-engine/tests/analyzer_taps.rs:17) holds the recovered guard, requires zero taps/slots/charged bytes, then clears poison. Nonzero retained ownership fails without resetting accounting; the originating panic remains a failed libtest result.

Mechanically removing only the import, helper, and 17 guard statements reproduces the complete base test source. Every original assertion remains unchanged. The diff contains only the two granted paths, with no production, dependency, limit, or feature changes.

Owner-reported executions and controlled diagnostics remain **unreproduced by this review**. The parent’s fresh root default-parallel run remains pending. Actual graph/session routing, native delivery/UI, EQ live spectrum, integrated measurement, physical-device acceptance, and T8/P1 readiness remain open as documented in [ANALYZER-TAPS.md](C:/Users/ewhee/.t3/projects/windfall/docs/ANALYZER-TAPS.md:510).

## Spec

**0 new spec findings** for pinned `51c88f45212f9592b62349a9bc11ecf9d7343bc9...2a46e2aaa27390a25677e9ec0617f115f0b400c8`.

All 17 cases acquire the same test-local guard as their first local. It survives endpoint cleanup and explicit/RAII native joins, including unwind, and acquisition precedes allocator and timing intervals. There is no nested acquisition or worker/callback acquisition path introducing a deadlock.

Removing only the new import, helper, and 17 guard statements reconstructs the entire base test file exactly after newline normalization, preserving all 148 original assertion macro callsites. Production limits and code are unchanged.

Poison recovery checks zero taps, slots, and charged bytes while holding the poisoned guard before clearing poison. Retained credit causes failure without resetting accounting; the original libtest failure remains visible.

The complete pinned analyzer document and architecture were reviewed. Evidence labels distinguish prior serial runs, failing parent CI, private diagnostics, and owner post-repair checks. Those execution results remain **unreproduced by this review**; the parent’s fresh root default-parallel run is pending.

Both integrated routing and measurement gates remain unresolved: actual attached-graph/session/native delivery, EQ live spectrum and UI, fresh integrated measurements, and physical-device checks. No T8-row or P1-readiness closure follows from this repair.

Parent subsequently executed the composed root target under default parallel scheduling and again with one test thread: **16 passed / one existing optional CPU ignore** each. Strict engine all-target Clippy, workspace fmt, exact bindings, simulator freshness, parity and diff checks passed (`C:/Temp/windfall-root-t8-2a-final.log`). All original assertions and process-wide limits remain unchanged; serialization is limited to this analyzer test executable. Remote Windows behavior still requires the next CI run. No integrated analyzer, EQ, device or P1 readiness gate closes.

# Remaining app and preparation gates at this push

P1 stage 1 remains isolated engine implementation on the sole authorized prerequisite merge. The legacy attached-publication path is being made to refuse readiness work rather than silently construct native units under inherited document/controller guards. New source, exact queue/history/retirement proof and independent review must precede the separate Session publication window; no partially migrated engine source is imported here.

M1 App R2 has actual old-Session RED for a DOS drive remap after source hashing, alongside the repaired dialog lifetime. Local retained-object and direct-volume alias probes passed their bounded mechanisms, but the global/local shadow fixture was unavailable (`AccessDenied`) and was not executed as authority proof. Its production namespace source remains exactly `9243f20f`; proposed 66-handle/1,122 aggregate caps remain prototype-only. A further private test-only object-creation seam is authorized for unused owned slots without elevation, privileged helper, physical-drive or competitor mutation. No global namespace policy or M1 app source was accepted/imported. Foundation and test-local heap isolation remain distinct from app/inference acceptance.

The pushed batch contains reviewed timeline/MIDI/native-bar source and matching 183 bindings/WASM, isolated bridge ABI3 prerequisites/fixtures, test-only analysis/analyzer CI isolation, the Linux fixture identity increment and gated candidate-workflow scope repair. It excludes full production bridge `19f6ddc2`/`6b228e12`, M1 app `e3cdbb3f`/`9243f20f`, and P1 engine drafts. No version, published alpha, candidate dispatch, operator signing material or parity counter changed.

# CI results for the composed source `7027569f`

[CI run 37766014784](https://github.com/erivgout/windfall/actions/runs/37766014784) tests exact pushed source `7027569f8b466242a75faf327381e6ac44c88d28`. At this observation, Ubuntu Rust, simulator freshness, parity and all three platform binding jobs passed. Windows Rust remained running; the app jobs had not supplied acceptance evidence.

The UI job `113273753452` failed one test: `timeline-hydration.test.tsx`, “hydrates an armed native range before disabling the reload menu and clears it through the menu.” Its synchronous `getByRole` at line 79 could not find “Clear song time selection”; the captured DOM showed the Timeline trigger with `aria-expanded=false`. Counts were **one failed / 171 passed files; one failed / 2,661 passed tests**. This is actual remote evidence, distinct from the two complete local 172-file/2,662-test passes. The same timeline owner has a narrow test/menu-lifecycle diagnosis window; no cause or repaired CI result is assumed. Raw log: `C:/Temp/windfall-702-ui-ci.log`.

The macOS Rust job `113273753439` failed the same eight VST3 realtime cases with `Unsupported("VST3 bundle entry on macOS")`: **12 passed / eight failed / zero ignored** in that executable. The loader still explicitly refuses the macOS bundle entry path. The portability owner is tracing the proper bundle lifecycle and fixture exports read-only; Windows/Linux success does not establish macOS support. Raw log: `C:/Temp/windfall-702-macos-ci.log`.

The macOS log separately records the analysis integration executable **30 passed / zero failed / zero ignored**, the utility executable **152 passed / three existing ignores**, and analyzer taps **16 passed / one existing optional CPU ignore**. These remote results confirm those narrow CI repairs on this source; they do not close app namespace, integrated analyzer, native preparation, hardware or parity gates.

# Preparation stage 1 policy correction in progress

The isolated P1 owner reported five readiness tests and four owned R6 tests passing, with the utility native set at 40/42. Those are owner execution reports on uncommitted isolated source, not root acceptance. Tracing the two failures identified that the first reservation implementation incorrectly promoted every changed active native revision/kind into a departure. Existing `Plan::keep_leaving` instead directly replaces an active owner while its ID remains requested, retaining any already-outgoing owner. An actual ID removal has different possible-source requirements.

The owner is restoring that exact policy in the granted Plan window. Paired same-ID revisions must supersede unheard current owners while retaining the existing outgoing generation; genuinely unresolved removal requiring three source generations must still refuse before construction/publication. All original process/drop/live-owner, latency, signal and callback-allocation assertions remain mandatory. No Processor or fixture expansion is granted. Corrected fixed-source checks and independent review are still required before any P1 source integration.

# Independent M3 implementation started

The T3-owned GPT-6.1-Sol implementation child is bound to `gpt/t3-pitch-analysis-m3` at exact base `7027569f`, in its separate worktree. Its narrow window is the new monophonic pitch module/tests/contract, one module export, and one dependency/reference to the already-locked `rustfft 6.4.1`. The early worker-only contract uses checked borrowed PCM, exact frame ranges/origins, explicit channel selection, measured F0/voicing/segments and visible resource/cancellation refusal. The cached dependency checksum/license was verified by the owner. No source checkpoint or signal-quality acceptance is claimed yet.

Session/job/UI registration, editable pitch/timing proposals, correction/harmony, tuner, warp editor and M3 parity closure remain open. M1 foundation acceptance does not imply acceptance of its excluded app or the P1 publication path.

# Terminal CI results for `7027569f`

The earlier observation above is superseded by the terminal result: [CI run 37766014784](https://github.com/erivgout/windfall/actions/runs/37766014784) failed. Ubuntu Rust, simulator freshness, parity and all three platform binding jobs passed. The UI, macOS Rust and Windows Rust jobs failed; the app job was skipped, so it supplies no app acceptance evidence.

The Windows raw log `C:/Temp/windfall-702-windows-ci.log` records the bridge executable at **26 passed / one failed / one ignored subprocess role** (6.76 s). The sole failure is `vst3_capture_does_not_relabel_dsp_proof_after_a_reset_without_processing`: the second capture after reset returned `"capture timeline ownership changed"` at `tests/bridge.rs:1386`. The test requires epoch 2 with zero processed-generation proof and zero acknowledgement before any subsequent DSP. No native hang was established; compilation accounts for the earlier long job duration. The same bridge owner has a read-only diagnosis window on the reset/control/header/native-epoch ordering. No retry, assertion relaxation or cause is assumed.

Windows separately passed the desktop executable **314 tests** and the analysis integration executable **30 tests**. The macOS eight unsupported VST3 bundle failures and the UI lookup failure recorded above remain the actual remote failures. The Linux identity repair is confirmed by the Ubuntu job; macOS bundle lifecycle and the Windows reset/capture mechanism still require repair and fresh remote verification.

# Timeline asynchronous hydration lookup review R1

Immutable owner pin `393cc39ebfd8ddea9f416ab4a38175fa2785d51f`, direct parent `aa6c55b6`. The parent imported only this two-path repair as `20e58627d1cc9430bd86f026508d56d3121a5abc`.

## Standards

**Standards: 0 new documented breaches; 0 new baseline-smell findings.**

Reviewed `aa6c55b6138d34257d73bb55a4e5c30cc6b6deb5...393cc39ebfd8ddea9f416ab4a38175fa2785d51f`: one commit, exactly two paths.

The change at [timeline-hydration.test.tsx:79](C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src/features/playlist/timeline-hydration.test.tsx:79) only replaces the synchronous lookup with `await findByRole`. The exact role/name, default timeout, single trigger/Clear clicks, and every assertion remain unchanged. Registry-backed menu execution and canonical backend assertions remain consistent with `docs/ARCHITECTURE.md:430,437,448`.

Performed read-only checks: resolved both pins; inspected the diff/log, complete pinned timeline document, supplied standards, shared control/dropdown/action code, test setup, dependency lock, and relevant ledger history at fixed `fd4ad940`. Blob comparison confirms the control, dropdown and lock match root `7027569f`. The raw CI log confirms the closed trigger, failing lookup and **1 failed/2661 passed tests**; its SHA-256 matches the document.

Base UI implementation code is unvendored in the pinned Git tree. Its animation-frame trace therefore remains owner-reported source evidence; the locked **1.8.0** dependency was independently checked.

The owner’s baseline **4/4 pass**, subsequent **64/64 passes** and strict checks remain owner execution evidence. The parent’s two complete local UI passes remain distinct from the remote failure. I executed no tests, builds or generators.

Earlier findings/responses—including the inherited nonblocking validator-duplication judgment—remain intact. Fresh combined CI, P1 preparation/retirement under guards, N4 production activation, native/plugin/platform acceptance and all deferred T1 requirements remain open. No roadmap row or phase of the full **0–7/342-row** goal closes through this review.

## Spec

**Spec: 0 new findings. Accept the bounded repair at `393cc39e`.** Verified `aa6c55b6...393cc39e`: one commit, exactly two paths.

At [timeline-hydration.test.tsx:79](C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src/features/playlist/timeline-hydration.test.tsx:79), `await findByRole` waits for the same exact Clear role/name with the default timeout. Both single clicks and every hydration, range-label, enablement, backend-clear, selection and returned-focus assertion remain unchanged. No production behavior or scope expansion appears.

The raw CI log’s SHA-256 matches the document. It confirms the synchronous lookup failure, closed Timeline trigger and totals of **1 failed/171 passed files; 1 failed/2661 passed tests**. The deferred-opening explanation is consistent with the supplied Base UI trace and uncontrolled app wiring; callback timing is explicitly not claimed as instrumented.

[The evidence record](C:/Users/ewhee/.t3/projects/windfall/docs/TIMELINE-REGIONS.md:984) correctly separates the passing local **4/4 baseline**, owner-reported **64/64 focused checks**, and earlier root full-suite passes from the remote failure. Validation-only root WASM is identified as **1,982,055 bytes**, hash `d1a3aa6b…`; own-source typecheck used **179 cached bindings**, distinct from root’s **183**.

Read-only checks covered resolved pins, diff/log, complete pinned timeline document, specified spec sources, prior ledger contexts, relevant controls/store/registry/dropdown/lock/configuration, CI evidence and `git diff --check`. No edits, builds, tests, generators, imports or nested reviewers occurred. Dependency internals are absent from pinned Git objects; their trace remains supplied evidence.

Fresh combined-root/artifact and CI verification remain required. P1 preparation/retirement under guards, N4 production activation, native/platform acceptance and retained T1 work remain open. Prior findings/responses stay intact; this repair closes no roadmap row or phases 0–7/342-row goal.

The parent subsequently executed the four focused hydration/Add/lifetime/reload files on the combined source: **64 passed / four files**, 12.14 s (`C:/Temp/windfall-root-timeline-393-focused.log`). Root `pnpm typecheck`, scoped ESLint and changed-test Prettier checks passed. Simulator freshness remains current at **1,982,055 bytes**, SHA-256 `d1a3aa6b1d4b908b160ed77fc88c4ee82fa1ec8bad7206ee0e715e5ed27e51a3`, source inputs `732414a3bad9f213c17a4791f5bb3301caa4e5f6e1b8beb04f8e5fc3c180ea59`. No new full UI or Rust execution is attributed to this one-line increment. Fresh CI verification remains pending; the separate macOS and Windows failures remain unresolved.

# Monophonic pitch foundation review R1

Immutable owner pin `60327573326a27ebdf000588e0ff0a7239ec30c0`, direct base `7027569f`, six authorized paths. This source is **not imported**. The worker-only foundation implements checked borrowed PCM, explicit channel selection, exact integer range/origin, pitch/voicing/segments and resource/cancellation refusal. Owner execution reports 51 ordinary tests and an explicitly invoked long resource case; those are not parent execution or real-vocal quality evidence.

## Standards

Standards review of `7027569f...60327573`: **0 hard violations; 1 optional finding.**

- **Optional — possible Duplicated Code:** [pitch.rs:603](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-analysis/src/pitch.rs:603) and [pitch.rs:639](C:/Users/ewhee/.t3/projects/windfall/crates/windfall-analysis/src/pitch.rs:639) repeat the raw squared-difference formula, `E[w] + E[w+k] - E[k] - 2*correlation[k]/N`. A private helper would keep interpolation and difference calculation aligned during future numerical changes. Preserve the existing finite/negative-roundoff checks. This is a maintainability judgment, not a demonstrated defect.

The module satisfies the reviewed interface and locality requirements: checked preparation, immutable borrowed input, integer frame coordinates, explicit channel selection, bounded work/output, complete-result construction, and reusable scratch after failure. The resource formulas include scratch and RustFFT plan-construction transients; the documented infallible-constructor allocation limitation remains.

The dependency change stays within scope. Cached RustFFT 6.4.1 source, checksum, MIT/Apache texts and notice information agree with [PITCH-ANALYSIS.md:96](C:/Users/ewhee/.t3/projects/windfall/docs/PITCH-ANALYSIS.md:96), satisfying the dependency-license rule in [ARCHITECTURE.md:462](C:/Users/ewhee/.t3/projects/windfall/docs/ARCHITECTURE.md:462). The lock diff adds only the analysis crate’s existing RustFFT reference.

I considered all three documented initial failures. Raw-difference interpolation follows the cited bias correction in [YIN §II.E](https://www.ee.columbia.edu/~dpwe/papers/deChevK02-yin.pdf); the boundary test uses authored transition coordinates, and range refusal inspects shorter lags. These checks do not establish real-vocal quality.

Read-only checks covered ancestry, commit log, the six-path diff, complete module/tests/document, and pinned public `Work`/`CapturedInput`/`AudioShape` APIs. No edits, builds, tests, generation, imports or agents were run. P1/R2 attachment gates and the remaining M3 editor, correction, warp and listening requirements remain open.

## Spec

Spec review: **1 finding (P2)** at `60327573326a27ebdf000588e0ff0a7239ec30c0`, against `7027569f8b466242a75faf327381e6ac44c88d28`.

- **[P2] Apply the threshold to interpolated CMND trough depth.** `crates/windfall-analysis/src/pitch.rs:630–657` requires a sampled CMND value below threshold before interpolation. For supported 8 kHz / 50–1000 Hz settings, a noiseless **941.17647 Hz fundamental plus a second harmonic at 1.2× its amplitude** has an 8.5-sample period. Its sampled first trough exceeds the default 0.15 threshold; lag 17 repeats exactly. By source/math inspection, the detector therefore selects approximately **470.6 Hz with confidence near 1**, an octave error. This conflicts with the requested “musically useful detection” and “calibrated notes/harmonics … octave traps.” [YIN II.E](https://www.ee.columbia.edu/~dpwe/papers/deChevK02-yin.pdf) uses interpolated CMND minimum depth for selection, then raw-difference interpolation for the period. The harmonic fixtures at `tests/pitch_analysis.rs:161–180` use only 48 kHz and miss this case. Add low-rate fixtures with fractional sample periods and correct trough acceptance.

The fixed-window FFT difference calculation, integer coordinates, stereo policy, validation, checked budgets, reuse and complete-result error handling otherwise appear consistent with the foundation contract. No scope expansion found.

Foundation acceptance should await this correction. Full M3 editor/warp/correction work and real-vocal/broader-platform evidence remain open, as documented at `docs/PITCH-ANALYSIS.md:118–125,236–238`.

Read-only review covered the complete six-path change, pinned public APIs/specs and relevant primary sources. No edits, builds, tests, generation, handoff, imports or nested agents; owner-reported green tests were not treated as independent proof.

The same implementation owner has a narrow correction window in `pitch.rs`, its authored tests and `PITCH-ANALYSIS.md`, retaining `60327573` unchanged. It must reproduce the 8 kHz fractional-period harmonic octave error, correct interpolated CMND trough acceptance while retaining raw-difference period interpolation, and test a bounded low-rate/range/threshold grid without widening signal assertions. A separately tested incremental pin and a new full-context independent review are required before integration. The optional raw-difference duplication cleanup may accompany that numerical correction only within these owned paths. Session/job/UI registration, real-vocal evidence, pitch editing/correction/warp/tuner, and all P1/M1 composition gates remain open; no parity status or counter changes.

# Namespace prototype remains isolated

M1's next granted artifact window is only three excluded standalone diagnostic source/build/plan files and an ordinary non-elevated MSVC/Windows-SDK compilation. Execution of SYSTEM/medium roles, actors, namespace objects, privilege changes and CI dispatch is not granted. The proposed experiment requires an externally approved disposable environment and explicit verified tokens; local object probes do not establish global/local DOS namespace authority. Production `SourceFile`, its 65/1,105 handle caps, and the excluded app source remain unchanged.

# Fresh CI UI confirmation at `427ed0a7`

[CI run 37770742963](https://github.com/erivgout/windfall/actions/runs/37770742963) tests exact pushed source `427ed0a7a07f3cae41ab1b91133b5e317b0f3dc9`. The completed UI job `113289416573` passed **172 files / 2,662 tests**, including all four `timeline-hydration.test.tsx` cases. This is fresh remote confirmation of the reviewed asynchronous lookup repair, separate from the parent focused 64-test execution and earlier local full-suite passes. Raw log: `C:/Temp/windfall-427-ui-ci.log`, SHA-256 `08edfeb2673ddaad30ba5ef4366145d4647bbd32fde999d46d9267a3416f6ad8`.

Ubuntu Rust, simulator freshness, parity and all three platform binding jobs also passed. The macOS Rust job `113289416625` failed the same eight `Unsupported("VST3 bundle entry on macOS")` cases: **12 passed / eight failed / zero ignored** in realtime. Its analysis integration executable passed 30 tests and analyzer taps passed 16 with the existing optional CPU ignore. Raw log: `C:/Temp/windfall-427-macos-ci.log`, SHA-256 `ac703aa899b72e4df429b93f8622e9587a3878efa872f21af57ddf8808246da7`. Windows Rust remained live at this observation; no Windows or whole-run verdict is inferred.

The Mac proposal is frozen as documentation-only `f78a82e5ac9e94af24beb68fc3a173d3ea96e7c8`, direct parent `5d3fe8e7`, and has a new independent read-only design audit. No Mac loader/fixture implementation was included in `427ed0a7`, so the repeated refusal is expected evidence of the remaining work, not a failed candidate repair. Source grants and actual native Mac validation still follow the audit.

# Scoped implementation progress and further dispatch

The P1 owner reports the corrected full isolated engine target at 130 library passes and 280/281 integration passes, including all 42 unchanged utility cases and 17 readiness cases. The remaining sampler case expects implicit garbage destruction from a transport accessor. Parent granted only the exact observation after the second transport call in `sampler_reload_evicts_cache_but_old_voice_bank_stays_charged_until_control_retirement`: use the already granted outer preallocated retirement carrier, collect and destroy it after the accessor has returned, before the unchanged one-bank assertion. The earlier two-bank assertion and every original source/cache/audio/allocation assertion remain untouched. No production sampler, Processor, render, Session or support expansion follows. A tested fixed-source checkpoint and independent review are still required; this is owner execution on isolated source, not parent acceptance.

N4's read-only diagnosis has a deterministic actual-native owner-turn mechanism proof on exact copied `7027569f` bridge sources: the no-later-old-turn baseline passes; after a helper claims old sequence/epoch, host reset cannot revoke `HELPER_WRITE`, and finishing that old block restores native old-epoch ownership before the next capture. The unchanged capture oracle then returns the exact ownership error. This forced in-process native diagnostic is one pass/one RED; the original CI supplies separate-process failure evidence. No production source changed. The owner must propose versioned authoritative reset publication and conservative helper adoption before a repair grant; the capture guard and processed-epoch/no-DSP proof must remain intact.

The next independent E4 implementation is bound to `gpt/t3-lofi-e4` at exact `427ed0a7`. Its owned new DSP module/tests/document and one additive module declaration cover genuine level quantization, sample-rate reduction, drive and filtering. Shared DSP registry, project/engine/Session/UI/IPC and artifacts remain closed; resource/signal/allocator proof precedes a later serialized integration window. This is implementation toward `fx-fruity-squeeze`, not a row closure or substitution for the remaining nonlinear/guitar/rack family.

# macOS bundle proposal design audit R1

Immutable documentation-only pin `f78a82e5ac9e94af24beb68fc3a173d3ea96e7c8`, direct parent `5d3fe8e7`. The independent T3-owned GPT-6.1-Sol audit examined the complete proposal, existing source/fixtures, prior accepted Linux context and primary APIs. It reported six pre-implementation contract gaps; no Mac source window is open.

## Independent design report

The proposal is implementable, but **six lifecycle contract gaps should be resolved before a source grant**. These are design findings, not observed candidate failures.

Verified `f78a82e5ac9e94af24beb68fc3a173d3ea96e7c8` directly parents `5d3fe8e7c7f35cb1c0fd4538cfbc381796296387`; diff/log contain only the proposal document. I read its complete Git object, the requested host/fixture/tests, and relevant architecture, roadmap and prior reviews. All proposal lines below refer to that pin.

1. **P1 — Reentrant retirement has no valid outcome (143–151, 194–207).** Load B holds the executable reservation; native entry/factory code reenters the host and drops loaded ticket A’s final owner. A’s Drop cannot return the prescribed reentrant Load error. Waiting deadlocks; abandoning cleanup leaks; recursively exiting overlaps native lifecycle calls. Specify reservation-owned deferred retirement, preserving A’s factory/bundle until native code returns, then draining cleanup on the proper owner before notification. Admission must reserve its capacity beforehand.

2. **P1 — “Other callers wait” permits dependency cycles (198–202).** Thread T1 reserves executable A and reenters loading B; T2 reserves B and reenters loading A. Neither is same-thread reentry into its occupied gate, so both wait forever. A native callback joining a worker that loads the occupied executable has the same problem. Define cycle-free contention handling, such as Busy refusal for contended loads, plus deferred retirement where Drop cannot refuse. Releasing mutexes alone does not resolve logical-reservation cycles.

3. **P1 — Reservation coverage starts too ambiguously (159–180, 194–208).** CF executable loading runs initializers before returning; concurrent retirement can therefore overlap constructors even if entry itself is serialized. Missing-export cleanup also releases a loaded bundle without ever obtaining an entry ticket. Require the reservation before executable loading/symbol lookup, through every loaded-bundle failure release. Audit resolver CFBundle creation/release too: the cited CF implementation contains CFPlugIn registration paths and final-release unloading. Prefer fresh directory metadata for preliminary resolution, then create/validate the native owner under reservation. [Apple initializer contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dlopen.3.html), [CF implementation](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.c).

4. **P1 — Retired source identity lacks an admission rule (163–170, 210–214).** Load A, retire all host tickets while another owner keeps its image resident, then replace the executable at the same path with different size/date. Fresh plist and executable URLs still agree; “conflicting live” stamps no longer refuse B, while CF loading can succeed using resident A. “May require a fresh process” must become a deterministic refusal policy. Preserve known executable/bundle-context history after retirement and reject changed generations until process restart unless native evidence establishes safe replacement. Revalidate after waiting. [CF’s already-loaded behavior](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.h#L256-L261).

5. **P1 — Logical reservation unwind is unspecified (176–180, 204–209).** A host-side Rust panic after entry/factory acquisition occurs while mutexes are released; mutex poisoning need not mark the reservation or wake waiters. Specify a non-panicking reservation guard with explicit ownership stages: factory release, attempted exit exactly once, CF release, terminal state, notification. Cleanup must survive poisoned metadata locks. Foreign exceptions or aborting FFI panics remain process-containment events.

6. **P2 — Registry lifetime and admission are unbounded (194–207).** Distinct paths, especially exit-refused bundles, accumulate permanent metadata. Evicting refused/history entries would silently restore admission; removing an occupied entry can create competing gates. Define entry/byte/ticket limits, refuse before native preparation when capacity is unavailable, and retire only entries proven safe to forget. Existing tickets must retain cleanup capacity; metadata must never become a permanent native owner.

The ownership/API direction otherwise holds: fresh metadata comparison, mandatory exports before entry, balancing refused SDK entry, owned factory before exit before CF release, and instance-held module lifetime. These agree with [Steinberg’s entry implementation](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/source/main/macmain.cpp). Optional interface improvement: keep factory and lifecycle owner in one private owned state, encapsulating retirement ordering.

Atomic fixture publication, differing executable names, C++ ABI ledger variants and the raw owner-guard adaptation are coherent. Target-header widths, Mach-O acceptance and unchanged Mac ThreadId/TLS regressions remain external execution gates.

The captured baseline confirms **12 PASS / 8 FAIL / 0 IGNORE**, all Unsupported. No candidate Mac execution is claimed. P1 preparation, N4 release/activation and the full 342-row goal remain open; preserved Windows/Linux evidence is unchanged. No edits, builds, tests, installations, handoffs or nested agents occurred.

The initial response was documentation-only. The subsequent delivery workflow correction grants the same owner coherent implementation and tests addressing all six concrete findings, retaining both frozen pins. Actual target headers, native bundle/ABI/owner-thread execution, P1 and editor/platform gates remain open. The fresh `427ed0a7` Mac refusal above remains baseline evidence, not a candidate failure.

# P1 engine stage 1 R1

Fixed source: `b4e07679a035c6e57afe74d8e9b2de232965cf4b`, sole parent
`937891439e4c9f268ddfa5cd91c1b7c6a9d4e8ff`. Both new independent tasks received
the original goal, prior design findings/responses, all utility/T1/N4/M1
constraints, exact source scope and owner evidence. Reports below are separate
and verbatim; overlapping findings retain each reviewer's severity.

## Standards

**Standards verdict: changes requested — one documented ownership breach; one optional heuristic concern.**

Verified source `b4e07679a035c6e57afe74d8e9b2de232965cf4b` and sole parent `937891439e4c9f268ddfa5cd91c1b7c6a9d4e8ff`. Their log contains one implementation commit; the diff is exactly **12 paths, +3458/−418**: eight engine files, new `project_preparation.rs`, two authorized test migrations and the preparation-document append. The prerequisite merge was treated as context.

- **P2 — Live attachment replacement violates endpoint lifetime ordering.** [project_preparation.rs:547](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/project_preparation.rs:547) replaces `state.link` without requiring the previous Processor to have stopped. Entry rejects Starting/Closing but permits Running; unchanged generation checks likewise permit this replacement. Dropping the returned retirement carrier releases the old control endpoints while its Processor can remain alive. The documented rules require direct replacement without shutdown ordering to refuse `StaleStream` ([PROJECT-PREPARATION.md:760](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:760)) and prohibit silently replacing a live attached link ([line 847](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/docs/PROJECT-PREPARATION.md:847)). Enforce that admission condition before construction and revalidate before mutation, preserving abandoned-Processor retries. This is source-traced interface noncompliance, not an executed CPAL failure.

**Optional Fowler judgment:** [rack.rs:1065](C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-project-preparation-p1/crates/windfall-engine/src/rack.rs:1065) duplicates the constructor shape at line 995. Centralizing shared initialization would keep stage storage and selected-count changes together while preserving their intentional capacity difference.

The borrowed readiness/lease/retirement interface otherwise hides controller state and keeps ownership local. No additional documented-rule or legal breach identified. Original test assertions remain intact, including the exact three-line sampler migration. Initial R3/R5, invalid-meter and sampler failures remain documented with their corrections.

Read-only Git-object review; no edits, builds, tests or agents. Reported **131/281 passes** and strict checks remain owner evidence. Deferred Session/N4 integration remains open scope. The Spec axis is not assessed or reranked here.

## Spec

**Hold stage1 acceptance: two spec findings.**

Verified source `b4e07679a035c6e57afe74d8e9b2de232965cf4b`, sole parent `937891439e4c9f268ddfa5cd91c1b7c6a9d4e8ff`. The log contains one implementation commit; its diff is exactly 12 paths, **3,458 additions / 418 deletions**. Locations below refer to that source.

- **P1 — Live attachment silently replaces control endpoints.** `crates/windfall-engine/src/project_preparation.rs:490` rejects Starting/Closing but accepts Running with a live Processor. At line 547 it replaces that Processor’s Link; `controller.rs:626` immediately drops the returned retirement carrier. Trigger: attach again while the previous Processor remains alive. Both Processors can publish shared transport/native selection, and the old Processor becomes the final ring-endpoint owner, so queued native-bearing messages can be destroyed during its later teardown. This violates `docs/PROJECT-PREPARATION.md:749–762`: retain control endpoints through Processor destruction and refuse still-attached replacement with StaleStream before mutation. Reject a live existing consumer before construction/admission; preserve abandoned-processor retry compatibility.

- **P2 — Moving a restored effect loses predecessor ordering while retaining the old transfer.** `crates/windfall-engine/src/plan.rs:531` carries an outgoing reservation even when its active ID moved elsewhere, while line 599 links the successor only within the same track. `state.rs:1467–1476` then restarts the moved successor’s insertion with no predecessor remainder. Trigger: remove heard g0, restore g1, then move g1 to another sounding track before g0 finishes. g0 continues processing the old track while g1 can become audible after only its own priming. This violates the preserved immediate old-track switch (`docs/ARCHITECTURE.md:246`) and concrete predecessor-wait contract (`PROJECT-PREPARATION.md:520–530`). Preserve the existing move policy instead of retaining that old-track reservation. The unchanged R5 movement case waits for departure completion, so it does not exercise this trigger.

These are source-traced counterexamples, not executed reproductions. No edits, builds, tests, imports or delegation occurred. Owner-reported passes remain owner evidence. Deferred Session stage2 integration is not counted as a finding.

The same owner is repairing these concrete findings while completing coherent
Session stage 2 preparation/publication/retirement. The immutable stage-1 pin
stays available. Attached legacy refusal is not accepted as the final production
route. These findings prevent P1 acceptance; no P1 row or global lock-ownership
closure follows from the owner test counts.

# RAM, lo-fi and authoritative reset final reviews

## Toolbar RAM

Fixed initial source `0e81f508`, follow-up `7f180337`, base `125d709c`.
Standards R1 reports zero findings; Spec R1's accessible accounting-scope P2
was repaired and the bounded Spec R2 is clean:

R1 Spec P2 is resolved at `7f1803379c3e2e439b1591733e20c067ea845f34`.

The [label at line 51](/C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src/features/transport/memory-readout.tsx:51) includes resident desktop-host meaning and webview/plugin-helper exclusions for both numeric and unavailable states. The [exact accessible-name assertion](/C:/Users/ewhee/.t3/projects/windfall/apps/desktop/src/features/transport/memory-readout.test.tsx:37) checks that wording.

No remaining concrete Spec blockers or regressions found in the bounded follow-up diff.

This was a read-only audit of pinned blobs; I ran no tests. Reported test passes remain owner evidence, and macOS/Linux runtime verification remains an external platform gate.

## Lo-fi reduction

Fixed complete source `be6c9811` through immutable foundation `d073d6e3`,
base `427ed0a7`. Parent composed those increments at `9c565d8b` and
`13822c08` without importing other owner's work.

### Standards

**Standards: 0 findings.** Reviewed `427ed0a7…be6c9811`, including ordered foundation `d073d6e3`, from pinned source blobs.

No documented contract breaches or meaningful heuristic findings identified. Fixed callback storage, parameter sanitization, append-only registration/defaults, and generic document/UI integration follow the [architecture contracts](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-lofi-e4/docs/ARCHITECTURE.md:229) and [DSP interface rules](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-lofi-e4/crates/windfall-dsp/src/effect.rs:34).

Read-only audit; worktree remained clean. No builds, tests, imports, browser actions, nested agents or artifacts. Owner verification claims were inspected, not reproduced.

### Spec

No supported **Spec findings** for `be6c981166c500d04fa6a9aa8b0637d943819175` against `427ed0a7a07f3cae41ab1b91133b5e317b0f3dc9`, through immutable foundation `d073d6e3`.

The source covers distinct quantization, rate reduction, drive, timed preservation/replacement, resonant pre/post filtering, and all 16 controls through registration, persistence/history, automation, engine rendering, both stem modes, and generic UI editing. Replacement relationships and filter placement align with the [primary manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Fruity%20Squeeze.htm).

The accepted finite activity and intentional aliasing policies are documented. Parent-owned artifacts and physical/platform qualification remain outside this acceptance.

Read-only audit completed; tests were inspected, not executed. Worktree remains clean.

## Native reset authority

Fixed host-only source `2dc22609`, direct `6b228e12`. Parent composed
ONLY the nine-path increment at `a0e71327`; no production `19f` routing
or `6b` facade was imported.

### Standards

No documented-standard violations found. One nonblocking heuristic:

- **P3 — Possible Duplicated Code (judgment call):** [reset_timeline](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-plugin-process-bridge/crates/windfall-plugin-host/src/bridge/adapter.rs:314) and [fail_timeline](/C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-plugin-process-bridge/crates/windfall-plugin-host/src/bridge/adapter.rs:349) repeat `self.health.acknowledged_generation = 0`, `self.completed_proof = None`, and the Signals proof reset. Future proof fields could drift between these paths. Consider a private proof-invalidation helper while preserving their distinct transition behavior.

Pinned-source review only: `6b228e12...2dc22609`, exactly nine paths. The original reset test body is unchanged. No edits, builds, tests, imports, browser use, or nested agents.

**Standards result:** 0 documented violations; 1 optional P3 heuristic. No blocker identified for this host-only leaf; production `19f`, P1, and full N4 acceptance remain outside this review.

### Spec

No actionable Spec findings in `2dc226099d18c02fa4d7a64ac2ab90e8331b6dca` against exact parent `6b228e1216d6e33757faa2d5970fba66f935039d`.

Source supports the requested reset contract: proof invalidation before the sole strong authority CAS, preserved HELPER_WRITE ownership, stale retirement without DSP/DONE, old-epoch proof retention, capture fences after native recovery and under the cache mutex, and Hello3/ABI4 compatibility rejection. The original epoch2/proof0/health0 oracle remains unchanged.

Read-only pinned-blob audit; no edits, builds, tests, imports, browser, or nested agents. This verdict covers only the host reset increment. Production19f routing, P1/meter-render gates, and full N4 acceptance remain separate.

The optional P3 proof-reset duplication heuristic is retained as a maintenance
note; it does not block the tested reset leaf or require another review round.

## Parent combined execution

The actual parent composed source passed all DSP targets: 135 library, 152 DSP,
13 filter, six registry and 21 lo-fi tests (327 passes; five existing optional
measurement ignores). Four project lo-fi and four real engine lo-fi tests pass,
including all 16 automated controls, live/offline and both stem modes.

The full shared-WASM UI run passed 2,680 tests and failed one stale literal
inventory assertion: the new effect expands automation ranges from 149 to 165.
Only that expected count, its nine-sample total and comment were updated;
original mapping/tolerance assertions remain. The affected curve/lo-fi/access/
RAM files then passed all 84 tests. This is a full-run result plus a focused
correction, not a claim of another full run after the one-line inventory repair.
TypeScript and full desktop ESLint pass. Fresh parent generation contains 186
binding/artifact files and a 2,001,474-byte simulator, SHA-256
`16dc0d3e2b28974459f1d0cc4b630a870c540da0990a86de7d3dfa1996dd4a7e`,
source inputs `5aee11f9b285c10cbc506924113f275086933f834a52d11011ca157915a7d0c6`.
The freshness check passes; no child-generated artifacts were imported.

The ABI4 root check used a separately prebuilt current fixture and executed
all ignored native library regressions explicitly: **82 passed, zero ignored**.
Actual bridge processes passed **31 cases**; the one ignored authentication
role was invoked by its passing parent tests. Realtime passed **20 cases**.
The original CI reset oracle remains unchanged and passing. Native log:
`C:/Temp/windfall-root-abi4-native.log`, SHA-256
`a220a8d64786c5c5310bfa287eddf8cf7d2753d4517f0198bfc322819a277305`.

Strict five-package all-target Clippy, host all-feature/all-target Clippy and
workspace formatting pass. An initial overly broad desktop all-feature command
entered optional ASIO and stopped for missing libclang; it is not a source
failure or ASIO validation result. The final command uses the unchanged
production desktop features and the host's required feature matrix. No SDK or
dependency was installed to expand that unrelated validation scope.

## GitHub Actions budget constraint

The user explicitly prohibited further Actions minutes. Automatic `ci.yml`
is disabled remotely and no jobs remain queued/running. Run `37783570574`
had already completed before the cancellation command arrived; no new run was
dispatched. Its Ubuntu Rust/UI/bindings/parity/freshness passed; macOS executed
the new RAM tests successfully before its separate eight VST3 bundle-entry
refusals. The overall run remains failed. All new combined checks above are
LOCAL, and subsequent pushes preserve disabled CI and use `[skip ci]`.

Full P1 Session readiness/retirement, production N4 activation, ordinary-global
Windows namespace authority, macOS native loader execution, hardware/editor/
installer and remaining roadmap gates stay open.
