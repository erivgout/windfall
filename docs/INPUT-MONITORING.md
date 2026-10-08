# Routed input monitoring — source implementation

Recording can monitor its selected mono/stereo input through an existing mixer
track. The dialog exposes the route, 0–100% input level and a 5–100 ms buffer,
with monitoring off by default. Headphone guidance explains the feedback risk
at the choice. Monitoring enters the selected track before its effects, fader,
pan, routing and meter. The recording WAV retains dry input audio.

The input worker feeds a separate bounded SPSC monitor queue. A 32-tap sinc
converter maps ADC packets onto the DAC rate, using the input/output clock
period fits. Input callbacks still only stamp/convert/queue. Monitoring runs
during count-in even while the recording gate excludes pre-roll. The output
callback dequeues into the selected mixer buffer without allocation, locks,
waiting or disk work. Initial priming accommodates the selected buffer, level
ramps soften startup, and underruns fade the last sample and re-prime. A fixed
bounded excess drain avoids indefinitely accumulating old audio. Monitor queue
overflow drops monitor frames and reports them without corrupting the recorded
source. Buffered duration, dropped frames and starved frames appear in the take
status.

Control messages replace/remove monitor readers at output boundaries; retired
queues are released on the control side. Stop/discard/open failure/take cleanup
ends the owned monitor route. Recording's existing output configuration and
project ownership guards apply. Fresh export controllers carry no live monitor
queue.

No builds, tests, generators, listening/feedback/device checks, browser QA or
review rounds were run. Bindings are provisionally edited. Actual monitor
latency and stability remain unverified. Saved per-track inputs and multitrack
arms are described in [MULTITRACK-RECORDING.md](MULTITRACK-RECORDING.md).
External output routing is described in [EXTERNAL-OUTPUT-ROUTING.md](EXTERNAL-OUTPUT-ROUTING.md). No GitHub CI or
Actions are used.

## Standalone monitoring

The mixer inspector's Listen action now starts every saved enabled input
monitor independently of transport, record arm and recording source mode.
Stop inputs closes the group. It opens one stream per device with independent
route queues, clock fits and the same worker-side sinc conversion, and feeds
the existing native monitor readers. It creates no WAV, playlist clip, history
entry or recording gate. The inspector displays each route's input rate,
clock drift, buffered duration, dropped frames and starved frames.

The session owns a single standalone group, serializes start against output
configuration and recording, and joins an old worker before replacing its
readers. Starting a take closes standalone input streams before recording
opens its own group. Changing output configuration also joins monitoring
before reopening the output. Saved monitor-route changes and project
replacement update a control-side signature; the worker ends the old group
and reports that monitoring must be started again. Mixer gain, pan, effects,
mute, solo and recording arm can change without changing that signature.
Input clock interruption, device loss, input packet overflow or output clock
replacement stops the group and surfaces an error. The worker removes its
owned output readers on exit; session cleanup joins it. Fresh render/export
controllers still contain no live input state.

Standalone controls and status use new native IPC and provisional source
bindings. Plain browser mode reports the native-only capability explicitly.
This section describes source implementation without build, generator, test,
hardware, listening, browser or review execution.
