# Channel note timing

Implementation is present; QA and artifact builds are deferred under the
user's feature-first instruction.

Channels store optional `ChannelTiming` settings. Legacy defaults are 100%
global swing mix, gate disabled and zero shift. The channel inspector exposes
these three controls for both samplers and instruments, with one-gesture undo
and a registered Reset channel note timing action.

Swing mix scales the project's global swing independently per channel.
Global swing keeps the existing paired-sixteenth warp; an unpaired trailing
step remains unswung. Gate zero preserves played duration; a nonzero gate caps
the duration after swing, up to 245760 ticks. Shift translates the played
start after swing, from -960 to +960 ticks. Negative starts clamp to zero while
keeping duration. Starts reaching the pattern end do not play in that pass.
None of these controls rewrites the piano/step notes.

The shared Rust `ChannelTiming::place` implements these rules for native plan
compilation and MIDI pattern/song export. Live playback, offline render and
mixer stems already consume those compiled pattern events. Ordinary file and
channel history paths persist timing; clone carries it forward. Commands and
project admission enforce the timing ranges. MIDI's Swing export toggle omits
the swing component while retaining the channel's gate/shift.

The browser's visual transport has a timing mirror for its simulated onsets.
It remains a meter/playhead simulation and emits no audio. The checked-in
shared Rust WASM must be regenerated in the deferred artifact pass to accept
the new model/commands. Source declarations are synchronized provisionally.

This delivery ran no builds, tests, reviews, QA checks or GitHub Actions.
Boundary durations, timing at different tempos, replacement/undo, real audio
and MIDI parity, UI gestures and shared-WASM behavior await the later QA pass.
