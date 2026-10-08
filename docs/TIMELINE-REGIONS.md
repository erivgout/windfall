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
