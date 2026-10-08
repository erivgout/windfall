# Timeline and region delivery (T1 first scope)

Base: `9b3001f47304167d2cced8dc09fcc8971ca7bf62`. This document is the
implementation contract and will record actual verification before handoff.
The work is owned by `gpt/t3-timeline-regions`; other owners' source and generated
artifacts remain reserved.

## Musical data and conversion

An optional, default-empty `Playlist.timeline` holds ordered song meter changes and
markers. IDs use the document's existing monotonic allocation. A meter change
has an ID, absolute song tick and supported `TimeSignature`. The scalar setting
remains the meter before the first change, including old v1 files. An explicit
change at zero overrides that adapter. Changing a meter never moves notes,
clips, automation, seconds or the tempo clock.

Conversion is checked integer arithmetic at 960 PPQ, independent of tempo.
Positions use one-based bar/beat and a zero-based tick within the beat. A change
starts a new bar. If the preceding bar is incomplete, it remains a shortened
bar; it is counted once. Exact boundaries belong to the new segment. Inverse
conversion rejects positions in the missing part of a shortened bar rather
than silently relocating them. Meter changes are strictly ordered by tick;
duplicate ticks, invalid signatures, duplicate/reused IDs and song-bound
overflow are refused atomically. There are at most 2,048 changes and markers
each. Shared Rust fixtures cover nonzero and unaligned 4/4 → 7/8 → 3/4 changes.

Named markers contain labels. Loop and skip markers contain an explicit
positive half-open tick range; pause markers contain a tick. Executable
marker intervals cannot overlap, and executable actions cannot collide at
one boundary. Names may share a tick. Commands allocate/add, update and remove
by stable identity, with one undo step, exact redo IDs and timeline patches.
Selection is transient and is pruned on deletion and cleared on replacement.

SMF song import/export uses the actual map. Unsupported meters retain explicit
adjustment reporting rather than claiming exact preservation. FLP song meter
and supported song markers use this data; per-pattern markers/signatures and
unsupported marker types remain reported follow-ups.

## Transport and rendering

Song playback can have an explicit checked half-open region `[start, end)`.
It is session state, separate from musical timeline data and from the UI's
dragged selection. An absent range retains existing whole-song behavior.
Pattern mode ignores song regions. Invalid endpoints, empty/reversed ranges,
overflow are refused before mutation. The accepted seek API still normalizes
nonfinite values to zero; region clamping then applies. The existing
loop-song switch loops the selected range; otherwise it stops at its end.
Seeking outside an active range clamps to that range. Seeking at its end
returns to its start when looping and remains stopped there otherwise.

Named markers never change playback. Skip jumps from its start to its end;
pause stops at its position and Resume advances past that same action once;
loop jumps from the loop range's end to its start while song looping is enabled.
A selected playback region takes precedence over loop markers. Navigation
boundaries use the tempo map's tick-to-frame mapping and the sequencer's existing
jump/clock-shift/clip-resync contract. Jumps end sequenced voices and audio clips
through existing fades and restore clips/automation at the destination. They
retain UI/hardware note ownership and plugin token/revision/binding/render role.
Immutable marker plans are compiled off locks; navigation has a maximum of 64
transitions per `Processor::process` call, including zero-frame chains. Exceeding
it stops playback and increments the controller's observable counter; the
playlist shows an error on the stopped transport event. The device's existing
scratch-buffer chunks retain that bound for each processor call. Navigation is
allocation/free/lock/wait/IO-free. No native API or DSP event format is added.

Export takes an explicit optional region in its immutable snapshot. Paired,
optional `regionGeneration` and `regionRevision` guards are checked before
snapshot preparation, under the existing recording-before-State order. A
partial pair or stale edit/New/Open pair refuses atomically. Both absent retains
the accepted legacy API. The selected-export UI takes both values from the
canonical backend state; its frontend replacement epoch only discards late
replies and is never sent as document generation. It renders
that linear interval once, with navigation markers disabled: a loop or pause
cannot create an unbounded/stalled export, and skip markers do not remove audio
from an explicitly selected export. Region endpoints use absolute sample-frame
boundaries through the tempo map (`ceil(end frame) - ceil(start frame)`). The
clock starts on the aligned sample while the event floor retains the requested
musical tick, so notes/markers exactly at a fractional-frame start are included.
Playback starts inside clipped sources at the
correct source position; sequenced notes crossing the region end release there.
PDC remains removed from the front, and fixed/automatic tails follow existing
automation hold and stream/stem duration rules. Buffered and streaming output,
stems and native WAV readback must agree. No region means legacy output. A
positive explicit region can export silence from an empty playlist; an empty
whole-song export still refuses. A cleared selected-export range produces an
inline error instead of falling back to the whole song.

Preparation, source identity, request/generation/edit checks, recording → State
ordering and native capture remain the accepted authorities. Timeline session
wiring must use those paths; it must not bypass sampler or portable-file guards.

## UI and ownership windows

Ruler labels follow the meter map, including shortened bars, through shared
fixtures. Controls expose meter changes, marker CRUD, timeline range selection,
play once/loop/clear and export selection. Actions are discoverable in the action
registry and context menus. Field/Alert and accessible titles convey validation
and desktop capabilities. A dragged region zoom fits its endpoints through the
existing logical viewport; CSS scale 75–200%, DPR and scroll are normalized once.
Escape, pointer cancellation, focus loss and project replacement abandon a drag.
Accepted split/slip, audio inspector and piano/stamp guards remain intact.

The parent granted these timeline-only central seams before integration:

- Engine `plan.rs`: immutable bounded navigation records; `processor.rs`:
  sample-boundary subdivision and existing jump/resync/stop integration;
  `controller.rs`, `message.rs`, `shared.rs`: Copy region transport payloads.
- Engine `lib.rs`: only module declarations/exports; `sequencer.rs`, `render.rs`,
  `stems.rs`: region and navigation integration. `tempo.rs` only if a checked
  frame conversion helper is needed.
- IPC `lib.rs`: optional region transport/export fields. Desktop
  `session/mod.rs`, `transport.rs`, `export.rs`, command registration: narrow
  new `session/timeline.rs` wiring using existing guards.
- Project `model.rs`, `command.rs`, `check.rs`, `lower.rs`, `edit.rs`, `patch.rs`
  and module export: additive timeline-only changes. Avoid `document.rs`:
  derive patches through owned playlist data if necessary rather than touching
  portable exact-source helpers. MIDI/FLP conversion: timeline mapping only.
- UI Backend/Tauri/mock/simulator and patch wiring: additive timeline/region
  plumbing; preserve browser/sample helpers and project history-intent signal.

No edit to reserved engine `rack.rs`, `state.rs`, `plugins.rs`, native plugin
runtime/factory routing, sampler edit/sample/preparation files, browser library,
portable session files or piano editor/grid/stamp files is planned. If existing
hooks cannot satisfy the callback contract, propose the exact additional hook
and wait for the parent's integration window while continuing owned work.
`Plan::keep_leaving`, departing definitions and timing remain utility-owned and
unchanged. Tauri registration is additive in `commands.rs`; desktop main/lib and
the N4 helper entry remain untouched.

## Retained follow-ups and verification limits

Per-pattern signatures/markers (`win-piano-time-markers`), alternative
arrangements, linked tracks, grouping/make-unique (`win-playlist-arrangements`)
and scrubbing remain separate T1 delivery requirements. WAV marker metadata
(`fmt-export-wav-markers`) is distinct from rendering a region. This delivery
does not close those rows. Generated bindings/WASM may be rebuilt locally only
for verification and are excluded from source commits; the parent regenerates.

Focused checks must cover checked commands/history/legacy v1 save, processor
marker behavior, tempo ramps, clipped sources/offsets/all clip kinds, unaligned
boundaries, tails/PDC, callback allocator counts, native WAV content/duration,
stream parity, selection/replacement/cancellation and accessible ruler controls.
Windows headless fixtures do not establish physical native UI/audio, installed
external plugin behavior or non-Windows execution.

## Implemented source and focused proof

Project commands/conversion live in `crates/windfall-project/src/timeline.rs`;
SMF import/export and selected FLP song conversion preserve the actual map.
The engine's new `timeline.rs` compiles immutable navigation and sample bounds;
processor/sequencer integrate existing note-release, clip-chase, automation and
garbage retirement. Native `session/timeline.rs` publishes checked transient
ranges; export options feed the existing native snapshot/encoder transaction.
No new plugin or DSP events/state capture APIs were introduced.

The playlist has Seek, Select time and Zoom region tools; its Timeline menu,
context menu and command palette expose CRUD, play/loop/clear, fit and export.
Ruler and song-position readout share meter labels. Pattern labels, the legacy
snap/bar grid adapter and per-pattern signatures remain scalar. The common
canvas module is unchanged; a variable-meter clip snap/grid is a retained
refinement rather than a change to absolute ticks or clip lengths.

Completed focused behavior checks:

- Project timeline integration: five tests, including checked atomic refusal,
  monotonic IDs, one-undo/redo/save, legacy v1 3/4 and shared meter fixtures.
- MIDI: two new real SMF map tests and 27 import compatibility tests. FLP: one
  actual written/imported FLP map/marker test with unsupported distinctions.
- Engine timeline: eight tests. They exercise real loop/skip/pause/resume/seek,
  fractional marker/note start samples, selected-end precedence, bounded tiny
  loops with zero callback allocator/deallocator calls, source offsets, all clip
  kinds, tempo ramps, 480-frame PDC, fixed/automatic tails, stream/stem bit parity
  and cancellation/refusal.
- Engine compatibility: 20 sequencing, 22 rendering and 11 stem tests passed.
- Native session: three tests passed, including canonical matching/stale/partial
  source guards, edit/New/Open/recording refusal, float WAV readback equal to the
  renderer with exact body/tail duration, cancellation after replacement,
  preserved destination and removed staging files. Seventeen existing native
  export/format/stem/cancellation tests passed unchanged.
- Shared-WASM UI: Rust fixtures are asserted against the actual WASM conversion
  and UI ruler labels; CRUD uses the real Rust Document and undo. Actual ruler
  pointer events prove Escape/focus/scale/replacement cancellation; region zoom
  covers 75–200% scale, DPR 1/1.5/2 and scroll. Selected-export form/backend tests
  prove copied ranges, canonical pairs, stale edit/New/Open refusal and absent
  guard compatibility. Software browser checks also verified accessible meter
  editing and selection-fit actions at 100/125/200%; physical UI/audio is not
  inferred from those checks.

Final validation uses 196 passing tests in 11 focused UI files, including the
existing slicer, playlist editing lifetime, transport and replacement suites.
The selected-export form deliberately receives a backend generation different
from the frontend replacement epoch and forwards that canonical pair. Legacy
whole-song calls query neither source guard and send neither field. Escape
during a ruler drag preserves the existing clip selection.

`cargo fmt --all -- --check` and strict all-target package Clippy (`-D warnings`)
pass for project, MIDI, FLP, IPC, simulator, engine and desktop. Desktop
TypeScript and ESLint checks pass. Local regeneration produced 177 bindings;
the rebuilt simulator passed `scripts/check-sim.mjs` before the shared-WASM
checks. Bindings, simulator binaries and build metadata are validation-only
and are excluded from the delivery commits. Parent integration must regenerate
its own artifacts from the combined accepted source.

## R1: lifetime races and stopped navigation tails

The immutable first delivery ends at
`ed4ad8713ffb6157ada552882e75eb5919722e5d`. R1 requested three P2 fixes:

1. Play selection guarded region publication but then awaited unguarded
   transport set, seek and play. A New/Open replacement during those awaits
   could receive the old cursor and start playing. The action now captures
   its entry lifetime/request and checks every continuation. Set/seek/play
   also carry the publication's canonical generation, protocol revision,
   request and region to native authority; an old request cannot mutate the
   replacement even when its native execution was delayed, not just its reply.
2. Clearing a pending first arm did nothing while UI `active` was false.
   Selection changes now invalidate that action and reconcile pending or
   possibly committed publications. Newer clear/arm requests win in native
   transport; a late old reply never schedules a compensating clear that
   could overwrite a newer arm. A committed arm whose reply was discarded
   after an edit remains eligible for guarded clear/reconciliation.
3. A skip `[90,200)` inside non-looping selection `[10,100)` clamps its
   destination to 100 and stops. The jump cleared automation hold while
   effect tails still sounded. It now holds that stopped destination through
   the existing automation/tail policy. Healthy playing jumps still clear
   hold, and explicit seek/resume retains the accepted behavior.

The request watermark is transient Session `Inner` storage, initialized once
and retained across document replacement. **Every load/store occurs while the
existing State mutex is held.** Guarded publications and transport mutations
take recording-idle before State, compare generation/revision/current request/
current region under that same State guard, then mutate the controller before
releasing it. The atomic storage adds no callback work; it avoids changing the
portable-owned `files.rs` State reconstruction. No project field, history step,
dirty state, controller, Plan, Rack, engine State, departure or adoption change
is involved. Legacy unguarded transport and source-guard-free export calls keep
their accepted paths.

Requests are positive integers through `2^53 - 1`, exactly representable in
TypeScript and native JSON. Allocation rebases from the canonical watermark
and retains higher requests already sent by this UI. It refuses exhaustion
before sending a command, and native refuses old, reused or out-of-range
numbers before mutation; neither side wraps. A window/module reload observes
the retained native watermark rather than restarting its sequence at zero.
Exhaustion remains observable across reload and requires restarting the app.

Executed RED/GREEN proof:

- Against original production functions with only the new tests present and
  freshly generated local shared WASM, 15 of 17 UI cases failed. Ten failures
  reproduced New/Open races with set/seek/play deferred before native commit
  or after commit before reply; four reproduced pending first-arm clear/
  replacement, and one reproduced reordered old arm/clear versus a newer arm.
  The two post-play-reply cases already preserved replacement state.
- The compiled engine regression failed with the exact requested skip and
  selection. With 480-frame limiter PDC, its first differing interleaved sample
  was 4962: the held fader diverged during the audible reverb tail. After the
  one-line destination-hold fix, output matches the constant-fader reference
  sample for sample, including PDC and fixed/automatic linear render references;
  callback allocator/deallocator calls are zero. Stopped seek releases hold and
  resumed playback restores automation.
- GREEN UI lifetime coverage includes every awaited source/publication/set/
  seek/play boundary, New/Open, clear/replacement/rearm, delayed native commit
  and delayed replies, edited committed publication cleanup, and subsequent
  valid arms. Actual module reloads at native requests `2^53 - 3` and
  `2^53 - 2` allocate the final two exact numbers and refuse exhaustion without
  project/history/revision/dirty changes or wrap reuse.
- Native session tests execute matching guards, wrong current region/request,
  stale edit/New/Open, recording refusal, out-of-order arm/clear and a fresh
  valid arm with actual processor output. They also execute the last safe
  increment and atomic refusal above the limit, while retaining legacy calls,
  native WAV/PDC/tail readback and destination/staging cancellation checks.

Final R1 checks passed: 234 shared-WASM UI tests in 14 focused files (including
28 lifetime races and the module-reload boundary test); nine engine timeline,
25 engine automation, five native timeline, 14 native playback and 17 native
export/format/stem tests. Strict all-target Clippy for the seven delivery
packages, workspace fmt, desktop TypeScript/ESLint and changed-UI Prettier pass.
Own-worktree regeneration again produced 177 bindings and a current simulator
verified by `check-sim`; these local validation artifacts are excluded/restored
before the source-only commit.

R1 stays atop the isolated fixed source; no parent commits were imported or
original commits amended. Parent E1 DSP and utility revision work remains
independent and was not treated as an accepted prerequisite or edited here.
These checks establish this source's behavior, not combined-root verification.
All per-pattern, arrangements/grouping/linked-track/scrub, WAV marker metadata
and scalar clip snap/grid follow-ups listed above remain open.

## R2: native cancellation ownership and retained range recovery

R2 reviewed immutable `e0809d0e1933e650667f379dbaf4687196d30865` and requested
three P2 changes. This repair is incremental on that exact source, without
parent imports or amendments:

1. Clear invalidated UI continuations but awaited a canonical source query
   before publishing its newer request. A pending guarded Play could commit
   during that wait, then continue playing after the region was cleared.
   Pending selection actions now carry an exact cancellation target through
   newer intents. Native reconciles a still-owned guarded Play in the same
   recording-before-State decision that changes the region and watermark.
2. Refused or edit-cancelled Clear erased selection and `active` before native
   acceptance. The UI now retains the canonical active range through a pending
   replacement. A failed or edited continuation queries canonical state again,
   restores the retained range and shows an inline retry error. A later newer
   intent or project replacement suppresses old recovery and error replies.
3. A new frontend had no knowledge of the native retained region, so Clear
   was disabled or skipped publication. Playlist startup now hydrates canonical
   region state after the document mirror is ready. Until it is known, Clear
   stays available. Actual Clear and selection actions always query canonical
   state; they do not depend on an optimistic `mayBeArmed` flag. A retained
   range is shown and can be cleared after reload, including after selecting
   another range. Delayed hydration cannot overwrite a newer request/intent.

### One native decision, exact pending action ownership

Session `Inner.timeline_play_request` is transient and initialized once.
Every read/store is under the existing State mutex, like the publication
watermark. Successful guarded Play records its exact positive JS-safe request.
The optional cancellation guard carries generation, revision, request and
region. Region publication first validates its own current canonical source,
checked bounds and strictly newer safe watermark under recording-idle then
State. It stops playback only if the cancellation guard still matches the
canonical generation/revision/request/region **and** the successful guarded
Play owner. If playback already ended, it relinquishes ownership without a
second Stop, preserving the accepted automation/tail hold. All checks precede
mutation; a recording, source, range or numeric refusal retains the owner,
transport, region and watermark. The transport event follows the completed
region/watermark decision.

The R2 UI recorded the target before publishing its region; R3 below corrects
that timing for chained successors. It keeps `playPending` until a fresh Play
reply completes the action. Cancelling an action whose Play committed during
the source-query window stops that exact still-owned playback; if Clear wins
first, the old Play guard refuses. A later valid Play owns a different request,
so a stale cancellation cannot stop it. A settled selection Clear carries no
cancellation target and preserves the existing ordinary playback semantics.
On edit retry, the target keeps the original generation/request/range and uses
the newly queried canonical revision. Native still checks the complete guard
against its current State; no frontend cached epoch is a native authority.

The approved four legacy transport hooks only relinquish ownership immediately
before successful Play/Stop/Seek/Set mutation. Invalid pattern/empty-song Play
and recording-refused Set/Seek retain ownership. Stop and Seek serialize with
the existing State mutex; Seek keeps recording-before-State order, and Stop
retains its existing availability during recording. Toggle delegates to these
helpers. Caller inspection found the production command wrappers and Toggle;
recording finalization/take attachment does not call these helpers under State.
No controller, Plan, Rack, engine State, utility departure/adoption, callback,
plugin, source, history or portable-file change is involved.

UI `playback` holds the canonical region snapshot; `active` is its derived
display state. View-only selection is still transient and may exist without a
native region. Intent identity and entry frontend lifetime/revision reject late
continuations, while native guards remain the mutation authority. Recovery
does not send late compensating mutations, so it cannot clear a newer arm.
Canonical request rebasing, exact-number checks and exhaustion without wrap
remain unchanged, including actual module reloads near `2^53 - 1`.

### Executed RED/GREEN

Before production changes, the six new actual-function UI cases failed using
fresh local shared WASM: the Play-during-Clear-query case ended playing with no
region; recording refusal and both edit query/commit races hid `[17,839)`; both
reloaded user-action paths retained the region after direct Clear or select-
then-Clear. The native compiled regression held the query off State, committed
guarded Play, confirmed audible processor output and then cleared the region;
its stopped-playback assertion failed against the original function.

GREEN covers the original R1 source/publication/set/seek/play New/Open races,
out-of-order first arm/clear/newer arm, numeric exhaustion and module reload,
plus R2 cancellation before/after native Play commit, committed-before-reply
Play, each ordinary Play/Seek/Set/Stop supersession, settled selection Clear,
later valid Play, recording/refusal retry, edit-query/commit recovery, delayed
hydration and the accessible startup menu showing the retained range. The
legacy export test observes the export action after selection's necessary
canonical query, so it still proves absent export source guards/query. Meter
CRUD also opens the menu with its accessible ArrowDown keyboard path after
closing the editor, without a timing sleep or ambiguous toggle.

Native tests prove audible cancellation, later valid playback, stale arm/clear
refusal, all four ordinary supersessions, invalid legacy mutations preserving
ownership, recording-before-State refusal, fresh retry after an edit and natural
selected-end stop followed by a refused empty-song Play. Project/history/
revision/dirty data stays unchanged by timeline decisions. The original R1
stopped-skip test still proves held automation through audible fixed/automatic
effect tails and 480-frame PDC with bit parity and zero callback allocation/free.
Native WAV readback and cancellation destination/staging cleanup still pass.

Final R2 checks passed: 248 shared-WASM UI tests in 15 focused files, including
39 lifetime cases, three actual module-reload cases and the retained-range
startup menu; ten native timeline, 14 native playback, two native recording/
take, 17 native export/format/stem, nine engine timeline and 25 engine automation
tests (77 Rust/native tests). Strict all-target Clippy for project, MIDI, FLP,
IPC, simulator, engine and desktop, workspace fmt, desktop TypeScript/ESLint,
changed TypeScript Prettier and source diff checks pass. Local generation
produced 177 bindings and a 1,964,361-byte shared simulator, verified current by
`check-sim` before UI validation. These artifacts are restored/excluded before
the source-only commit; parent remains the combined-artifact owner.

Parent E1 DSP and utility R5/R6 work remains separate; no root code or artifacts
were imported. Verification here applies to this isolated source, not combined
root, installed external plugins, physical UI/audio or non-Windows behavior.
The full arrangements, per-pattern timelines, WAV metadata and scalar clip
snap/grid refinements remain explicit follow-ups.

## Review R3 repair (parent source `59c8f7f3`)

This incremental repair preserves immutable parent
`59c8f7f3adc0c552fca972b515e5219ddaf24d06` and all earlier source commits.
It addresses the two P2 behavior findings and three documented-standard
findings; it does not close the retained full-T1 follow-ups.

### Bounded cancellation authority through successors

Play A can remain awaiting its native Play commit while B, then C, prepare
newer region publications. Their unpublished guards cannot authorize
transport yet. R2 incorrectly preferred one of those guards over the inherited
live A target, so Clear could remove the region while A continued playing.

An operation now acquires its own cancellation guard only after receiving an
accepted region reply while its entry lifetime/revision/intent is still current.
Until then, it carries its predecessor's exact target. This is one inherited
target, not a growing lineage. When a successor publication commits, the same
existing native recording-before-State decision cancels an owned predecessor
or makes its delayed Play stale. Only an accepted current successor reply can
continue into guarded Set/Seek/Play. A cancelled committed publication cannot
become another Play target. No new native API, watermark or owner field is needed.

Clear/refusal retries retain that exact target with the fresh canonical revision;
New/Open resets UI lifetime metadata and canonical generation guards reject old
requests. Ordinary successful transport still relinquishes ownership, so Clear
does not stop unrelated playback. Natural region end relinquishes a matched
owner without a second Stop, preserving endpoint automation/tails. Counter
rebasing, exact JS-safe numbers, exhaustion without reuse, canonical source
authority and atomic refusal are unchanged.

### Mode-correct hosted transport

Processor's meter lookup now uses the song map only in song mode. Pattern mode
uses the scalar project signature, matching the retained pattern-label contract.
The new test installs actual engine `PluginFactory` probes for CLAP/VST3 saved
bindings, both instruments and effects. It observes every `PluginTransport`
field through the accepted hosted facades at song tick zero (7/8), pattern tick
zero and tick 2001 (4/4), then song tick 2001 (3/4) and back to zero. It switches
the selected pattern source too. Probe writes are preallocated atomic stores;
every processing assertion also requires zero callback allocation/free.

Desktop runtime forwarding and CLAP/VST3 adapter signature assignments were
source-traced unchanged. This test executes engine hosted facades, not installed
external plugins or OS native fixture binaries; no N4/native fixture edits or
new capture/format/state APIs are involved.

### Registry controls and numeric identity types

Playlist registry invalidation includes `hydrated`. An already-open registry
consumer now updates when hydration returns no range, as well as when it
restores an existing range. Clear explains that no selection remains. The five
static Play/Loop/Zoom/Export/Clear dropdown entries render `ActionMenuItem` from
their registered action IDs, including registry titles, shortcut labels,
contextual enablement and disabled reasons. Entity-specific CRUD remains local.
The separately approved caller cleanup removes only the unused controls metrics
prop; playlist viewport/layout/ruler behavior is unchanged.

`MeterChangeId` and `TimelineMarkerId` are separate serde-transparent u32
newtypes, with the same conversions and numeric TypeScript representation as
existing project IDs. Timeline command IDs are typed; allocation and global
collision checks explicitly compare their underlying u32 values. Numeric JSON,
v1 format version, legacy defaults, created-ID reply order, playlist patches,
next-ID non-reuse and undo/redo stay unchanged. MIDI/FLP conversions already
dispatch timeline commands and obtain IDs from the typed project, so they need
no vendor-ID casts or wire changes. Project native and actual shared-WASM tests
cover numeric add/update/remove, wrong-family/invalid-number atomic refusal,
allocator identities, save/open and undo/redo.

### Executed RED/GREEN and limits

Before either P2 production fix, compiled/executed actual-function tests with
fresh local shared WASM failed for A→B and A→B→C, leaving `playing: true` after
Clear. The native engine hosted transport test failed at pattern tick zero:
received `(7,8)`, expected `(4,4)`. The no-range hydration registry test also
failed because its notification version did not change. These were behavior
assertions, not enum-presence or synthetic source predictions.

GREEN executes deferred publication and reply reordering, Clear/rearm, edited
source, New/Open, refusal/retry and each ordinary Play/Seek/Set/Stop supersession.
Native audio proves the inherited A guard stops only its still-owned Play,
refuses delayed B/C requests, preserves ordinary and naturally stopped transport,
does not mutate project/history/revision/dirty state, and allows a later audible
valid Play. The full existing R1/R2 timeline guard, replacement, reload, safe-number,
recording, actual WAV/PDC/tail and destination/staging cleanup cases are retained.

All work uses the existing isolated `target/timeline-verification` cache,
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, isolated TS export directories and
at most two UI workers. Validation-only generation produced 179 bindings and a
1,964,581-byte simulator. `check-sim` verified the simulator current after source
formatting; generated bindings/WASM are restored/excluded before commit.
Final executed checks passed: **251 shared-WASM UI tests in 14 focused files**,
including 50 lifetime cases, three actual module-reload cases, four hydration/
registry menu cases and eight timeline/identity/zoom cases. Project timeline
seven, MIDI map two, FLP actual-map one, engine timeline ten and native session
timeline eleven cases passed (**31 distinct native/project/engine cases**).
The chained native case was rerun after adding natural-end hold/position
assertions. No full native compatibility cluster was repeated just for counts.
Strict all-target Clippy (`-D warnings`) passed for project, MIDI, FLP, IPC,
simulator, engine and desktop; workspace fmt, desktop TypeScript, ESLint,
changed-TypeScript Prettier and source diff checks passed.

Commands used the isolated cache and export environment above:

```text
cargo test -p windfall-project --test timeline -- --test-threads=1
cargo test -p windfall-midi --test timeline -- --test-threads=1
cargo test -p windfall-flp --test conversion timeline_ -- --test-threads=1
cargo test -p windfall-engine --test engine timeline:: -- --test-threads=1
cargo test -p windfall-desktop --lib session::tests::timeline:: -- --test-threads=1
cargo test -p windfall-desktop --lib timeline_chained_unpublished -- --test-threads=1
cargo clippy -p windfall-project -p windfall-midi -p windfall-flp -p windfall-ipc -p windfall-sim -p windfall-engine -p windfall-desktop --all-targets -- -D warnings
cargo fmt --all -- --check
pnpm --dir apps/desktop exec tsc -b
pnpm --dir apps/desktop exec eslint .
pnpm --dir apps/desktop exec vitest run [14 focused files] --maxWorkers=2
```

The 14-file UI run at 07:09:25 UTC and final four-case menu refinement run at
07:15:26 UTC used current WASM SHA-256
`e4161abd2abe188a7d43d5d7d4429d242545b13f430f7423c05ff4b4bb1c0abc`.
Its verified input fingerprint after formatting was
`0da6ea9d89ec046006eb8865f6fce439fd923cc298c6f588886eefdf6aed983d`.
The final menu refinement checks the correct disabled reason outside playlist
context and awaits the actual menu portal before inspecting its registry items.
An intervening check after premature artifact restoration hit legacy artifacts;
it was discarded, the matching local artifacts were reapplied/verified, and
the final menu/TypeScript checks passed before restoring artifacts again.
Private executable provenance under `target/timeline-verification/debug/deps`
(2026-10-08 UTC; not committed/generated product artifacts):

| Binary | Written UTC | SHA-256 |
| --- | --- | --- |
| `engine-d477693dd860322b.exe` | 07:07:27 | `aae0b84c6a15dcb71a5341b6574eeac4293bf3989bb1e9c10ef86b9eee9596b0` |
| `windfall_desktop_lib-a486fa5ea3ee0578.exe` | 07:09:51 | `0da7168ad85d21f33d67c9163ddc694f0a33009f50203690c58f25aa1a1cdc3a` |
| `timeline-8c4ccc693213df2b.exe` | 07:10:03 | `3a628989044b01ff016b716bf0725d57a7f007a48652a0e76599242d8070a8cc` |

Corresponding Cargo `.fingerprint/windfall-{package}-{binary suffix}/`
`test-integration-test-engine.json`, `test-lib-windfall_desktop_lib.json` and
`test-integration-test-timeline.json` match those build times; `.d` dependency
paths identify this bound worktree. Independent inventory of these executables
lists ten engine and eleven session timeline tests. Final engine timeline source
SHA-256 is `872973f89e3aa9711d1ec4c8105e751bd08b3848c8a36f23c08deb69ff184065`;
session timeline source is
`8e8847d61a0632a06227d4bb745297ec45819c0baafe40d7535a4f3e298ebeab`.
Combined-root integration remains the parent's gate.

No root source/artifacts or other owners' changes were imported. Controller,
Plan, Rack, engine State, utility departure/adoption, native runtime, sampler,
browser, portable files, piano/history-intent and DSP seams remain untouched.
Physical UI/audio, installed external plugins and non-Windows behavior remain
unverified. Arrangements, per-pattern timelines, linked/group/make-unique/scrub,
WAV marker metadata and scalar snap/grid refinements remain open.

## R4: static creation actions and native bar origins

R4 reviewed immutable `a8e33f209c3d5d85e3ecd55b876d02bdd5e22119` and
requested both inherited findings be repaired:

1. The five static Add entries still duplicated registered actions with local
   labels/handlers. All five now use `ActionMenuItem` and their existing
   registry IDs. The actual entity editor/CRUD controls stay local. Add actions
   expose the registry's contextual disabled reason. No registry primitive,
   panel layout or central registration changed.
2. Song meter changes start new bars, but hosted adapters calculated bars as
   though the current signature applied from tick zero. In the shared fixture,
   4/4 to 7/8 at tick 4001 starts zero-based bar 2 at `4001 / 960` beats;
   the old CLAP/VST3 ABI builders reported a downbeat at 3.5 beats instead.

The separately consumable metadata prerequisite is source-only commit
`94e168ae6d056bae8b17ef95dfcc8a92b0b1699a`, directly above `a8e33f20`.
It contains exactly seven paths: engine `plugins.rs` adds the Rust-only
`MeterAnchor { bar_origin_beats: f64, bar_origin_index: u32 }` and optional
transport field; engine `processor.rs` initially supplies mechanical `None`;
host `events.rs` supplies the same type/default and checked `bar_position()`;
host `lib.rs` re-exports the type; CLAP/VST3 processor modules use the checked
native bar fields and contain private ABI regressions; host
`tests/processing.rs` adds only `meter_anchor: None` to its explicit literal.
The realtime/VST3/example literals already use `..Default` and were untouched.
No project/Plan/UI or N4 runtime/bridge/render-error hunk is in that prerequisite.

The producer uses checked Rust-only immutable `MeterSegment` records, prepared
off State/audio locks. Their absolute start ticks and cumulative zero-based bar
indices come from the same shortened-bar conversion as the ruler. A bounded
binary lookup selects the segment at the first frame. Song transports carry
its anchor; Pattern transports carry `None` and the scalar signature. Native
bar derivation is `origin + floor((position - origin) / width) * width`, with
the cumulative index added to that relative bar. The host helper refuses
nonfinite/negative anchors, positions preceding their anchor and CLAP index
overflow. Adapters return the existing `ProcessFailed` before starting native
processing on failure. Absolute beats/seconds/tempo/playing/signature are
preserved; host `advance` changes the absolute clock without changing the anchor.
Neither persisted numeric JSON, shared-WASM types nor IPC changed.

Malformed raw meter data is also explicit failure. `MeterMap::checked` returns
a Copy `MeterMapError`; the existing `MeterMap::new` string adapter preserves
document error messages. Infallible Plan compilation retains
`Result<Vec<MeterSegment>, MeterMapError>` for unpublished introspection.
`Controller::try_prepare_project` checks before sampler preparation/snapshots.
The approved control-side publication gates at `set_prepared_project` and
`set_plan` latch existing
`Unsupported("Invalid song meter map; project preparation refused.")` before
State/factory/selected-pattern mutation and retain the installed plan.
A healthy next installation clears the error. No panic, empty-map fallback,
invalid anchor sentinel, callback error string or new general error API is used.
The defensive callback invariant branch fills silence if an internal failed
plan ever reaches it; public publication gates prevent that path.

Executed RED/GREEN evidence:

- Seven new actual-function/shared-WASM Add menu cases failed RED on the absent
  registry titles. GREEN verifies configured shortcuts, direct registry
  invocation, disabled reasons and real Rust document creation for all five
  kinds, plus New/Open cancellation and fresh-source re-entry. The prior
  timeline, hydration, chained lifetime and safe-request/reload cases pass.
- Two compiled native ABI tests failed RED with the wrong 3.5-beat downbeat;
  the two scalar Pattern controls already passed. Seven host meter tests now
  pass, including fractional/exact bar boundaries, all unchanged transport
  fields, invalid-anchor refusal, scalar legacy bars, index bounds and advance.
  These execute the concrete CLAP `TransportEvent` and VST3 `ProcessContext`
  builders used by processing, not an installed external plugin or bridge.
- The expanded engine hosted-factory probe failed RED on missing Song anchors.
  GREEN observes hosted instrument/effect roles for both saved format
  identifiers through actual engine processing. It proves tick-4001 origin/
  index, aligned/later changes, fractional seeks, selected-start clamping,
  Song/Pattern/source switches and zero callback allocator/deallocator calls.
  Its clock/tempo fields are bit-identical to a scalar-map control under the
  same tempo ramp and frame timing. The new fixture was corrected to respect
  existing tempo look-ahead, ceil-aligned first-sample positions and Pattern
  source bounds; the original six-field assertions were preserved.
- A compiled private Controller regression exercises raw invalid scalar
  signatures and colliding meter changes through fallible preparation,
  infallible compile, `set_prepared_project`, direct `set_plan` and convenience
  `set_project`. The installed Plan Arc, playing transport, selected pattern,
  cursor/range, native provider/preparation/process counts and owner remain
  unchanged. Audio continues at 0.25 per channel with zero callback allocator/
  deallocator calls; a healthy next install clears the error and reuses the
  owner. Nine project timeline cases also preserve v1 persistence, numeric ID
  validation/history and typed/string error compatibility.

Final focused execution: **27 distinct native/project/engine tests** (seven
host meter, ten engine timeline, one Controller refusal and nine project
timeline); **93 shared-WASM UI tests in eight files**. The engine timeline run
retains R1 stopped-skip automation/PDC/tails, all clip sources/offsets, tempo
ramps, fixed/automatic tails, linear buffer/stream/stem parity and the 64-transition
callback bound. UI coverage includes all 50 prior ordered lifetime cases,
reload/exhaustion/hydration, selected export guards, the position readout and
selection zoom. No prior assertions were removed or skipped.

Strict all-target Clippy (`-D warnings`) passed for the three changed Rust
packages (project, engine and plugin host), as did workspace fmt, desktop
TypeScript, ESLint and changed-TypeScript Prettier. Commands used the existing
isolated target, jobs/tests 1 and at most two UI workers:

```text
cargo test -p windfall-plugin-host --lib meter_tests -- --test-threads=1
cargo test -p windfall-engine --lib timeline_invalid_meter_snapshots -- --test-threads=1
cargo test -p windfall-engine --test engine timeline:: -- --test-threads=1
cargo test -p windfall-project --test timeline -- --test-threads=1
cargo clippy -p windfall-project -p windfall-engine -p windfall-plugin-host --all-targets -- -D warnings
cargo fmt --all -- --check
scripts/gen-bindings.sh "$PWD/target/timeline-ui-bindings"
scripts/build-sim.sh
node scripts/check-bindings.mjs target/timeline-ui-bindings
node scripts/check-sim.mjs
pnpm --dir apps/desktop exec vitest run [eight focused files] --maxWorkers=2
pnpm --dir apps/desktop exec tsc -b
pnpm --dir apps/desktop exec eslint .
```

The final UI run started at 08:16:04 UTC with 179 freshly regenerated bindings
and a 1,965,327-byte simulator. WASM SHA-256:
`2a09632df8c4d2e4588ad45f27b2d7b6208d0db7802ac127d64aed6d4665c7bb`;
verified input fingerprint:
`0d72dc6ccb1e953f083bab667b6a9dfe7469301e811642c19d4ca318da158a6b`.
Validation artifacts are restored/excluded before commit, and no UI check is
run against the subsequently restored legacy artifacts.

Private executable provenance under `target/timeline-verification/debug/deps`
(2026-10-08 UTC, validation only):

| Binary | Written UTC | SHA-256 |
| --- | --- | --- |
| `windfall_plugin_host-a142c4b240164585.exe` | 07:50:52 | `b99cc4c40b9543444f4ddeb8c0af94ccf1c445f6fad2bd12d20c08c8d418ed9f` |
| `windfall_engine-31efa27a98778ef9.exe` | 08:08:21 | `10111dd3ae310261c26f731241c58a9171c7fae1bc3ca9caa1a2a3a80a6b2239` |
| `engine-d477693dd860322b.exe` | 08:11:23 | `0dfe009d15ce6f85796081b6d60d68f5304346cb7fa27152eab28f53e80732f9` |
| `timeline-8c4ccc693213df2b.exe` | 08:13:34 | `93ff0f8387e8510fbfa7d60b949b94521e8c02e27cc7f3921abc8cbf97d3409d` |

Their corresponding `.fingerprint/windfall-{package}-{suffix}/test-*.json`
records and `.d` paths identify this bound checkout; inventories list seven
host meter, one Controller refusal, ten engine timeline and nine project
timeline tests. Final engine timeline source SHA-256:
`5aaf1780686c53a85b7ca4baf6f633b91a44c909241c39721ccc19a9990a576a`;
Controller source:
`e7647f53595ae90ad11fea418fc016f136b6c24c9ecee5aa23a779bdaf60cd3b`;
project timeline source:
`ad1e8655cd142584f668d68331c6b6442366f7d5700c23f9f6a11d2decbebbb2`.
Local RED/GREEN/strict-check logs are `target/timeline-r4-*.log`.

N4 exclusively owns runtime anchor forwarding, its literal compatibility,
versioned bridge ABI3 layout/codec/helper/adapter, and checked render/stem error
reporting. Its precompile/finalization meter refusal must use the same public
checked map and existing Unsupported classification before the void publication
gate. T1 has not edited or imported those paths. Consequently combined bridge,
desktop native tests/Clippy and malformed-map offline render refusal remain
composition gates, not claimed GREEN here. No unversioned ABI2 extension was
made. The parent regenerates combined artifacts after accepted integration.
M1's Session/commands/IPC/backend registration window remains untouched, as do
Rack/engine State, utility departures/adoption, sampler/browser/portable/history
and native fixture classes. Physical audio/UI, installed external plugins and
non-Windows behavior remain unverified. Full arrangements, per-pattern
timelines, links/groups/make-unique/scrub, WAV marker metadata and scalar snap/
grid refinements remain open.

## R5: exact native downbeats after independently converted ticks

R5 reviewed immutable `c5448921960ead05d0f0d4846fc627fbcf035fc6`
(above immutable prerequisite `94e168ae` and `a8e33f20`). Its P2 trigger is a
4/4 song with a 7/8 change at tick 485, played at tick 3845 in a longer song.
The origin and current position are independently divided by 960. Subtracting
these rounded beats before division yields `0.9999999999999999` bars, so the
old helper reports the preceding downbeat. The correct origin is tick 3845,
zero-based bar 2, consistent with the project's shortened-bar conversion.

The repair changes only the checked arithmetic in host `Transport::bar_position`.
It does not add fields, alter the public anchor/default/advance contract, change
the bridge ABI, or modify absolute beats, seconds, tempo, playing or signature.
For anchors that round-trip exactly from an integer 960-PPQ tick and integer
bar widths, downbeats are derived from the absolute integer tick sum followed
by division by 960. The supported song ticks are exactly representable in f64;
the helper explicitly bounds reconstructed ticks to `2^53`. Other anchors use
absolute fused multiply-add boundaries without claiming a tick-grid origin.
The initial quotient supplies a candidate, with at most one correction in each
direction. A final half-open-boundary check refuses unrepresentable results.
There is no epsilon, rounding of the current position, loop, allocation, lock,
wait, I/O, error string or plugin call in the helper. `None` retains the exact
legacy scalar calculation, including negative pre-roll bars. Invalid anchors
and native i32 index overflow remain refusal, rather than lost metadata.

Actual compiled RED/GREEN:

- Before the production fix, both new concrete ABI regressions failed at
  `origin = 485 / 960`, `position = 3845 / 960`. CLAP's bar start was
  `FixedPoint(1084926635)` instead of `FixedPoint(8601119403)`; VST3's
  `barPositionMusic` was `0.5052083333333334` instead of
  `4.005208333333333`. The native builders are the same functions used by
  processing. GREEN checks the exact tick-derived origin and CLAP index 2,
  preserving their absolute clock/tempo/signature/playing fields.
- Concrete CLAP/VST3 tests separately check one tick before, fractional ticks
  before/after, `next_down`, exact boundary, `next_up` and the next complete
  bar. Inputs use independent absolute tick divisions, not anchor-plus-width
  construction. A one-ULP earlier position stays in the preceding bar;
  it is not promoted by a tolerance.
- Host helper cases cover five anchor magnitudes (including near u32's upper
  bound), denominator 2/4/8/16 bar widths, three successive boundaries and
  their representable neighbors. Non-grid anchors, last valid i32 bar,
  overflow exactly at the boundary, unrepresentable anchors, legacy `None`,
  invalid signatures and unchanged `advance` remain checked. All seven prior
  host meter tests are retained without weakening their assertions.
- The actual engine hosted instrument/effect probe for both saved format
  identifiers now observes independent tick-485 anchors and tick-3845/7205
  positions, song/pattern/source switches and selected-start clamping to 3845.
  It preserves all prior transport assertions and zero callback allocator/
  deallocator calls. This observes engine forwarding to hosted factory probes;
  it does not execute the N4 runtime/bridge or an installed external plugin.
- The project's new fixture proves tick 485 starts one-based bar 2, tick 3845
  starts bar 3, tick 7205 starts bar 4, and neighboring ticks/inverse conversion
  and shortened-bar rejection agree. No project production validator changed.

Final executed GREEN is **33 focused tests**: 13 host meter tests, ten engine
timeline tests and ten project timeline tests. The engine run retains the
stopped-skip hold, PDC/tails, all clip sources/offsets, tempo-ramp and linear
buffer/stream/stem parity, navigation and bounded-callback regressions.
Strict all-target Clippy (`-D warnings`) passed for project, engine and plugin
host; workspace fmt and `git diff --check` passed. Commands use the existing
private target, one Cargo process, jobs/tests 1 and isolated TS export directory:

```text
cargo test -p windfall-plugin-host --lib independently_converted_ticks_report_the_exact_native_downbeat -- --test-threads=1
cargo test -p windfall-plugin-host --lib meter_tests -- --test-threads=1
cargo test -p windfall-engine --test engine timeline:: -- --test-threads=1
cargo test -p windfall-project --test timeline -- --test-threads=1
cargo clippy -p windfall-project -p windfall-engine -p windfall-plugin-host --all-targets -- -D warnings
cargo fmt --all -- --check
```

The first command is the two-case compiled RED; the remaining commands are
GREEN/strict checks. Local excluded evidence logs are
`target/timeline-r5-host-red.log`, `timeline-r5-host-green.log`,
`timeline-r5-engine-green.log`, `timeline-r5-project-green.log`,
`timeline-r5-clippy.log` and `timeline-r5-fmt.log`.

Final private executable provenance, 2026-10-08 UTC, under
`target/timeline-verification/debug/deps`:

| Binary | Written UTC | SHA-256 |
| --- | --- | --- |
| `windfall_plugin_host-a142c4b240164585.exe` | 09:05:32 | `32aec03029aa56e1fb9892740dc1db193e0250311f20437a89cc1d83f46a3770` |
| `engine-d477693dd860322b.exe` | 09:02:38 | `580d6b4077615952f7d2bf673d7590082c27c2f2c8712727df0ab8725001cec0` |
| `timeline-8c4ccc693213df2b.exe` | 09:03:53 | `cbf81dd84d77d68613d8fdb548d00a79fab14b4d970d940bd900e1f37a2a70ac` |

Their corresponding Cargo `.fingerprint/windfall-{package}-{suffix}/test-*.json`
timestamps match; `.d` dependency paths identify this bound checkout. Executed
inventories confirm 13 host meter, ten engine timeline and ten project tests.
Final source SHA-256 values:

| Source | SHA-256 |
| --- | --- |
| host `events.rs` | `6cc27bc5ad26977596bf86ca5aae1234068f91e25643da0dc5eb2903af2c09eb` |
| host `clap/processor.rs` | `3e6a3008045b2c1b1f7039067d1daca49e62abfb99234e2ef96f46912a966297` |
| host `vst3/processor.rs` | `4655911c38811370f97065d9a0ee9438db6ac6d56b33d71eeb0aa5a82a3b16de` |
| engine `tests/engine/timeline.rs` | `24beda1025f6402e5193b4bfed84832bedb99b108b5c90f4ef23d310ae5d1649` |
| project `tests/timeline/meter.rs` | `ad446aac231e4d34eb22648a7bb8e5f65e2498e9e362575142ddcdbb9ee471fb` |

No WASM input/model/UI source changed in this repair; no bindings/WASM were
generated or committed, and no UI tests were run against restored artifacts.
Earlier UI/Session/Controller evidence remains its original provenance. Their
canonical generation/revision/request guards, cancellation/hydration policies,
meter publication gates and typed preparation error are unchanged. The R5 P3
signature-validator duplication is nonblocking and intentionally unchanged in
this authorized arithmetic-only repair.

The acknowledged whole-composition P1 preparation/retirement-under-guards
objection remains open for its serialized owner. No Controller/Plan/State/Rack,
utility/adoption, Session/IPC/runtime/bridge/render, sampler/portable/history,
native fixture, root or other-owner source was edited or imported. N4 forwarding,
versioned ABI and checked render refusal, combined-root/artifact acceptance,
physical UI/audio, installed external plugins and non-Windows behavior remain
unverified here. Arrangements, per-pattern timelines, links/groups/make-unique/
scrub, WAV marker metadata and scalar snap/grid refinements remain open.
