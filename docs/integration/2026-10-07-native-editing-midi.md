# Native MIDI, editing, slicing and plugin ownership integration

This local integration combines MIDI hardware `98a914ca`, immutable audio editing `de5da276`, clip slicing `eab66af9` and bounded native plugin ownership exchange `d2bb8902`. Their merges are `7e8984ea`, `d6f5aaba`, `f060022f` and `11e1a54b`. The full-project roadmap `aee8be65` is integrated through `3e188b67`. These are development changes after the immutable `v0.1.0-alpha.1` release, not an updated installer.

## Review repairs

- The combined bindings were regenerated: 146 files, including native audio-editor and MIDI wire types. The editor now imports generated types. The pinned browser WASM was rebuilt to 1,634,799 bytes and exercises the real Rust `slice_analyze` and `slice_command` exports.
- Audio-editor and slicer Apply replies now check the existing UI document-generation signal and component lifetime before applying patches or changing selection. Deferred-reply tests replace a project with reused revisions or unmount the dialog before resolving the old result. Analysis/opening replies have equivalent stale-result guards.
- Native and browser slicing explicitly refuse tempo automation before analysis or mutation. Constant-tempo source-offset math cannot preserve slices across a tempo map. Grid/transient refusal tests retain the document/history and original native WAV bytes.
- A MIDI/sampler integration regression checks forward and ping-pong loops beyond source duration, sustain/key release, pedal-up release, retrigger and panic. Every measured callback has zero allocation/reallocation/deallocation calls.
- A macOS hosted CI timing failure is repaired with deterministic realtime-feed deadline tests and a bounded live delivery/lifetime test. See [CI portability](../ci-portability.md).

## Checks observed

Before the review repairs, the combined source passed strict workspace/all-target Clippy, the full Rust workspace suite (1,657 passed, eight intentional ignores), plugin-host all-feature tests (97 passed) and strict all-feature host Clippy. The full UI suite passed 2,129 tests across 136 files.

After the editing repairs, the three editor/slicer UI test files passed all 24 tests. UI lint and the production TypeScript/Vite build passed. Native slicing passed nine tests, MIDI hardware passed eight tests, realtime-feed tests passed three tests and the session playback suite passed 14 tests. Strict all-target Clippy for desktop/engine passed, including a follow-up desktop check after the scheduling repair. Generated WASM freshness, parity freshness and Rust formatting passed. These focused checks supplement the earlier full suites; they are not a claim that a second full suite was run.

The parent updates parity to reflect partial workflows: live hardware input/output/settings, selected-clip audio editing, detector/slicing foundations and plugin ownership. No additional row is marked done merely because a foundation exists. There are 342 rows: 59 done, 34 in progress, 247 todo and two previously justified won't-do rows.

## Piano, transport and runtime follow-up

Selected-note transformations `666e1394` are integrated through `9f716d8a`.
They share one atomic Rust command across desktop and browser: quantize with
strength/original grooves, legato/staccato, grid chop, compatible glue, strum,
time/pitch flip, transpose/key-range limiting and velocity scaling. See
[selected-note tools](../PIANO-TOOLS.md) for limits and remaining phrasing work.
The combined bindings now contain 149 files, and the pinned browser WASM is
1,722,263 bytes. Parent checks passed 12 native piano tests and 130 UI tests
covering tools, piano-roll integration and exhaustive shared-WASM commands.

CLAP repairs `320fe342` are integrated through `c2b8eb3c`. Immediate releases
and panic have reserved bounded capacity, dialect-specific native event lists
are sized for expansion, accepted parameter updates wait for available space,
and retirement flushes ordered bounded chunks. Native fixture regressions cover
saturation, retained UI key ownership, final accepted state and zero callback
allocation. Parent processing/state/realtime checks passed 43 tests, followed
by all eight desktop MIDI tests. Strict all-target Clippy for project, host,
engine and desktop and workspace formatting passed. See
[CLAP saturation](../plugins/clap-saturation.md).

The transport now has a review-and-Apply tap-tempo dialog. Shared store guards
also discard edit/history/snapshot replies from a replaced project and avoid
snapshot rollback after a newer patch. The focused transport/tempo/store suite
passed 73 tests; UI lint and production TypeScript/Vite build passed. See
[tap tempo](../TAP-TEMPO.md). These checks supplement the earlier full suites.

Editing review round two closed the original WASM and tempo-automation
findings but reproduced a new inspector lifetime regression: publication of
Apply's own replacement patch can unmount its dialog before the IPC reply and
leave selection on the removed clip. A dedicated task is fixing stable
operation ownership and adding event-before-reply inspector regressions.
Piano rows remain in progress until follow-up review; no unreviewed tool is
marked fully complete here.

## Remaining work and evidence limits

The initial CLAP findings have implementations and passing fixture regressions.
Independent follow-up review and the inspector lifetime repair still need to
finish before this local batch is pushed as verified progress.

## Browser library integration

Browser library `e2558567` is merged through `433234e0`. Four additive conflicts
in session fields, mock imports, IPC exports and WASM exports retained both
features' additions. Recursive indexing, Boolean/wildcard path queries and
separate persisted favorites/tags now coexist with editing, slicing and MIDI.
Native file tokens check root generation and filesystem identity for audition,
waveform and rack/playlist/replacement imports. See
[browser library](../BROWSER-LIBRARY.md) for bounds and deliberate limits.

The combined artifacts contain 155 binding files (153 TypeScript and two JSON)
and a 1,743,421-byte WASM
module. Parent browser/rack/playlist-import UI checks passed all 403 tests
across 20 files, followed by ESLint and the production TypeScript/Vite build.
Native library and sample-cache checks passed 16 and 14 tests respectively.
Shared IPC tests passed 52 tests and simulator tests passed 13. Strict all-target
desktop/IPC/simulator Clippy and workspace formatting also passed.
Independent library review is pending, so its three parity rows remain in
progress. The matrix now has 60 done, 42 in progress, 238 todo and two won't-do
rows. These are development-source counts, not release or platform readiness.

## Independent follow-up findings

Piano/transport review found late New/Open snapshots could bypass the shared
store guards, and Chop could prune new IDs before revision-gap recovery.
Parent reproduced seven initial failures, repaired event authority and dispatch
recovery ordering, and passed 66 focused flow/store/note-tool tests including
replacement during recovery. See [document replies](../DOCUMENT-REPLIES.md).

Runtime review closed the original CLAP release and retirement findings but
traced a further committed parameter loss when the adapter rejects enqueue
under saturation. The existing runtime owner is adding retained desired values
and a native regression; a competing runtime implementation was not started.

Browser review reproduced delayed token lookup importing into a replacement
project and refresh rebinding a pending tree selection; it also traced reuse of
an older project audio buffer after a changed file is refreshed. A dedicated
repair task owns all three paths, with targeted native/UI regressions and
unchanged-source deduplication/undo checks. No claim of completed library parity
is made while these findings remain open.

## Stable inspector ownership and utility effects

Inspector lifetime repair `7bd55ed5` is merged through `4cb9287a`. Its own
replacement publication no longer ends the dialog awaiting Apply; explicit
selection changes, closure and project replacement still revoke ownership.
The implementation task passed 496 editor/slicer/playlist/WASM UI tests,
including 11 actual-inspector regressions, plus typecheck and focused lint.
Independent editing round three and document-reply round two are pending.

Seven measured utility effects `3181d87a` are merged through `5254a40e`.
The parent regenerated 163 binding files (161 TypeScript and two JSON) and the
shared browser WASM to
1,775,678 bytes. [Utility effects](../UTILITY-EFFECTS.md) records the worker's
signal, smoothing, allocation, latency, aliasing and throughput evidence;
combined parent validation and independent review remain pending.

The roadmap's accounting audit was reconciled against source: existing EQ
shows a parameter response rather than a live spectrum; existing limiter lacks
the row's compression/gate/history display; existing delay lacks modulation.
Those three rows now explicitly retain their implemented core processing while
listing remaining behavior as in progress. Utility rows also remain in
progress pending review. Current totals are 57 done, 52 in progress, 231 todo
and two won't-do rows, preserving all 342 requirements.

## Portable files and combined validation checkpoint

Portable ZIP archives and numbered saves `099969f2` are merged through
`bfe1be6c`. The session/commands/backend/actions/report UI are integrated.
The dependency conflict preserves library file identity and native staging
support. Ordinary and archive Open both retain the parent's loaded-event
authority instead of reloading a delayed reply. The archive crate is outside
the simulator dependency tree; the 163 bindings and 1,775,678-byte WASM remain
current after this merge.

The implementation task passed eight archive-crate tests, eight session archive
tests including native CLAP capture, six numbered-save tests, 25 existing file
tests, two plugin tests, three recording-guard tests and 31 UI tests. Those
are worker evidence, not parent combined results. Independent archive review
and full parent UI/Rust validation were started at `bfe1be6c`; their results
are still pending at this checkpoint. The matrix holds both file rows in
progress, totaling 57 done, 54 in progress, 229 todo and two won't-do rows.

All changes remain local pending open review repairs and validation. The
private repository and immutable `v0.1.0-alpha.1` release are unchanged.

At this portable-file checkpoint, production VST3 additions remained disabled;
the later delivery below supersedes that gate state. MIDI note recording/
controller mapping/sequenced output, playable slice mapping, advanced audio-
editor tools and independent sampler stretch remain unfinished. Physical MIDI/
audio hardware, installed native UI and new macOS/Linux desktop workflows have
not been verified by these synthetic tests. Hosted CI evidence for the earlier
sampler/portability commit is recorded separately from these unpublished changes.

## Recovery follow-up and VST3 delivery

Editing review round three closed the inspector lifetime finding. Its delayed
browser-token wrong-document reproduction is the same P1 already assigned to
the browser repair owner. Document-reply review round two closed the New/Open
and original Chop-selection findings but reproduced a new starvation case:
dispatch awaited later recursive recovery even after its own result was mirrored.

Parent repair `314ac502` waits for its captured generation and own revision,
while later recovery proceeds independently. Deferred real-WASM regressions
cover continuing edits, immediate replacement cancellation and failed recovery
followed by retry. The starvation case failed before repair. After repair, 63
focused tests, ESLint and TypeScript/Vite build passed; the full combined UI
suite at that source passed **2,274 tests across 148 files**. The prior full run's
only failure was an inventory count expecting 122 automation ranges rather than
141 after the utility additions. The forward/inverse value checks were retained.
Independent document-reply review round three closed starvation and preserved
the earlier fixes, with 23 pinned in-memory checks passing. No actionable issue
remained in revision waits, listener cleanup, replacement ordering, failed-fetch
retry, created-ID following or loaded-event authority.

Portable review found a P1 where ZIP parser fallback can select metadata outside
the preflighted directory, and a P2 where a timestamp-shaped backup base name
produces mismatched numbered-save sample roots. One dedicated repair task owns
both bounded regressions and fixes. Utility review found three P2 issues in
asymmetric-delay unbypass warmup, rapid delay retargeting, and UI chain latency
accounting. Another dedicated task owns those repairs. Their rows remain in
progress.

The first combined native suite at `bfe1be6c` passed 1,405 tests with five ignored
before stopping at the VST3 scanner's six-versus-seven fixture-class assertion.
The source defined six classes, but the test copied a seven-class DLL from the
older fixture cache. Removing only the fixture package's cached build artifacts
(23.3 MiB) and rerunning that scanner test passed. No assertion was relaxed.
The full suite had not reached its Clippy/format stages; this is not a native
workspace-green claim.

VST3 delivery `7476b074` was cherry-picked as `72718071`. Scanned VST3 additions
are now enabled. Deferred native state capture carries token/revision/binding
identity through recording exclusion; queued owner jobs cancel safely, and
already-running native calls are joined through recovery. Fallible deactivation
returns exact ownership on refusal. Bounded desired parameter/current-note
reconciliation also fixes the independently reproduced committed CLAP parameter
loss under saturation. The worker passed 106 host, 202 desktop, 314 engine and
six plugin UI tests, plus scoped strict Clippy, formatting, types and lint.
These are worker results at the feature commit. Independent runtime review round
three found three additional P2 issues: save-before-drain capture can replace a
committed parameter with an older native value, bundle validation stamps the
directory rather than its inner binary, and full multi-channel panic expansion
can exceed native event capacity. The same runtime owner is reproducing and
repairing them in its existing isolated T3 worktree. Live parameter delivery and
the original CLAP panic/retirement findings remain closed. A fresh combined
native suite at `72718071` is still running.

The generated 163 bindings and 1,775,678-byte WASM remain current: no exported
model/IPC/DSP descriptor or simulator dependency changed in the recovery/VST3
repairs. Parent parity and simulator freshness checks passed. No new release,
tag, visibility or access change was made. The batch remains local pending
open review repairs and appropriate combined checks.

## Scale and stamp integration

Scale/stamp source `46de77e3` merged cleanly as `73149138`. Root/scale preferences,
opt-in highlighting and pitch snap, rigid-group pitch policy, and atomic chord/
single-octave scale stamps use existing project commands. No exported field or
renderer shader changed. The worker passed 265 focused tests and browser/
Canvas 2D/WebGL2 checks; WebGPU hardware and native Tauri windows remain untested.

Parent checks at the merged source passed 281 tests in 15 piano/canvas/playlist/
flow/store files, ESLint and the production TypeScript/Vite build. Six plugin UI
tests and their focused lint also passed after the VST3 integration. Independent
review passed 23 focused tests and additional real-WASM selection/lifetime
checks, then reproduced two stamp defects: a choice awaiting menu exit can arm
after blur/tool-change cancellation, and modifier keys can restore a preview
after the pointer leaves the grid. The original scale/stamp owner is adding
focused regressions and repairs. Scale guidance/snap has no actionable finding
and is accepted; the stamp row remains in progress.

The parent T3 preview reconnected on port 5190. At the merged source, the scale
menu and chord choices were present, and Escape cancelled an armed major-triad
stamp with Undo still disabled. Tap tempo showed four taps and 43.74 BPM while
the project remained at 128 BPM with Undo disabled. Apply changed the displayed
tempo to 43.74 and marked the document dirty; one Undo restored 128, the clean
title and disabled Undo, with Redo available. This is browser/real-WASM UI
evidence, not a physical device or native-window check.

Six accepted piano rows are now done: quantize, glue, strum, key limit, flip and
velocity scaling. Articulation retains pending portamento; chopping retains
pending custom patterns despite its reviewed grid workflow. A new isolated T3
task owns custom chopping, arpeggio, flam and rhythm-reshape transforms. Current
accounting is 64 done, 49 in progress, 227 todo and two won't-do rows, preserving
all 342 requirements. Open native/library/archive/utility findings still prevent
publishing this batch as verified progress.

## Native validation checkpoint before further repair merges

The combined native suite at `72718071` completed with **1,777 passing tests and
nine ignored tests**, then passed strict workspace/all-target Clippy and workspace
formatting. Rust source remained unchanged through the scale/UI documentation
commits at `8ff536a5`. This is Windows headless/fixture evidence; it does not
close the independently found runtime, archive or utility behavior gaps.

At `8ff536a5`, regeneration into a separate temporary directory exactly matched
all checked-in bindings and fixtures. The 163 files comprise 161 TypeScript
files and two JSON files; earlier references to 163 TypeScript files counted
the JSON fixtures as well. Simulator freshness and parity checks also passed,
with the WASM still 1,775,678 bytes. Subsequent source repairs require their own
freshness and appropriate combined checks.

## Browser, utility and stamp repairs; sampler integration

Browser repair `d90db39e` and utility repair `99792c90` merged as `1dc0b5e1`
and `bde3fd77`. Parent checks at `bde3fd77` passed 326 UI tests across 20 files,
ESLint and the production TypeScript/Vite build. Native checks passed six core
tests, 128 DSP unit and 149 DSP integration tests (three measurement ignores),
103 engine unit and 220 engine integration tests, 12 desktop library tests and
the 16-test samples filter. Scoped all-target Clippy with warnings denied and
workspace formatting passed. The samples filter overlaps two library tests;
these counts are per command, not an inflated unique-test total.

Regeneration matched all 163 checked-in binding files exactly (161 TypeScript
and two JSON). Simulator rebuild retained the 1,775,678-byte size but changed
its binary hash as well as its source record; both were committed as `1f4e1482`.
The three original findings in each feature were closed by independent round
two. Browser review found three further P2 cases: an outstanding source reload
can overwrite an accepted import, overlapping configured roots can reject a
valid tree token, and a delayed rack-drop reply can select a replacement-project
channel. Utility review found four further P2 cases: asymmetric live insertion,
zero-to-delay and downstream-delay compensation timing, and bypass-fade wet-tail
accounting. The existing owners are reproducing and repairing these cases.

Stamp repair `a58e9e83` merged as `d8146b42`. Its delayed menu choice now owns
cancellable completion, and idle preview updates require pointer presence while
captured drags retain modifier behavior. Parent checks at that merged source
passed 113 tests in six piano/flow files, ESLint and the production build.
Independent review remains pending, so the stamp row remains in progress.

Sampler source `68a30a61` merged as `42522322`. The shared file-opening conflict
retains archive cancellation and sampler preparation refusal, including both
test barriers. Native all-target desktop compilation passed after resolution.
Regeneration produced 166 binding files (164 TypeScript and two JSON) and a
current 1,794,146-byte document WASM. Full combined native/UI checks and an
independent regeneration comparison are still running; no completed sampler
parity claim is made. The implementation's signal, retained
bank budget, source lifetime, cancellation and native fixture evidence is in
[sampler stretch](../SAMPLER-STRETCH.md), with musical listening, installed native
UI/device operation and non-Windows evidence explicitly outstanding.

A T3 server restart cancelled the unfinished runtime, archive, browser, utility
and rhythm implementation turns and both outstanding reviews. Their worktrees
and edits survived. Each implementation owner resumed in its original bound
worktree; replacement independent review tasks received the complete original
brief, prior findings, repair response and preliminary cancelled-review evidence.
Cancelled or cut-off checks are not counted as passing. Current parity totals
remain 64 done, 49 in progress, 227 todo and two won't-do rows out of 342.
This batch remains local while open review findings are resolved.

Current GitHub inspection reported public visibility despite the user's private
repository instruction. Visibility was restored and independently confirmed
private. The alpha tag still dereferences to `157f96fd068b46adab046a6880ce27ea4b49aea1`,
and both published release assets are unchanged. The `artoomreinhart` write
invitation remains pending. Authenticated Git access through the existing GitHub
CLI credential helper confirms remote main remains `c37ae0b9`; no source push or
new release was made by this verification.

The combined UI suite after sampler/artifact integration passed **2,347 tests
across 154 files**, then passed ESLint and the production TypeScript/Vite build.
Native workspace validation and independent fresh-binding comparison remain
live. Independent stamp review closed the initial cancellation/preview cases
but browser-reproduced pending-choice right-click and rejected-press pointer
capture defects; the existing stamp owner is repairing both. Sampler review
traced four P2 transaction gaps: imported source commit before preparation
failure, no-op Apply discarding runtime recovery, history publication attaching
different cached audio, and a one-directional clip source guard missing new
attachments. Sampler and browser owners share the preparation helper/import
seam with explicit file ownership; no completed sampler claim is made.

Archive repair `d42c810e`, browser follow-up `24b241ff` and selected-note rhythm
tools `959ed0c0` have clean source-only deliveries. The archive worker passed 52
focused tests; browser passed 256 UI and 18 native tests; rhythm passed 21 native
and 121 UI/real-WASM tests. Each also records relevant static/build checks in its
feature document. These are worker results, not parent combined acceptance.
Archive round two and rhythm round one independent reviews have started at
those fixed commits. Root Rust source remains frozen until the current native
job completes, so these deliveries are not yet merged here.

A separate T3 owner is implementing persisted whole-application scaling through
the common canvas/control coordinate contract. The parity requirement also
includes native plugin windows, which remain required host work under the
runtime owner's separate ownership. The row is in progress; no application-only
completion substitute is recorded. Totals are 64 done, 50 in progress, 226 todo
and two won't-do rows, retaining all 342 requirements.

## Completed sampler baseline and next combined batch

The full native run at sampler source `42522322` and artifacts `186652c6`
completed with **1,822 passing tests and nine intentional ignores**, including
the real CLAP archive/save/export fixtures. Strict workspace/all-target Clippy,
workspace formatting and independent regeneration comparison of all 166 binding
files passed. The full UI baseline passed 2,347 tests in 154 files, ESLint and
the production TypeScript/Vite build. These results precede the next Rust merges;
they do not close the four independently traced sampler transaction findings.

Root T3 browser interaction also checked the sampler inspector: Independent
mode reveals preparation settings; Apply reports the desktop-engine requirement.
Undo and redo remain disabled, and reopening the channel restores its published
Tape setting. No project dirty marker appears. This checks the browser refusal
workflow, not native spectral playback or physical audio hardware. T3 snapshot
calls failed with a disconnected automation-client error; focused DOM inspection
and ordinary preview clicks succeeded without another browser system.

Archive `d42c810e`, browser `24b241ff` and rhythm `959ed0c0` merged as
`ee925c23`, `e841f73a` and `9bd413ed`. Runtime fix `fa3fd16f` was cherry-picked as
`19caf872`; second stamp fix `aab93768` was cherry-picked as `7f70315c`.
Combined bindings/WASM regeneration and affected native/UI checks are pending.
The rhythm review is clean: 121 focused UI/real-WASM tests and twelve independent
in-memory WASM case groups passed; native tests were inspected rather than rerun.
The four selected-note rhythm rows remain in progress until combined acceptance.

Archive round two closed the original parser and backup-shaped destination
findings but source/model tracing found two P2 cases: simultaneous sample carries
can overwrite a competing session's audio, and moving a numbered save can strand
samples reachable only through history. The existing archive owner is producing
compiled RED/GREEN regressions and repairs. Runtime round four and stamp round
three reviews are running with complete prior context at their delivered commits.
The original sampler/browser and utility owners continue their assigned repairs.

The N4 process-bridge owner has started isolated audio/state containment work,
with explicit ABI, nonblocking callback, fallback/PDC and supervisor acceptance
requirements. Existing host/factory wiring requires a coordinated edit window.
Native editors, packaged helper discovery, installed external plugins and other
platforms remain required evidence; scanner isolation is not audio containment.
No further source push or release was made by these integrations.

At `7f70315c`, regeneration produced 170 binding files (168 TypeScript and two
JSON) and a current 1,863,302-byte WASM. Affected native and UI checks are running;
the independent fresh-binding comparison follows the native checks. Current
totals are 64 done, 54 in progress, 222 todo and two won't-do rows out of 342.

Runtime round four source-traced closure of all three prior reproduction paths,
then found two further P2 cases: a deduplicated document parameter adoption can
hide its generation and suppress later processed automation during capture;
VST3 pending/editor/ordinary parameter points can exceed native storage while
readback and delivery acknowledgement still report success. These were not
executed native reproductions. The runtime owner is producing compiled regressions
and repairs, with the shared runtime/parameter fixture window reserved. N4 can
continue its new modules, module export and existing Windows mapping API features;
production runtime/helper routing remains deferred until those repairs land.

## Rhythm acceptance and transactional source follow-ups

The rhythm batch at `7f70315c` passed 13 archive, 306 project and 109 host
tests, plus 257 desktop tests before one test-only default-target assumption
failed. The scanner fixture path was corrected in `101c096e`; that exact test
passed with `CARGO_TARGET_DIR` unset. Scoped strict Clippy, workspace formatting
and an independent comparison of all 170 fresh binding files then passed.
The affected UI batch passed 536 tests in 29 files, ESLint and production build.
This records the initial 257 desktop passes and subsequent single-test result,
not a second full 258-test run. Logs are the local temporary
`windfall-root-rhythm-combined-native.log`, `windfall-root-rhythm-native-resume.log`
and `windfall-root-rhythm-combined-ui.log`.

Actual T3 preview interaction verified Flam changes four selected notes to
eight, and one Undo restores four. Invalid Before placement at tick zero
leaves history and dirty state unchanged. Combined acceptance and clean
independent rhythm review close Chop, Arpeggiate, Flam and Rhythm reshaper.
Parity now records 68 done, 50 in progress, 222 todo and two won't-do rows,
retaining all 342 requirements. Native hardware audio was not exercised.

Sampler repairs `3f2a70f9`/`fcbd0599` and composed transactional import routing
`6538722c` were cherry-picked as `499c16a6`, `5a3e76dc` and `282b62a3`.
Portable save repair `dc57588a` was cherry-picked as `e9fdcb70`, preserving
both archive cancellation and sampler preparation/install guards. Application
scaling `f418da57` was cherry-picked as `9b3001f4`; its full parity row remains
in progress because native third-party editor scaling is still required.

Independent sampler round two closes the original four cases and passed 22
sampler-filter tests plus the 51-case import barrier on the existing composed
binary. It source-traced a new P2, independently confirmed by browser round
three: an identical import cannot attach a restored settled missing source
because its musical command is a no-op. The helper owner is repairing guarded
runtime attachment; the import owner is adding ordinary/checked real-file
regressions. History, dirty state, exact held sources and pending-loader
ownership must remain unchanged except for successful runtime source recovery.

Browser round three also reproduced a P1 through actual picker/New/Open flow
functions: opening another project while a rack/replacement file picker is
pending lets its result start an import in the new project. The browser owner
is adding action-entry generation guards and four corresponding regressions.
The review also source-traced case-sensitive cache provenance falsely refusing
unchanged Windows path aliases; the browser owner is adding native casing and
cache-eviction regressions while retaining file identity/length/timestamp guards.
Portable round three and scaling round one reviews are running at their fixed
commits. Root combined native/session checks, including the real CLAP archive
fixture, and the full UI suite are running at `9b3001f4`.

Runtime round four compiled RED reproduced both new capture-adoption and native
point-overflow cases. Its first GREEN has passed all four real CLAP/VST3 effect
and instrument capture/save/reopen roles and native point regressions with zero
guarded callback allocations, reallocations and frees. Final focused checks and
source delivery remain pending. Stamp round three and utility round three
findings remain assigned to their existing owners. No new source push, tag,
release or asset replacement has occurred in this batch.

Regeneration after the additive project history helpers rebuilt the document
WASM successfully. Its binary remains exactly 1,863,302 bytes; its source hash
record was updated, and `check-sim.mjs` passed. Independent fresh comparison of
the complete binding inventory follows the current native checks. Application
scaling preview navigation confirmed the clean demo at 100%; a subsequent click
timed out and the tool explicitly reported no automation host. No new root
125/200% interaction or physical-window proof is claimed from that attempt.

The next isolated T3 owner has started T1 meter maps, timeline markers, selected
loop/playback/export regions and region zoom. Full arrangements, linked tracks,
grouping and per-pattern signatures remain separate required follow-ups. Its
shared Rack/State/runtime and session source windows remain reserved to the
active repair owners until narrow integration hooks are coordinated.

The full UI check at `9b3001f4` finished with **2,450 passing tests across
159 files**, then passed ESLint and production TypeScript/Vite build. Current
native archive tests passed 13, project tests passed 308 and engine checks
passed 49 sampler plus three source-pool and three bank-budget tests. Desktop
test compilation succeeded; the root orchestration script initially selected
the wrong Cargo target kind (`lib` instead of Tauri's `rlib`/`cdylib`/`staticlib`).
Selection was corrected to the actual `windfall_desktop_lib` test target and
the already-built executable is running directly, including native fixtures.
No production change or repeated earlier test run was needed for this script
correction. Later strict checks and fresh-binding comparison remain pending.

Independent scaling round one found two P2 cases beyond the passing suite:
200% scale collapses the browser tree, and independently capped canvas densities
misalign large piano ruler/value layers. Both are assigned to the scaling owner
with narrow browser-layout ownership coordinated separately from import work.
Portable round three closed the prior collision/history reproductions, then
source/model-traced historical output-path aliasing, local ZIP header/extent
validation and a missing already-written-path error. The portable owner is
checking real reachability/reader behavior and producing bounded regressions;
no new exploit or Rust reproduction is claimed from the review's models.

The resumed native command finished successfully: **all 278 desktop tests
passed**, including `selected_native_plugin_capture_archive_and_export_round_trip`.
Scoped all-target strict Clippy for project/archive/engine/desktop, workspace
formatting, independent comparison of all 170 generated files, WASM freshness
and whitespace checks passed. Together with the preceding 13 archive,
308 project and 55 engine sampler/source/budget cases, these validate the fixed
integrated source at `9b3001f4`; they do not close the independently traced
import, scaling, runtime, utility, stamp and portable edge cases still assigned.
The local logs are `windfall-root-imports-combined-native.log` and
`windfall-root-imports-native-resume.log`. The root Rust freeze is released for
the next tested source delivery. The private GitHub main and alpha remain
unchanged while the critical picker/source and capture repairs are completed.

Runtime R4 `ec601edf` was cherry-picked as `ea145d93`, preserving the scanner
fallback and the narrow pre-dedup adoption hook. Its owner passed 87 host,
38 desktop and eight engine plugin cases, including all four real capture
roles and truthful native point failure/recovery. Independent round five is
reviewing the fixed repair. The N4 leaf `a5fdc398` was cherry-picked as
`7c7ed3ff`: atomic mapping/slots, callback adapter, control/helper/supervisor and
standalone audio-helper foundation. Its owner passed eleven units, eight real
process cases and sixteen preceding realtime cases. Desktop activation,
manager controls, production helper discovery and native editor parity remain
pending. Independent timing/ownership review and parent combined R4/bridge
checks are running at this fixed composition.

The R4 owner closed its shared fixture window. N4 may add distinct class-10+
and CLAP bridge fixtures after adopting the repaired point/drop API; existing
classes zero through nine remain intact. Production runtime/main/factory
wiring waits for acceptance. T1's new timeline/navigation Plan data is
separate from the utility owner's newly approved `Plan::keep_leaving` fix for
an unrelated-plan edit interrupting a live departure splice. No second owner
is rewriting the reserved Rack/State/native runtime seams.

## 2026-10-08 accepted import and stamp boundaries

Independent runtime R5 is clean at `ec601edf` / root `ea145d93` by
source tracing. Root fixed-source `7c7ed3ff` passed 106 host, eight engine
plugin and 39 desktop native cases, including all four R4 capture roles,
the default scanner fallback and the real CLAP archive/export round trip.
The command then stopped on a strict Clippy test-initializer warning in
bridge protocol tests. Root `7e0e3852` corrects that initializer without
changing production behavior; the modified test and remaining strict checks
passed on the next composition. The earlier command's exit 101 is retained
as a failed check, not reported as a complete pass.

Stamp `f39b90f8` is integrated as `cdd3b0c3`. Independent R4 found no
actionable defects, independently passed 174 focused tests and checked
actual delayed animation, scale changes and pre-IPC history cancellation
in its isolated browser. Root independently passed 171 tests across eight
stamp/history files, narrow lint and production TypeScript/Vite build.
Physical WebGPU/native Tauri focus remains unverified.

Sampler recovery `25ca8b03` and browser `8ff5a6bb` are integrated as
`7ed8153d` and `8660fbe2`. One fresh composed review covered browser R4
and sampler R3 together, closing the shared restored-source finding once,
along with stale picker/reply and Windows alias provenance findings. It
independently passed 19 bounded UI tests and inspected native assertions;
it did not rerun the owner's native tests. Parent composition `8660fbe2`
passed 84 focused native cases: 22 sampler preparation, 35 session library,
nine sample-cache, seven native library, six import-format/recording,
two recording and three clip-recording cases. An initial `import::` filter
matched zero; the actual `import_formats` and `import_recording` modules
were then run and all six passed. This correction is not a repeated pass.

Root also passed 268 browser/import UI tests across 14 files, narrow lint,
production TypeScript/Vite build, all-feature/all-target host Clippy,
engine/desktop all-target Clippy, workspace and fixture formatting,
WASM freshness and whitespace checks. The simulator remains 1,863,302
bytes and the model/schema has not changed in these repairs. These are
focused checks on top of the preceding 654-native/2,450-UI composition,
not another complete workspace run.

Independent N4 leaf R1 identified six source-level counterexamples in
the fixed `7c7ed3ff` composition: READY turnover resetting watchdog age,
lost note intent falsely acknowledged, offline success before supervisor
failure publication, old DSP proof relabelled across capture epochs,
concurrent capture cache order, and an unauthenticated first loopback
client. They are assigned to the same bridge owner for compiled
reproduction and repair. Desktop runtime/main/manager/factory activation
remains closed. The separate `27dc8a4a` followup supplies truthful native
37-frame delay fixtures and retry/cancellation hardening; it does not
constitute acceptance of these six findings or graph PDC integration.

Utility R3 `3073d220` and portable R3 `d9ac3da5` are delivered for new
independent reviews. Utility includes the approved progress/owner fix for
successive unrelated plans; portable includes actual bounded public ZIP
reader and exact source-identity regressions. They remain unaccepted until
review/composed checks. T1 reports actual navigation, bounded callback
transitions and buffered/stream/stem region parity; PDC/tempo/tails/native
WAV and shared-WASM UI coverage are still being completed before delivery.

GitHub was rechecked: the repository is private, alpha assets retain their
original names/sizes, and `artoomreinhart` still has the pending write
invitation. No release, tag, asset replacement or new invitation was made.

## 2026-10-08 portable and scaling acceptance; active feature deliveries

Portable R3 `d9ac3da5` is integrated as `6a6b72c4`. Independent R4 passed
81 attributable scoped native tests and found no actionable defects in the
exact original source classifier, concurrent publication or bounded ZIP
local/central validation. The same-ID/different-path history guarantee is
tested through legal Document contract fixtures; no production command that
creates that shape is claimed. Historical external sources still require
their original files.

Scaling repair `9418404a` is integrated as `4c8af7b2`. Independent R2 passed
117 tests and checked actual capped Canvas2D alignment and twelve viewport/
scale combinations. Root passed 208 composed UI tests, narrow lint and the
production TypeScript/Vite build with the stamp and picker repairs present.
These checks cover browser scrolling and logical canvas coordinates; native
third-party editors, physical WebGPU and other operating systems remain open.

Root fixed-source `015da36c` passed **311 unique native tests**: 152 DSP,
72 engine, seven project, 21 archive and 59 desktop cases. Three existing DSP
timing/demo tests remained ignored. The desktop cases include the actual
CLAP portable archive round trip and all four R4 capture roles. One new
project source test missed by the initial filter was subsequently run through
the attributable cached binary and passed. Logs are
`windfall-root-portable-utility-native.log` and
`windfall-root-portable-identity-completion.log` in the local temporary
directory. Strict all-target Clippy for DSP/project/archive/engine/desktop,
workspace formatting, independent comparison of 170 generated bindings,
WASM freshness, parity and whitespace checks passed.

The simulator was rebuilt from this source. Its binary remains byte-identical
at 1,863,302 bytes, SHA-256
`ae07e308eea6a7123310d243d60ec885cb09dc42b564f331786726a79342e53a`;
only the checked source-input hash changed. This is a focused composed check,
not a new complete workspace or complete UI run.

Utility `1e0fa525` and `3073d220` are integrated as `b6f6cd74` and
`015da36c`. Their preceding regression cases pass, but independent R4 found
a remaining same-ID restore during an unfinished departure splice. The
owner reproduced both limiter and hosted-effect tone discontinuities and
is repairing serial outgoing/active ownership, predecessor readiness and
generation-specific compensation. Parameters and automation must reach
only the active owner. This utility delivery remains unaccepted until that
incremental repair, a fresh review and composed checks pass.

T1 has source checkpoints `3ba8b13c` and `401939d4` for checked timeline
model/conversion and engine/session behavior. Its final UI checks and source
commit are pending. Selected-export guards use the canonical backend
generation and protocol revision; a frontend replacement epoch only rejects
late replies. Its 177 bindings and simulator are validation artifacts in the
owner worktree, not yet root artifacts or an accepted delivery.

E1's isolated DSP checkpoints `f0ad5e44` and `796a62f9` supply a resonant
lowpass, seven-mode filter and bass shelf. The owner passed 13 debug tests
and 14 release tests including headless CPU measurements, with zero guarded
callback allocations. Fixed-checkpoint independent review is running.
Registry, project persistence, automation, engine/export and UI integration
remain pending; all three parity rows remain incomplete. Shared registry
windows stay closed until the utility and timeline owners finish.

N4's six leaf findings remain under the same owner. Compiled reproductions
and repairs now cover offline mapped failure, capture epoch proof, capture
publication order, helper authentication and note-only overflow. The real
CLAP idle-owner hang was also reproduced during continued READY turnover.
The approved mapping ABI 2 owner-completion counter supplies watchdog
liveness only, never DSP acknowledgement. Private stdin authentication sends
no metadata to unrelated loopback clients. Final fixed-source checks and a
fresh independent review are still required before desktop activation.

No new parity row is marked complete. GitHub publication remains pending
these remaining repair gates; the existing private alpha is unchanged.

## 2026-10-08 integrated filter foundation and next repair boundaries

Utility restoration `7491dfb6` is integrated as `1e24389e`. Root native
execution 69297 passed **94 unique focused cases**: 65 engine effects, one
maximum-slot Plan bound, three Rack, three State, eight plugin, nine DSP repair
and five real R4 adoption/capture cases. Strict DSP/engine/desktop all-target
Clippy, workspace formatting and simulator freshness passed. The command's
tee covered only its final command; its named file is not a full transcript.
The executed command and results are retained in the T3 native tool activity.

Independent utility R5 passed 89 of those cases and confirmed the R1–R4
closures. It found one further P2: replacing a restored native owner by changing
only the existing factory revision can retain its progress identity while
losing its predecessor wait. The review's non-periodic cancellation residual
and excess outgoing-owner example are source-equation predictions, not new
compiled failures. The same owner is writing bounded regressions and a repair
in the approved Plan/Rack/State window. This remains an acceptance gate.

E1 pure DSP `f0ad5e44` and `796a62f9` are integrated as `7061a055` and
`9f365ef1`. Independent R1 source review and R2 found no actionable defects.
R2 independently passed 13 attributable release tests, leaving the throughput
test ignored, and sampled float32 filter poles without finding instability.
This sampled check is not an exhaustive mathematical stability proof. Root
passed 13 native tests with that same throughput test ignored, all-target DSP
Clippy and workspace formatting. Log: `windfall-root-e1-native.log`.

Root independently regenerated and compared all 170 bindings/fixtures and
passed 70 Document/transport UI tests against its rebuilt simulator. Logs are
`windfall-root-e1-bindings.log` and `windfall-root-e1-sim-ui.log`. The simulator
remains 1,863,302 bytes but is **not byte-identical** after this build; its new
SHA-256 is `7d2e0cb206eeeadb8cdc17fe669912bd08de1aee7fe3b59400aaead7b635a56e`.
Both the generated binary and source record are checked in. The three filter
rows are in progress; effect registry, persistence, automation, UI and actual
engine/export integration remain reserved until utility/T1 acceptance.

T1's complete first source range ends at `ed4ad871`, following `3ba8b13c`,
`401939d4` and `e9febc2c`. It remains outside root. Independent R1 passed
eight engine and three native timeline cases, reproduced two races through
the actual UI functions and source-traced a stopped-navigation automation
hold error. The owner is repairing guarded selection playback, ordered
pending arm/clear publication and destination automation hold. The approved
request watermark and guarded transport checks are transient session state,
preserve legacy callers and do not change musical history. Its 177 bindings
and larger simulator remain owner validation artifacts, not root artifacts.

N4 `ba61a255` follows the separate `27dc8a4a` leaf, both outside root.
Independent R2 source review supports the original watchdog, note-overflow,
offline-failure, capture-order and authentication trigger closures. It still
finds a post-reset incomplete-block proof epoch case, valid startup state
socket backpressure, and cancellation/deadline racing the final DONE state.
These three cases are assigned to the same owner for compiled regressions.
The additional native capture/flood/latency fixtures remain separate work.
A narrow sticky state-writer refusal guard was approved for the existing
LimitedWriter and CLAP save boundary; a plugin reporting success must not
make partial bytes acceptable after the host stream refused a write.
Desktop production routing remains closed pending acceptance.

M1 is active in the T3-bound `gpt/t3-analysis-jobs-m1` worktree at base
`1e24389e`. Its first owned stage is the native analysis crate: bounded worker
jobs, immutable inputs, explicit model provenance/checksums, owned artifacts,
cancellation and review tickets. Session/IPC/UI attachment is a later serialized
window. Fake inference verifies lifecycle only; it cannot close any ML row.
No additional parity completion or release publication is claimed.
