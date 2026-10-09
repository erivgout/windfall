# Channel voice tools

Persistent Copy/serde/TypeScript settings now live in `crates/windfall-project/src/channel_voice.rs`. Each `Channel.voice` defaults and sanitizes on deserialization; project checks reject invalid in-memory settings. `SetChannelVoiceSettings` sanitizes the complete replacement and uses the existing channel undo edit. The project crate has no engine dependency.

`crates/windfall-engine/src/channel_voice.rs` retains the fixed-storage processors and re-exports the model settings. `PlanChannel` carries sanitized settings; `PlanState::try_build` prepares aligned per-channel routing storage and envelope templates off the audio thread. Unchanged settings transfer runtime state by channel ID; replacement/removal releases tracked instances and retires the old prepared state through the existing engine path.

The selected-channel inspector mounts `ChannelVoicePanel`, dispatches the replacement command, and keys the draft by channel/settings so undo and project replacement restore the editor. The panel uses generated model types. No parity acceptance status is changed by this integration.

## Persistent shape

`ChannelVoiceSettings` and every settings struct are `Copy`, `Default`, and serde camelCase with struct defaults. Deserializing `{}` produces the complete default. A future channel field must itself have `#[serde(default)]` so old channels can omit it. Call `sanitized()` after deserialization and before preparing runtime state. Floating-point non-finite values recover to defaults; finite values clamp. Rust integer fields must already deserialize within their u8/i8 representation; the UI sanitizer also truncates integer fields.

The top-level fields are `arpeggiator`, `echo`, `polyphony`, and `envelopes`:

- `arpeggiator`: `mode` (`off`, `up`, `down`, `upDown`, `asPlayed`), `rate` (`quarter`, `eighth`, `sixteenth`, `thirtySecond`), `gate` (0–1), `rangeOctaves` (1–4). Defaults: off, sixteenth, 0.75, 1.
- `echo`: `enabled`, `time`, `feedback` (0–0.95), `pitchSemitones` (−48–48), `repeats` (0–8). `time` is `{ "unit": "milliseconds", "ms": 250 }` (1–60000 ms) or `{ "unit": "division", "division": "eighth" }`. Defaults: disabled, 250 ms, 0.5 feedback, 0 semitones, 3 repeats.
- `polyphony`: `maxVoices` (1–32), `monoLegato`, `portamentoMs` (0–60000). Defaults: 32, false, 0 ms. Mono legato enforces one surviving voice regardless of the stored maximum.
- `envelopes`: `filter`, `pitch`, `pan`, and `lfo`. Each envelope has `enabled`, `attackMs` (0–10000), `decayMs` (1–10000), `sustain` (0–1), `releaseMs` (1–10000), and `depth`. Depth bounds/units are ±8 cutoff octaves, ±2400 pitch cents, and ±1 pan. Defaults match `windfall_dsp::EnvelopeParams`: attack 2, decay 200, sustain 0.8, release 150; additional envelopes are disabled with depth 0.
- `envelopes.lfo`: `enabled`, `shape` (`sine`, `triangle`, `square`), `target` (`filter`, `pitch`, `pan`), `rateHz` (0.01–30), `depth` (−1–1). Defaults: disabled, sine, filter, 5 Hz, 0 depth. Depth is normalized; magnitude 1 means 8 octaves, 2400 cents, or full pan travel. Negative depth reverses polarity.

All additions default to neutral processing. This additional envelope set does not replace or disable the sampler's existing volume envelope. Spectral stretch remains the existing sampler implementation.

## Plan and voice integration

In `plan.rs::compile`, copy sanitized settings into each `PlanChannel` and prepare channel runtime state on the control side. Keep moving state outside the immutable plan, aligned to channel IDs in `PlanState`. Preallocate scratch buffers and event identity mappings; none of these processors allocate after `prepare`, including chord edits and event scheduling.

Prepare `Arpeggiator::prepare(settings.arpeggiator)` and `NoteEcho::prepare(settings.echo)` once per channel. Maintain the incoming chord separately from the output voice limit. `Arpeggiator::set_chord(&[HeldNote])` accepts play order, removes duplicate keys, and retains the first 16 distinct keys. Expanded octave notes above MIDI 127 are skipped. Up/down sort all expanded pitches; as-played keeps input order within each octave; up-down does not duplicate endpoints. A chord edit restarts the sequence and schedules the old note's release at the next processed frame. Off mode emits nothing: bypass it and route original notes directly. Gate zero is silent; positive gates last at least one sample.

Call `Arpeggiator::process_block(sample_rate, bpm, frames, &mut NoteEvents)` before dispatching notes to `voice.rs::Voices::start`. Use `NoteEvents::as_slice()`: offsets are relative to this processed subblock. Musical intervals retain fractional frame residuals, so splitting blocks does not change event timing. Tempo changes affect newly started steps; an in-flight step finishes at its previously chosen duration.

Call `NoteEcho::schedule_note(note, offset, duration_frames, sample_rate, bpm)` once for a source note with a known gate, before `NoteEcho::process_block(frames, &mut NoteEvents)`. Offset is relative to the next echo subblock. This schedules only the echoes, with matched on/off events preserving the gate (minimum one sample). For sequenced notes, determine the gate from the musical clock. The current API requires a known duration; for live held notes the parent must choose a defined gate policy or extend paired live-note scheduling before claiming live echo support. Do not recursively schedule generated echoes. The delay/tempo is captured at scheduling time. Repeat n shifts pitch by n times the offset and scales velocity by feedback to power n; stop below velocity 0.001 or outside MIDI 0–127. Zero feedback schedules no repeats.

Echo capacity is 256 delayed events (128 complete pairs). `schedule_note` returns false if the full requested set will not fit; it queues nothing from that request. Handle that result explicitly, e.g. retain the dry note and report rejected echoes to diagnostics. It never drops an already queued note-off. `pending_events()` exposes queue occupancy. Echoes at the same frame release before starting.

Both event processors cap each call at `MAX_BLOCK_FRAMES` (256). `NoteEvents::processed_frames` reports how much was advanced, even when there are no events. Split larger blocks, consume the batch, and repeat until every frame has been processed. `NoteEvents` stores at most 512 events. Use separate scratch batches for arp and echo, then merge chronologically on the audio side, placing off before on at equal offsets. Do not overwrite an unconsumed batch.

`NoteEvent::id` pairs generated on/off events. Scope it by channel and processor and map it to the engine's voice/note instance identity; release by identity, not key, since echoes of the same pitch may overlap. Preserve original source/expression/origin/routing metadata when adapting `HeldNote { key, velocity }` into the existing `Note`. Generated IDs use wrapping u64 counters. Do not use the current broad key-release path to terminate all same-key echoes accidentally.

Before starting a generated/dry note, call `admit_note(&mut HeldNotes, key, settings.polyphony)`. It maintains oldest-to-newest surviving keys, reports every existing key to release through `PolyphonyDecision::released_keys()`, and refreshes duplicate keys instead of stacking them. A smaller limit evicts every excess oldest key. `retrigger` is false for overlapping mono-legato notes. In that case transfer the existing mono voice's state to the target pitch, rather than killing it and starting fresh envelopes. `HeldNotes::note_off(key)` removes a surviving key; evicted keys are not remembered for last-note fallback. For same-key repeated generated instances, apply this policy deliberately and keep their release identity separate so an obsolete off cannot stop a newer note.

Compute `settings.polyphony.glide_coefficient(sample_rate)` once per configuration. In the per-sample pitch path, apply `pitch += coefficient * (target_pitch - pitch)` in semitones (or cents consistently). Zero ms returns 1 and snaps. Time is the exponential time constant: 63.2% of the gap closes in the specified time. Preserve `note_glide.rs::NoteGlide` for musical note articulations; the parent must define precedence/composition with this independent channel portamento, not replace it. Use the existing sampler playback-rate and spectral-bank selection paths.

Each sounding voice gets `PreparedChannelEnvelopes::prepare(settings.envelopes, sample_rate)` alongside its existing volume envelope. Call `gate_on()` when `retrigger` is true; call `gate_off()` on its matching note-off; call `process_block(frames, &mut ModulationBlock)` in `Voice::render`. This also caps the call at 256 frames; use `ModulationBlock::frames` and process the remainder. The fixed arrays are `filter_cutoff_offset`, `pitch_cents`, and `pan`; unused tails are cleared. Filter output is octave offset: multiply the existing cutoff by `2^offset` and clamp below Nyquist before configuring the existing filter. Pitch output changes the playback ratio by `2^(cents/1200)`. Add pan to the existing note/channel pan and clamp before pan gains. Combined envelope/LFO offsets clamp to ±8 octaves, ±2400 cents, and ±1 pan. The LFO is per voice and free-running after preparation; gate-on does not restart its phase.

On transport stop, seek, channel removal, or a prepared-settings replacement, release voices and flush queued state. `Arpeggiator::reset()` returns the sounding `(HeldNote, id)` to release; it retains the chord for a transport restart. `NoteEcho::reset()` discards pending pairs after voices are released. Clear `HeldNotes`; reset per-voice envelopes/LFO through `PreparedChannelEnvelopes::reset()`. Settings are immutable in prepared event processors; reconstruct them on the control side for an edit, following the engine's existing safe state-transfer/retirement conventions.

## Connected audio path and remaining limits

`processor.rs` routes source notes into the incoming chord when arp is enabled, otherwise keeps the dry source. It runs arp before scheduling echoes, then consumes separate batches with offs before ons and admits output notes before dispatch. Generated `(channel, processor, id)` pairs map to sampler/instrument instance IDs; duplicate-key admission deliberately replaces the earlier owner, and obsolete offs cannot stop the replacement. Rejected echo requests increment the prepared channel runtime's `rejected_echoes` counter; this counter is not yet exposed in the desktop diagnostics.

Event routing uses one-frame subblocks while arp or queued echo is active (always within the 256-frame bound), and bypasses the event loop for neutral channels. All chord storage, source metadata, ownership tables and event scratch batches are prepared in advance. This favors exact boundaries over callback efficiency; performance QA remains pending. Source expression, pan, origin and note-curve identity are retained. Expanded pitches resolve metadata from a matching input chord tone; octave-equivalent input tones with identical velocity are ambiguous in the existing `HeldNote` API.

For sampler voices the additional prepared envelope/LFO runs in bounded chunks, independently of the existing sampler volume envelope. Filter offsets multiply the existing expression cutoff, pitch cents multiply playback ratio, and pan adds to the note/channel pan. Channel exponential portamento follows the musical pitch produced by `note_glide`; articulation retains its own clock-domain trajectory. Mono transfer preserves envelope/filter/position state and uses the prepared spectral bank when selecting a new key.

Instrument owners route pitch/pan/filter through the existing per-instance expression APIs. Filter expression is limited to the instrument API's �4 octave range, and hosted pitch depends on provider support. Additional modulation is not advanced during an instrument's internal release tail after its logical owner has been removed. Its existing volume release remains intact. Full instrument filter range and modulation of instrument release tails are not connected.

**Live echo of a held note whose gate duration is not yet known remains unsupported.** Held live/hardware notes stay dry when arp is off; live arp output has a known generated gate and can feed echo. Sequenced echo gates are captured from the musical clock at scheduling time; generated echoes never feed the echo scheduler recursively.

## UI seam

`ChannelVoicePanel` is mounted in the channel inspector using the saved `channel.voice`, emits `setChannelVoiceSettings`, and remounts when the saved shape changes. This restores the draft for undo, redo and external project edits. Rapid control interaction/remount behavior remains for QA.

## Parity and focused validation

- `win-rack-arpeggiator`: fixed chord, all five modes, tempo divisions, gate, octave expansion.
- `win-rack-echo-delay`: tempo/ms delays, feedback taper, transposed repeats, fixed pair queue.
- `win-rack-polyphony`: bounded surviving notes, mono legato decisions, per-sample portamento.
- `win-rack-channel-envelopes`: additional filter/pitch/pan ADSRs and assignable sine/triangle/square LFO.

`channel_voice.rs` tests ordering/range, mode endpoints, fractional block continuity, requested echo counts and cutoff, pair identity/capacity, mono replacement, limit reduction, glide, attack/release, sine polarity, neutral defaults, clamps, and omitted-field deserialization. The focused panel tests exercise callback count under Strict Mode, accumulated edits, tempo timing, keyboard depth edits, initial-value immutability, and UI sanitization. Run `cargo test -p windfall-engine channel_voice --lib` after the parent manifest and unrelated workspace compile errors are resolved; run only the `voice/voice-panel.test.tsx` Vitest file for the UI.

Focused integration tests: `crates/windfall-project/tests/channel_voice.rs` exercises command sanitization, undo, save/reload, redo and legacy defaults. `crates/windfall-engine/tests/channel_voice_integration.rs` exercises a prepared two-repeat echo through the public processor audio path. Engine execution currently requires resolving existing compile blockers in unrelated device/render/plugin/state work.

The browser WASM document is rebuilt to recognize the replacement command. Browser-only simulated audio/meters still do not emulate the channel voice processors; audio application is in the native engine.
