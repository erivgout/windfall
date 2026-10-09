# Previously unlisted DSP registry kinds

The effect and instrument registries now expose the following existing
processors. All kinds are appended, preserving every existing variant's index
and every existing parameter descriptor's index. Menu labels come from each
parameter type's existing `ParamSet::NAME`. Both parameter unions remain `Copy`.

## Appended effects

After `Vocoder`, in registry order:

1. `EchoBank` — Echo bank
2. `FrequencyDelay` — Frequency delay
3. `SevenBand` — Seven Band
4. `MorphEq` — Morph EQ
5. `FilterBank` — Filter Bank
6. `XyPad` — XY Pad
7. `XyzPad` — XYZ Pad
8. `PanLfo` — Pan Motion
9. `EnvelopeFollower` — Envelope Follower
10. `NoteEnvelope` — Note Envelope
11. `LushSpace` — Lush Space
12. `Tuner` — Tuner
13. `StageStack` — Stage Stack
14. `ControlSurface` — Control Surface
15. `SendTap` — Send Tap

## Appended instruments

After `WaveRide`, in registry order:

1. `AcidLine` — Sequenced resonant bass
2. `TripleOsc` — Three oscillator synth
3. `WaveLane` — Wavetable synth
4. `MacroVoice` — Macro synth
5. `SpeechVoice` — Speech Voice

## Deliberate omissions

- `FormulaSource` and `KeyboardSource` are control-only sources and do not
  implement `Effect`. As described in [control.md](control.md), wrapping them
  requires a host destination binding.
- Every `notemap` type implements `NoteTransform`, not `Effect`, and belongs in
  a note-transform integration rather than either audio processor registry.
- `ZoneSampler`, `ZonePlayer`, `PadSampler`, and `KeyBed` stay outside
  `InstrumentKind` and `InstrumentParams`. Their `Copy` parameters embed a
  32-zone table of 1024-frame samples; including them would make every
  instrument parameter value hundreds of kilobytes.

This registers existing DSP implementations only. Host routing, UI integration,
binding generation, and parity completion remain separate work. This change
does not mark `parity.json` done.

## Validation limitation

Exposing `zones` also exposes its existing large-parameter unit tests; on
Windows, these need a larger test-thread stack (`RUST_MIN_STACK=33554432`).
With that stack, the library suite has one failure: the registry's bare-tag
deserialization check for `AcidLine` reports `missing field cutoffHz`. The four
analog parameter structs currently lack serde missing-field defaults. Their
processor metadata and algorithms are unchanged by this registry update.

The repository's `.cargo/config.toml` points `TS_RS_EXPORT_DIR` at desktop
bindings. Set that environment variable to a temporary directory when running
DSP unit tests to isolate the automatic `ts_rs` export tests.
