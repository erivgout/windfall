# Mixer disk recording — source implementation

Saved mixer recording settings choose dry hardware input, after track effects,
or after fader/pan. Dry input requires a device; processed modes can record a
track's instruments, playlist audio and incoming routes without opening input
hardware. Enabled hardware monitoring also enters that track and is included
in its processed recording. The inspector exposes source choice alongside
arm, hardware channels, monitoring and recording offset. Strips and palette
actions allow processed tracks to arm without an external input. The recording
dialog displays each group's dry/processed source choice.

The controller prepares one bounded disk queue per processed route and moves
the writer array to the output processor. The mixer copies fixed-size stereo
packets at the chosen stage, before sends/output routing. Post-effects ignores
the track fader/pan; post-fader includes their mute/solo behavior. No callback
allocates, frees, locks, waits or writes files. Arrays retire through the
existing control-side garbage queue. Taps start emitting only once the take's
start frame is scheduled, so slow device opening cannot fill a pre-start queue.

Packets carry native engine frame numbers adjusted by the remaining processing
latency between the track and the master. The worker maps them to the common
DAC start, removes count-in, applies signed recording offsets, and closes at
the take's timestamp end. Queue overflow, non-finite signal, output clock
replacement or processing-latency discontinuity after recording begins reject
the owned group. Unavailable initial ranges remain explicit silence. Disk taps
drain alongside hardware streams and disable their producers before their
readers are released. A mixer-only group needs no ADC stream or resampler.

Processed sources share the ordinary multitrack WAV/loop-pass selection and
single atomic source/clip attachment path with dry inputs. Printed clips use the
saved [Direct output playback route](PRINTED-CLIP-PLAYBACK.md), past all mixer
effects and faders, including Master. Their original mixer assignment is
retained for switching back to mixer playback. Track taps exclude downstream
bus processing, metronome
clicks, final device gain and detached fade-out voices; recording a bus or Master
captures that chosen track's own processed signal.

This is source coverage only. No builds, tests, artifact generators, hardware
or listening checks, browser QA or review rounds were run. Bindings are
provisional and simulator regeneration remains deferred. External output-port routing is now described in [EXTERNAL-OUTPUT-ROUTING.md](EXTERNAL-OUTPUT-ROUTING.md). No GitHub CI or Actions are used.
