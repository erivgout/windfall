# Integrated track EQ and stereo utilities — source implementation

Every mixer strip, including Master and Current, now has saved integrated
processing separate from its ten effect slots. The signal order is incoming
sources → effect slots → three-band EQ → stereo utilities → fader/pan → meter,
output, sends and external ports. Post-effects recording includes the integrated
stage; post-fader recording also includes the fader and pan. Finished Direct
prints continue past Master processing.

`TrackParams` provides low/high shelves and one middle bell, each with on/off,
frequency, gain and Q, plus whole-EQ enable. Frequency is 20–20000 Hz, gain
±24 dB, shelf Q 0.1–2 and bell Q 0.1–18. The audio uses the existing parametric
EQ implementation with its other bands disabled, retaining smoothing, filter
history, sample-rate handling and reported filter tails. No additional latency
is introduced by the integrated stage.

Stereo utilities provide independent left/right polarity inversion, channel
swap and signed separation. Mid is preserved; side gain is `1 − separation`:
−1 doubles side, 0 preserves stereo and +1 folds to mono. Swap follows width,
then output polarity. The resulting 2×2 matrix uses five-millisecond coefficient
ramps. There is no extra clipping or limiter in the matrix.

The optional saved `MixerTrack.processing` field defaults to neutral legacy
behavior and omits default settings on save. Loading and whole-setting edits
validate the native parameter ranges. `SetTrackParam` uses the append-only
17-control descriptor table and a captured mixer edit; whole-setting reset
uses `UpdateMixerTrack`. The native plan carries these parameters into both
realtime and offline rendering. Its inline EQ/ramp state swaps by stable track
id at plan adoption; unused new states retire with the old plan. That path
does not allocate or free filter storage on audio.

`AutomationTarget::TrackParam` addresses the same persisted descriptor indices.
Native range/value lookup, target validation/naming, lane compilation,
processor changes and restoration are implemented. Track deletion removes its
integrated-control automations. The frontend supplies target keys, range/value
formatting, ownership and live value feeds. Integrated processing and its
automation count as shaped signal paths for voice retirement and ringing
state; filter tails participate in the normal tail accounting.

The inspector exposes an expandable panel with all three bands, utility
controls, reset, single-gesture editing and each control's automation menu/live
marker. Rust types and the new automation/command bindings are provisionally
synchronized. Frontend descriptors are a provisional source mirror; the native
descriptor generator now includes a `track` table for the combined artifact
pass. Browser mode retains document/UI workflows and supplies no native DSP.

No build, test, generator, runtime check, measurement, listening or QA review
was performed for this packet. Artifact freshness and all acceptance checks
remain deferred. FLP integrated-EQ/utility mapping, analyzer displays, sidechain
routing, mixer layouts and other whole-project requirements remain work.
No GitHub CI or Actions were used.
