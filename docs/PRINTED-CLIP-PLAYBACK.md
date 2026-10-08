# Printed audio clip playback — source implementation

Audio clips now save an optional `output` destination: `mixer` (the legacy
default) or `direct`. Direct output sums the clip after all mixer effects,
faders, pans, sends and mixer mute/solo processing, including Master. Clip
gain, pan, fades, reversal, tuning/stretch and playlist mute still apply.
Device gain and the output device apply to the completed sum. The Master
output meter includes the direct sum. Mixer recording taps remain attached
to their chosen strip, rather than recursively capturing Direct output.

Post-effects and post-fader recordings attach with Direct output, including
Master prints. Dry input recordings retain mixer playback. A printed clip
keeps its original mixer-track assignment as metadata. The selection inspector
offers Direct output or any mixer track; choosing a track (or creating a new
one) explicitly returns the selection to mixer playback. All route edits go
through the existing native captured history commands.

A separately allocated native buffer and compensation delay align finished
prints with the main mix's processing latency. Plan replacement carries delay
history and crossfades changed delays. Already-playing clips retain their old
route while fading out when routing changes. The offline renderer includes
prints in the final mix and drains their pending delay; mixer-strip stems
exclude them because they do not enter a strip. Destructive audio editing
preserves the source clip's playback destination, as do clone/split workflows.
New imported files and FL clips use the legacy mixer destination.

Old project files default to mixer playback and omit that default on save.
TypeScript bindings are provisionally synchronized from source. Native fixture
constructors were adapted to the extended schema, without running them. No
builds, tests, generators, listening checks, browser QA or review rounds were
run. Feature coverage awaits the deferred QA pass. No GitHub CI or Actions.
