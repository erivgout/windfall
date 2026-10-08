# Step graph implementation

The channel rack now has a graph pane for the selected channel and pattern.
The pane shares the rack's step pitch and horizontal scrolling. Its controls
stay pinned beside the channel controls. The toolbar, row menu, rack menu and
action palette expose it; Alt+G toggles it inside the rack.

The source pass covers stored note velocity, pan, MIDI key, release, fine
pitch, modulation X/Y, length and start offset within a sixteenth-note step.
Length has selectable step, beat,
bar and pattern-sized display ranges. Every visible note receives a bar;
chords and multiple off-grid starts inside a step use separate bars. Notes
beyond the current pattern end are preserved outside the graph.

Painting applies to existing notes in each visited step. Fast pointer moves
interpolate across skipped steps. Empty steps stay empty. Right-drag or
Alt-drag restores defaults; double-click resets one note. Arrow up/down edits
the focused note, Shift increases the increment, Home/End use the range ends,
Delete/Backspace reset, and left/right move between note bars.

A stroke previews locally and sends one `UpdateCapturedNotes` transaction when
released, retaining note identities and unrelated properties. Project
replacement, history navigation, changed lane notes, changed pattern length
or changed graph controls cancel a pending preview. Pointer cancellation
discards it. The graph refuses a commit if its captured lane was changed;
the native command also rejects changed or missing captured notes atomically.

Release, fine pitch and modulation are stored in each note's optional
expression object and reach built-in sampler/synth voices. [Note-expression
details](NOTE-EXPRESSION.md) describe the mappings, MIDI/FL import handling
and the remaining hosted-plugin and note-identity work.

This source implementation has not been built, tested or reviewed under the
user's feature-first instruction. Full parity acceptance and interaction QA
are deferred until all feature implementations are present.
