# Loop recording takes — source implementation

The recording dialog offers a loop from the chosen start tick to an end tick.
Native recording owns song playback, opens input before count-in, temporarily
sets the loop region and enables region looping. The compiled tempo map defines
each pass's fractional sample duration; cumulative rounded boundaries prevent
per-pass rounding drift. ADC audio is resampled onto the actual output frame
grid. Regions must last at least 250 ms. A take can contain at most 256 passes.

State polling describes complete passes and the current partial pass. Take
checkboxes exclude unwanted passes; the latest-only choice resolves after the
capture tail drains, so it includes a final partial pass. Native callers can
also request all passes or an explicit index set. Invalid explicit selections
are refused before ending the active take. Discard retains no sources.

Stop finalizes the owned capture, partitions selected passes into separate
Float32 WAVs on the session worker, and prepares all sources and clips as one
project command. A single commit publishes them with one undo step. Each pass
starts at the loop's first tick, using the complete region length or the tempo
map's partial duration. Kept takes share a mixer route and occupy separate
playlist tracks; the latest kept pass can use the chosen existing track. Earlier
clips are muted and the latest is audible. Ordinary playlist mute/delete/edit
controls support auditioning and retaining those sources. This does not yet
implement a comp-lane editor.

All reserved split files stay owned until attachment succeeds. Failure removes
only owned capture/split files and leaves the document unchanged. Success keeps
selected source files for undo/save, removes the continuous working capture and
restores the prior loop, region and play mode. Recording blocks ordinary project
edits throughout capture and attachment.

Armed inputs share loop-pass selection and publish as one recording group;
[MULTITRACK-RECORDING.md](MULTITRACK-RECORDING.md) describes that source path.

This is provisional source coverage. Builds, tests, generated artifacts, QA,
listening/hardware checks and review rounds remain deferred. No GitHub CI or
Actions are used. Printed-master playback routing and comping remain implementation work.
