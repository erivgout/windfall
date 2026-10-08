# External mixer output ports — source implementation

Each mixer track can now save a post-fader hardware output assignment on the
currently selected output device. An assignment chooses a zero-based left port
and optional right port; the inspector displays 1-based labels. A missing right
port folds stereo to mono using the average of both sides. A stereo pair needs
distinct ports. The current bound is 256 physical output channels.

Non-Master tracks can replace their ordinary mixer output edge or copy to
hardware while keeping that edge. Explicit sends run in either mode. Turning
off the assignment restores ordinary output routing. Effects, track fader/pan
and mute/solo precede hardware output; the device gain applies to every port.
Master assignments route the completed stereo mix, Direct prints, detached
previews/fades and metronome. Without an assignment, Master keeps the legacy
first stereo pair or mono fold on a one-channel device. Multiple tracks on one
port sum before conversion and integer-format clipping. Unassigned ports are
explicitly silenced rather than retaining earlier device-buffer data.

The driver inventory now reports its writable output channel counts. Audio
settings can request one of those layouts, and engine status reports the actual
open count. Opening keeps an explicitly requested count through buffer/rate
fallback; it reports a failure if no format provides that count, rather than
opening fewer ports. Where that count requires another rate, native selection
chooses the nearest supported rate and reports it. The mixer inspector displays
unavailable saved pairs; an unavailable hardware path is silent, with its saved
assignment retained for reopening or editing. Assignments refer to port numbers
of the selected device, rather than opening an independent device per insert.

A prepared multichannel buffer collects each internal processor subdivision
at its correct offset, including transport/navigation boundaries. Allocation
and output layout configuration happen before starting the callback. Callback
work only sums, delays and converts prepared buffers. Port assignments take
effect at output plan boundaries. The native final Master path and external
paths are compensated to the latest selected path, and printed clips receive
that same overall delay. Delay history carries through plan adoption with the
existing latency crossfade. Processing-latency status and transport presentation
use the complete output figure; recording taps retain their pre-device strip
timing and compensate the remaining path through the common DAC clock.

Stereo export remains the Master mix and reflects ordinary mixer routing.
Hardware-only signals appear in their mixer stems rather than being silently
added to Master; strip stem discovery respects replacement of the ordinary
output edge. No offline renderer opens physical output ports or monitors.

Legacy projects/settings default to ordinary stereo routing. Native fixture
constructors and TypeScript interfaces were source-adapted, with bindings kept
provisional. No builds, tests, generators, hardware/listening checks, browser
QA or review rounds were run. Additional native platform/driver support and
the remaining routing/control features stay on the project roadmap. No GitHub
CI or Actions are used.
