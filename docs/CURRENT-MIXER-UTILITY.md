# Current mixer utility — source implementation

`win-mixer-tracks` now has a saved follow-selection utility in source. A mixer
button and command-palette action dispatch `EnsureCurrentMixerTrack`: the
native document allocates one ordinary project id for the utility, or returns
its existing id. The captured mixer transaction participates in undo/redo.
Older files omit the `current` flag and keep their ordinary tracks.

The Current strip is pinned beside Master and has its own saved effect chain,
volume, pan and mute. Its normal native effect/plugin editor, parameter,
automation, state persistence and meter paths use that project id. Selecting
Current opens its chain while retaining the last selected ordinary source.
Selecting Master or an insert changes the source; clearing the selection
retains it. Removing the source falls back to Master. New/Open clears the
native source and frontend remembered id before following the new document.

The frontend has one serial, coalescing selection owner even when the mixer
panel is hidden. Native IPC checks project generation, document revision and
source existence. The controller publishes a numeric source id through an
atomic; audio resolves it through the immutable plan's bounded id index.
Current is processed after the routing graph, copying the selected strip's
post-effects/post-fader stereo block into preallocated scratch. It runs its own
effects and fader and publishes meters without allocating, locking or doing
IO in that path. Its effect state remains continuous when selection changes.

Current has no output/send/hardware/recording route and cannot be a destination
for channels or clips. Document loading and routing commands reject those
assignments. Ordinary routing selectors hide it. Its solo cannot silence the
song; mute only affects the utility copy. It has no contribution to Master or
external outputs, cannot be selected as a stem, does not extend song tails,
and offline processors leave its selection disabled.

The native bound is 502 allocated mixer seats: 500 inserts, Master and the
single Current utility. A separate 501 signal-track bound governs ordinary
track creation, auto-routing on channel/audio import and FLP conversion.
Frontend insert eligibility counts ordinary tracks, so Current consumes no
insert. Native meter/buffer/plan-transfer arrays use the allocation bound;
insert strips continue using viewport virtualization.

Bindings are provisionally synchronized; combined WASM/binding generators,
builds, tests, runtime checks, UI/native plugin QA and capacity measurements
remain deferred. Browser mode retains document/settings workflows but does
not process native utility audio. Further analyzer integration, integrated
track utilities/EQ, control/sidechain routing, mixer layouts and the whole
project's remaining parity scope remain required work. No GitHub CI or
Actions were used.
