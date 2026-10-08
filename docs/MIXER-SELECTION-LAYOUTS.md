# Mixer selection, docks and layouts — source implementation

This packet implements source coverage for `win-mixer-multi-select` and
`win-mixer-docks-layouts`. Builds, tests, browser checks, code reviews and artifact
generation are deferred under the current feature-first instruction. This
document makes no verified acceptance or usability claim.

## Selection and editing

Mixer selection is transient, resets when the document is replaced, and prunes
deleted tracks. Ctrl-click toggles members; Shift-click and Shift+Left/Right
extend a range in visual order across the three docks. The primary track still
owns the inspector, Current utility source and keyboard focus. Clicking an
already selected member retains the group for control gestures.

Fader and pan gestures capture selected track values at pointer-down. Volume
edits apply the focused track's gain ratio to the captured levels (a zero
focused level uses the new absolute level); pan edits apply its signed delta,
clamped per track. Commands are native batches sharing the gesture's undo step.
The toolbar exposes select-all-inserts, clear, mute, solo, unity gain, center
pan, docking, output routing and movement. Master and Current are excluded from
ordinary routing, solo, dock and movement controls. Routing offers destinations
valid for every selected insert and excludes selected sources.

`MoveMixerTracks` captures the complete expected model order and rejects a
stale order, duplicates, missing sources, pinned sources or a selected/pinned
destination. It moves the selected inserts as one block in their original
relative order, using existing list edits within one native transaction. Track
IDs, channel ownership, routing, automation and effect ownership stay intact;
undo restores the list order.

## Docks and sizes

`MixerTrack.dock` saves `left`, `middle` or `right`; absent values mean middle.
Master and Current stay pinned independently of their saved dock field. Each
insert dock has a separate horizontally scrolling virtual window, with the
primary strip kept mounted for keyboard focus. Left and right docks have bounded
widths; the middle dock fills the remaining space and contains Add Track.

The eight explicit layouts are Compact (36 px), Compact 2 (48), Narrow (64),
Standard (80), Wide (104), Extra wide (128), Large (152) and Extra large (176).
Adaptive retains automatic sizing behavior. Each choice caps the strip's
content density, with further reductions when panel height is insufficient.
The app persists the layout preference and restores Adaptive on layout reset.
Virtualization and keyboard reveal use the chosen width.

## Deferred work

FLP docking import remains feature work. Rust/TypeScript bindings were manually
synchronized provisionally; the checked-in simulator remains the earlier
advanced-fill build. Generate combined artifacts during the later build pass.
QA should cover range selection/focus across docks, relative grouped gestures
and clamps, routing/history transactions, stale move capture, layout persistence,
scrollbar alignment, narrow-strip controls, dock resizing and large mixers.
No GitHub CI or Actions are part of this workflow.
