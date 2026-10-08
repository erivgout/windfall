# Saved take groups and multitrack comping — source implementation

Keeping loop takes or a multitrack recording now appends a saved playlist take
group in the same atomic source/clip transaction. A group has a global project
ID, a name, retained input lanes, each lane's pass number and source clip ID,
and the latest editable composite's clip IDs. Audio stays in normal saved
playlist clips and sample assets. Legacy projects default to no take groups.
Group limits are 256 groups, the current mixer capacity in lanes, 256 retained
passes per lane and 2048 comp ranges. Native commands validate names, unique
pass/clip associations, audio references and group ID ownership.

The playlist's Take groups popover chooses a saved recording group and can
rename it, select its retained sources, remove the association, audition a
common pass across every input lane, or restore the latest composite. Audition
changes the owned source/composite clip mutes together in one undo step; normal
transport performs playback. Ungroup leaves all clips and audio files intact.
Deleting source or composite clips prunes their saved associations and removes
empty lanes/groups; undo restores the association alongside the clips. Ordinary
audio edits keep their clip IDs, and comp capture uses their current windows.
Duplicated clips remain independent until explicitly grouped.

The clip menu and command palette can group selected audio takes manually by
their original mixer assignment, with clip-ID order assigning pass numbers,
or open the saved group of a selected source/composite. Group comp editing
captures the exact group and every retained source, offers passes shared by
all input lanes, and expands each song-range choice onto every lane. The native
command rejects stale groups/sources, missing passes and ranges outside an
input's retained window. It uses one common overlap geometry bounded by the
available audio on every lane, keeping multitrack edits and equal-power fades
synchronized. Adjacent ranges of the same pass coalesce.

One history command creates all composite lanes/clips and optionally mutes
all originals. Replacing the previous composite removes its owned clips and
playlist lanes that become empty; lanes containing other clips are retained.
When replacement is disabled, the previous clips remain muted and independent,
while the group points to its new latest composite. Mixer/Direct output,
sample, gain, pan, reverse, pitch and stretch are inherited per input source.
No render or extra WAV is created. Save, undo/redo and export use the ordinary
native project/clip paths. Recording cleanup still owns every temporary WAV
until the complete source/clip/group transaction succeeds.

This is source coverage only. Bindings are provisional and the checked-in
WASM awaits the deferred generator pass. No builds, tests, generators, hardware
or listening checks, browser QA or review rounds were run. No GitHub CI or
Actions are used.
