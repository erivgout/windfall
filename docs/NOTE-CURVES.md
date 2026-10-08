# Per-note expression curves — source work in progress

This packet adds saved curve data, atomic editing, a piano-roll editor and
musical-clock sampling for sampler and instrument occurrences. This is source
coverage, not verified expression parity; the whole-project goal remains open.
Builds, tests, reviews, browser checks and artifact generation remain deferred.
No GitHub CI or Actions are used.

## Saved data and edits

`Pattern.note_curves` stores curves keyed by stable `NoteId` and one of pan,
release, fine pitch, modulation X or modulation Y. Notes keep their existing
Copy value shape. An omitted curve list loads empty. Positions are fractions of
the sounding note's musical duration, so changing note length does not leave
points outside it. Curves use native property units and the same segment bend
formula as automation, with hold segments and first/last-value extension.

Validation requires an existing source note, unique note/property ownership,
1–256 finite strictly ordered points in [0,1], valid native property ranges and
segment bends in [-1,1]. A pattern has a 65,536-point aggregate budget. Nonfinite
evaluation positions return no value. Curve sampling uses binary point lookup.

`SetNoteExpressionCurves` compares captured complete notes in the named channel
and the captured curves for those IDs, replaces only those notes' curves,
validates the proposed pattern and commits one history transaction. PatternInfo
edits carry curves forward/backward with normal pattern patch publication. Final
transaction cleanup prunes curves whose notes no longer exist. Pattern and
channel duplication remap curve ownership to the newly allocated note IDs;
ordinary note edits keep the original IDs and curves.

## Piano-roll editor

The note context menu and command palette open “Note expression curves” for the
captured selection. Choose a source note and property, enable its curve,
double-click to insert points, drag or use arrow keys to move them, and edit
position/value/bend/hold fields. Delete removes a point while retaining at least
one. The editor supports copying the active normalized curve to all captured
notes, removing one property curve and clearing the selected notes' curves.
Apply is one native command; Cancel leaves the project unchanged.

The SVG preview samples the actual segment formula. An unsaved property initially
shows the note's static value. Pointer moves remain in the local draft. Project
replacement, history navigation and workspace disposal close the editor;
revision/generation changes disable Apply, and a refused apply retains the draft.

## Compiled source identity and playback

Compiled note events now retain their saved note ID. Each compiled pattern stores
curves sorted by source note and property, with a binary source range lookup.
`Plan.note_curve_controls` resolves a pattern/note pair independently of event
ordering and applies curves to a supplied normalized occurrence position and
base controls. It performs no allocation or ownership transfer.

Every sequenced sounding occurrence carries value-only `CurveSource` metadata:
pattern/note IDs, its own start/end, optional song-pass origin and base controls.
Pattern-mode timing uses the placed note's musical duration. Song timing converts
clock-domain ticks back through the tempo map before normalizing curve position;
the clip's end truncates the sounding duration. Repeated clips/passes create
independent source clocks even when their saved note ID is shared.

Sampler voices evaluate active curves per output frame before reading audio.
Pan follows the existing channel pan sum, fine pitch adjusts playback step and
translates an active glide, and X/Y update the per-voice low-pass filter. Release
curves choose the release duration before key-up; an already-running release
retains its trajectory. The last curve value is held through a sounding release.
Retiring sampler copies which have left their channel freeze their last controls
while they finish the existing fade and routing history.

Instrument held records retain the same source metadata alongside their separate
instance IDs. Active curves update the instance expression/pan and cooperate with
fractional pitch/glide updates. The note-off boundary is sampled too, including
hold-segment jumps at 100%, before releasing that owner. Synth expression updates
read current internal control/envelope values without advancing their clocks.
Ordinary noncurved nongliding notes retain block processing. UI-live/hardware
notes have no saved curve source.

Clock shifts and tempo-map replacement move source endpoints/origins along with
existing note endings/glides. Plan adoption refreshes curve availability, enabling
new curves on held sources. A source which loses its curves samples its captured
base controls rather than latching the last curve value. Plugin owner inheritance
copies the held source clock and current controls and restores fractional pitch
when supported. The [native plugin expression pass](NOTE-PLUGIN-EXPRESSION.md)
connects the desktop CLAP/VST3 provider to per-instance controls; MIDI-only inputs
retain fallback and unsupported VST3 controls are skipped. Realtime and
offline/stem processing share these paths.

Clipboard copy retains detached curve payloads indexed to its notes. Paste,
Duplicate and Ctrl-drag copies submit those payloads with their new notes in one
native insertion transaction. Native IDs are allocated before curve ownership is
assigned, and point-budget or curve validation refusals roll back the whole edit.
Fresh draw, paint and built-in chord/scale stamps start with fresh note controls.

Native generators retain source provenance for every generated note. Chopping
crops and rebases each source contour to the resulting note's temporal slice,
including segment bend and hold behavior. Other transforms preserve normalized
contours on their retained or newly allocated owners. Glue keeps different curve
shapes separate, and Rhythm Add includes curves in duplicate identity. Undo uses
the same note and pattern-metadata transaction as the rest of these edits.

Remaining source work includes slide-control-note curve policies, bridge
expression integration, live MIDI/MPE transport and live expression recording. Slide control
notes use their initial curve values with the existing held-chord slide and do
not become sounding curve owners.

Later QA must cover overlapping same-key notes, repeated clips/loops, resizing,
moving, source removal, duplication/history, point/aggregate limits, held
segments, native/UI bend agreement, clock changes and realtime/offline ownership
and block partition behavior. Provisional TS bindings need combined generation
with the other feature packets in the deferred build pass.
