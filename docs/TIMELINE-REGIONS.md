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

The UI records the target before publishing its region, so a delayed region
reply does not lose request identity. It keeps `playPending` until a fresh Play
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
