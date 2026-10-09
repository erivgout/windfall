# FL Studio channel voice import seam

`crates/windfall-flp/src/convert/voice.rs` imports the channel's optional
`Polyphony` block (event 221 in `model.rs`) after `AddChannel` succeeds in
`convert/channels.rs`. This includes silent instrument placeholders and
note placeholders created for audio or automation clip channels. Layer
channels have no corresponding Windfall channel; their notes go to children.

The importer dispatches `SetChannelVoiceSettings`, which replaces the whole
voice block, starting with `ChannelVoiceSettings::default()`:

- `max == 0` means unlimited in FL Studio and becomes Windfall's maximum, 32.
- `max` in `1..=32` copies through; larger values clamp to 32.
- Flag bit 0 sets `polyphony.monoLegato`, independently of `maxVoices`.
- Flag bit 1 means portamento is enabled in FL Studio. The repository has
  no documented conversion of `Polyphony.slide` from FL's scale to
  milliseconds. `polyphony.portamentoMs` stays at its default, 0, and the
  channel report says the glide time was not decoded. This also applies
  when the stored slide value is zero. A slide value alone does not enable
  portamento.

Unlimited/clamped voice limits and undecoded enabled portamento are reported
as approximations. The channel's overall count retains its worst outcome,
with one count per channel. Exact voice mappings need no additional line.
Without a polyphony block, the importer sends no voice command, retains the
default settings, and adds no report line.

Arpeggiator, echo, and channel envelopes retain all their defaults. In
particular, arpeggiator mode is off and echo is disabled. This block does not
store settings for those tools, so none are inferred. The existing 3x Osc
synth-parameter polyphony and envelope mapping remains intact alongside the
new channel voice command. See [channel voice settings](channel-voice.md).

## Focused verification

Tests run the full converter and check the resulting channel voice block,
project validity, and channel report. They cover mono with max 4, unlimited
max, missing polyphony, undecoded portamento with either mono state, every
supported maximum plus oversized values, and slide without portamento.
Full-block comparisons verify that the other tools retain their defaults.

Run from Git Bash (`C:\Program Files\Git\bin\bash.exe`):

```bash
source scripts/msvc-env.sh
rustfmt --edition 2024 --config skip_children=true crates/windfall-flp/src/convert/mod.rs crates/windfall-flp/src/convert/channels.rs crates/windfall-flp/src/convert/voice.rs
cargo test -p windfall-flp --lib convert::voice
```

Verified on 2026-10-08: all six focused tests passed (92 filtered out).
The three changed Rust files were rustfmt formatted with child-module
traversal disabled to preserve other agents' files.
