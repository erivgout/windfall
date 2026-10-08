# Pattern markers and meters — implementation pass

Patterns now carry an optional base time signature and their own timeline of
named markers and meter changes. Older projects inherit the song signature and
have an empty local timeline. Local meters change musical labels and pattern
mode plugin transport; notes and pattern lengths remain absolute ticks. Song
mode always follows the playlist meter map, including when clips reuse a pattern.

The piano roll View menu and command palette open **Pattern markers and time
signatures**. The ruler context menu opens it at the clicked tick. The dialog
sets or clears the base override and adds, edits or removes named markers and
meter changes. Each edit captures the complete existing timeline and base
signature for one native undo transaction. Project replacement and history
navigation close the dialog. Marker rows can seek the active pattern transport.

The ruler labels local bars, beats, markers and signatures. All grid renderers
share meter segments with their own origins. Beat/bar snapping for new notes,
stamps, note drags, resize edges, ruler seeking and length drags reads the local
segment. Auto-extension of pointer edits rounds to bars in the final segment.
The transport position and beat pulse also use the selected pattern's map.

Musical quantize now offers step, beat and bar divisions, plus an absolute
custom-tick grid. The native tool captures the pattern's signature and meter
list, refuses a stale meter snapshot, restarts grooves at changes, and applies
strength independently at each selected start or end. Mixed-meter painting
uses local cell spacing across a whole stroke. Paste follows local bar origins;
nudge advances the selection's earliest onset along local snap lines while
preserving the phrase. Rack beat shades, fractional step ruler positions,
bar separators and length descriptions also use the local map.

Native metronome clicks use the same sequencer clock and selected song/pattern
meter segments as notes. Recording count-in uses the song signature and tempo
at its chosen start. See [METRONOME-COUNT-IN.md](METRONOME-COUNT-IN.md).

Native validation limits each timeline list to 2,048 items, marker names to 256
UTF-8 bytes, and local ticks to the maximum pattern duration. Named markers may
share a tick; meters need distinct ticks. IDs share the document allocator and
are checked across song and pattern timelines. Duplicating a pattern allocates
fresh meter and marker IDs, along with fresh note IDs. Local navigation marker
kinds are rejected; loop, skip and pause semantics remain song controls.

Pattern MIDI export includes its base signature, local changes and named
markers. Song MIDI export includes the playlist's named markers and meters.
MIDI import carries named markers and local meter slices into generated
patterns; phrase sharing compares those slices as well as notes. It also
places imported named markers on the song timeline. FL Studio import maps
documented named and meter markers inside each pattern, rescales source PPQ,
and reports unsupported marker kinds individually.

The TS bindings are provisionally synchronized by source edits. No builds,
tests, checks, browser QA, reviews or artifact generators were run for this
pass. Acceptance and regressions remain deferred. Further recording work
includes timestamped capture gates and input/output clock and latency
compensation; these are implementation work before the full QA pass.
