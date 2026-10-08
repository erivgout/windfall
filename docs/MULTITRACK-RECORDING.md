# Mixer input routes and multitrack recording — source implementation

Mixer tracks save a hardware input host/device, mono or stereo channel map,
record arm, recording monitor preference, monitor level/buffer and signed track
offset. These fields use ordinary mixer-track commands, captured history,
project serialization and validation. Missing recording settings preserve old
projects. Clearing an input also clears arm/monitor through the UI. Device
availability is resolved when opening capture; a missing saved device remains
visible in the inspector instead of silently switching hardware.

The inspector exposes input/arm/monitor/offset and dry/post-effects/post-fader
recording-source controls, and each strip shows a
record arm button. Track context menus and the command palette can toggle arm
or open armed-track recording. The recording dialog selects single-input or all
armed tracks, shows arm count, and lists live captured frames, latency, drift
and monitor queue telemetry per track. Track and project settings stay locked
while a recording owns its inputs. Headphone guidance accompanies monitoring.

Armed recording snapshots saved routes and opens one native stream per distinct
host/device. Each callback shares its hardware timestamp across mapped stereo
packet queues, with a fixed route count bounded by the mixer capacity. No input
callback allocates, locks, waits or writes files. The group worker visits route
queues fairly, fits independent device clocks, resamples each route to the same
DAC-frame gate, writes separate Float32 WAVs and feeds enabled monitor queues.
All streams open before playback/count-in. Processed mixer taps join the same
group and can operate without an input stream; [MIXER-DISK-RECORDING.md](MIXER-DISK-RECORDING.md)
describes their native frame/processing-latency capture path. Per-track offset is added to the
dialog offset. Unsupported channel sets, missing devices, overflow, timestamp
discontinuity, device loss or writer failure reject the entire owned group.

Group progress counts the shortest currently written input. Stop closes one
capture window and drains each driver's final aligned samples before closing
streams. The keep path crops all inputs to the common recorded frame count,
keeps the same selected complete/final-partial loop passes across tracks, and
prepares every source/clip together. A single project command publishes all
kept recordings as one undo step; no partial track attachment survives failure.
Dry audio clips retain their original armed mixer route; processed prints use
Direct output past all mixer processing, with their original assignment saved
for returning to mixer playback. Each input gets playlist
lanes, with earlier loop passes muted and the latest kept pass audible. The
first input can use the chosen existing playlist track. Working captures and
unkept sources remain owned until successful attachment or discard. Playback
region, mode and loop preference are restored on cleanup. Kept recordings also
create a [saved take group](TAKE-GROUPS.md) with input/pass associations for
synchronized comping and audition.

Monitoring readers arrive as one preallocated owned array, so several inputs
can feed different tracks without output-thread allocation or replacement of
each other's queues. Retired arrays return to the control side for release.
Fresh offline/export controllers carry no hardware monitor state.

This is source coverage only. No builds, tests, artifact generators, hardware
checks, listening checks, browser QA or review rounds were run. Bindings are
provisional and the checked-in simulator has not been rebuilt. External output routing is described in [EXTERNAL-OUTPUT-ROUTING.md](EXTERNAL-OUTPUT-ROUTING.md). No GitHub CI or Actions are used.
