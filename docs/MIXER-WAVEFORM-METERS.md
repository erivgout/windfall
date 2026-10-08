# Mixer waveform meters — source implementation

Source coverage for `win-mixer-waveform-view`. Builds, tests, browser inspection,
reviews and artifact generation remain deferred. This is implementation coverage,
not a verified realtime-performance or display-correctness claim.

## Signal and transport

The engine captures each requested insert or Current strip after its effects,
integrated EQ/stereo stage, fader and pan. Master uses the completed mixer signal
including printed audio and fading voices, before metronome/device gain. Existing
peak meters remain independent.

Each allocation seat has a preallocated accumulator and shared atomic slot.
Requested tracks produce signed left/right minimum and maximum samples in
approximately 5 ms buckets (`max(sample_rate / 200, 1)` frames). The slot retains
64 buckets, about 320 ms, in a fixed ring. Non-finite telemetry samples become
zero and display values are bounded to ±8; this does not alter audio. Writers
perform no allocation, deallocation, locking or waiting. Off-audio snapshot
readers make at most four seqlock attempts and omit a contested slot for that
frame rather than wait indefinitely.

Optional `RealtimeFrame.waveforms` entries carry stable track IDs, stream epoch,
bucket sequence, sample rate, bucket frame count and oldest-first stereo extrema.
Only requested tracks are serialized. Native interest is bounded at 128 tracks;
the controller also caps direct engine callers. Histories reset when capture
starts, track identity changes, samples are discontinuous or the rate/epoch
changes. Device-stream starts and project replacement advance the epoch; project
replacement also clears native interest. A closed stream returns no waveform
payload.

## Display and lifecycle

The mixer toolbar switches Levels/Waveforms, with the preference saved in app
storage and Levels restored by layout reset. Waveforms replace the meter beside
the fader, while peak readouts and held clipping continue through the existing
peak feed. Stereo lanes show signed extrema, center lines and clipping color;
short history fills the trailing end of the fixed 64-bucket time span. Upright
meters scroll vertically and flat meters horizontally.

Canvases request interest only while their strips are visible. One app-window
owner coalesces those requests, follows document/layout changes, and supplies
generation/revision guards to the native command. Hidden mixer panels request no
tracks. Frames draw directly to canvas without sending 60 Hz audio telemetry
through React state. Resize and theme changes invalidate drawing. Native request
errors are surfaced in the toolbar. The browser mock validates interest requests
but supplies no fabricated audio waveforms; its display identifies desktop audio
availability in the tooltip.

## Deferred validation

Exercise signed stereo extrema and silence, history duration across sample rates,
clipping/reset, stopped/lost streams, project replacement with reused IDs, track
reorder/deletion and Current source changes, virtualized scrolling and dock/layout
resizing, native contention, maximum visible interests and callback allocation
behavior. TypeScript bindings were provisionally synchronized; regenerate the
combined bindings and simulator during the later artifact/build pass. No GitHub
CI or Actions are used.
