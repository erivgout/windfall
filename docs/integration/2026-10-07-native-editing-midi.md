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
