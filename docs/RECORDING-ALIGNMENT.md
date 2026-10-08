# Timestamped recording alignment — source implementation

The native output callback publishes its processor frame and predicted DAC time
into a bounded atomic clock snapshot before processing transport requests. Input
callbacks map ADC timestamps into the same monotonic epoch and push fixed-size
stereo packets with source frame numbers, input latency and timestamp quality.
Conversion and queue writes are the only work performed on the input callback.

Recording opens its input before scheduling playback. A per-take clock ticket
connects the count-in deadline, including mixer processing latency, to a capture
window. Zero-bar synchronized recording uses the same start path. The writer
gates by capture time, preserving queued pre-roll semantics regardless of disk
delivery delays. Opening another output invalidates the ticket. Stop closes the
capture window and waits for driver-delayed tail samples before shutting down
the input; discontinuities, overflow, device loss and missing tails discard the
owned take. Cleanup releases the ticket and restores the prior region, loop and
play mode. No project edits occur until the finished WAV is attached normally.

Input configuration prefers the output rate or the selected native rate and
otherwise chooses the nearest supported configuration. The worker resamples
onto the output time grid with a 32-tap windowed-sinc kernel and bounded circular
history. The start time follows subsequent output-clock anchors. Independent
input clock period fitting can be enabled; missing driver timestamps use the
nominal period. Unavailable audio is represented as silence rather than moving
later audio earlier. A signed offset from -1000 to 1000 ms advances positive
values and delays negative values. Session readouts show input rate, measured
input/output latency, input drift, timestamp availability and pre-roll trimming.

The dialog defaults to synchronized playback and drift correction. Old IPC
sources without alignment settings retain manual recording placement. Count-in
always owns playback. The original untimed capture seam remains available for
existing synthetic source fixtures; native recording uses timestamp packets.

This documents source coverage only. No builds, tests, typechecks, generators,
hardware checks, listening checks, browser QA or review rounds were run. Binding
edits are provisional. Monitoring, loop takes and multitrack routing remain
required implementation work. No GitHub CI or Actions are used.
