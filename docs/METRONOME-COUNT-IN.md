# Metronome and count-in — source implementation

The transport's **Click** toggle enables native metronome playback. Its menu
sets click volume and first-beat accents; the command palette and transport
context menu expose the same toggle. Preferences are session transport state,
outside project undo history. Optional IPC fields preserve older messages.

The native sequencer schedules beat triggers into its existing fixed-capacity
event rounds. Pattern mode uses the selected pattern's meter map; song mode
uses the playlist map, tempo warp, region and navigation clock. An unaligned
meter change begins a downbeat. Short synthesized accented/unaccented clicks
render through a preallocated buffer at trigger frames, then join runtime
output past the mixer's stem taps. Fresh offline render controllers leave the
metronome disabled. Device replacement carries transport click preferences.

Recording offers no count-in or 1, 2, 4 or 8 bars. Native pre-roll computes
notated beats from the meter at the chosen playlist tick and samples per beat
from its tempo. The output processor emits those clicks while sequenced
playback waits, splits processing at the exact pre-roll deadline, then begins
song playback from the chosen tick. Beat progress is published atomically and
shown in the recording dialog. A temporary non-looping region reaches the
maximum song tick so a recording can continue after the existing playlist or
start in an empty project. Stop/discard/error cleanup restores the previous
region and loop preference and ends the owned playback.

Capture now opens before count-in and carries ADC timestamps through a bounded
packet queue. The worker trims against the scheduled DAC frame rather than
delivery time, converts native input rates with a windowed-sinc interpolator,
fits independent device drift, and applies the chosen recording offset.
Playback can also synchronize without count-in. Stop closes a timestamp window
and drains driver-delayed tail packets before finalization. Reported duration
counts frames actually written. [RECORDING-ALIGNMENT.md](RECORDING-ALIGNMENT.md)
describes this source implementation. Monitoring and take workflows remain
implementation work; hardware alignment and quality are not verified.

No tests, builds, typechecks, artifact generators, browser checks, QA or reviews
were run in this feature-first pass. Bindings were provisionally edited in
source; all generated artifacts and acceptance work are deferred. No GitHub
CI or Actions are involved.
