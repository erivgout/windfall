# Timeline and region delivery (T1 first scope)

Base: `9b3001f47304167d2cced8dc09fcc8971ca7bf62`. This document is the
implementation contract and will record actual verification before handoff.
The work is owned by `gpt/t3-timeline-regions`; other owners' source and generated
artifacts remain reserved.

## Musical data and conversion

An optional, default-empty project timeline holds ordered meter changes and
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
nonfinite seek values and overflow are refused before mutation. The existing
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
Immutable marker plans are compiled off locks; callback navigation is bounded,
allocation/free/lock/wait/IO-free. No native API or DSP event format is added.

Export takes an explicit optional region in its immutable snapshot. It renders
that linear interval once, with navigation markers disabled: a loop or pause
cannot create an unbounded/stalled export, and skip markers do not remove audio
from an explicitly selected export. Region endpoints use absolute sample-frame
boundaries through the tempo map. Playback starts inside clipped sources at the
correct source position; sequenced notes crossing the region end release there.
PDC remains removed from the front, and fixed/automatic tails follow existing
automation hold and stream/stem duration rules. Buffered and streaming output,
stems and native WAV readback must agree. No region means legacy output.

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

Requested narrow central seams, subject to parent coordination before editing:

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
