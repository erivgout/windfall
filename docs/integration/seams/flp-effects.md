# FLP effect import seam

`crates/windfall-flp/src/convert/effects.rs` maps project-stored source names
to variants already registered in `windfall_dsp::EffectKind`. Source product
names are import keys only. Matching ignores ASCII case and surrounding
whitespace; marketing suffixes such as `(11 Guitar FX)` and `(12 FX)` are not
part of the keys.

## Existing state decoders

| Source import key | EffectKind | Settings |
| --- | --- | --- |
| Fruity Parametric EQ, Fruity Parametric EQ 2 | `Eq` | Existing band-kind, frequency, gain and approximate width decoder |
| Fruity Compressor | `Compressor` | Existing threshold, ratio, gain, attack, release and knee decoder |
| Fruity Limiter | `Limiter` | Defaults; existing settings-loss report |
| Fruity Reeverb, Fruity Reeverb 2 | `Reverb` | Existing size, decay, diffusion, cuts, damping and mix decoder |
| Fruity Delay, Fruity Delay 2 | `Delay` | Existing time, feedback, ping-pong and mix decoder |
| Fruity Delay 3 | `Delay` | Defaults; existing settings-loss report |

Those decoders and their automation links remain intact. In particular,
the older reverb keeps its decoded `Reverb` mapping despite the additional
room circuit described in [spatial.md](spatial.md).

## Added name mappings

All 42 added keys use `kind.default_params().sanitized()`. Their saved bytes
are not decoded, including nonempty or malformed state. No parameter byte
offsets or automation links are inferred. Each import reports:
“its settings were not decoded, so it starts from Windfall's defaults”.
The existing mixer importer includes this note in an `Approximated` report
line and retains the source slot's enable/mix controls through its normal path.

| Source import key | EffectKind | Parity reference |
| --- | --- | --- |
| Fruity Balance | `Balance` | `fx-fruity-balance` |
| Fruity Center | `DcBlock` | `fx-fruity-center` |
| Fruity Mute 2 | `ChannelMute` | `fx-fruity-mute-2` |
| Fruity Phase Inverter | `Polarity` | `fx-fruity-phase-inverter` |
| Fruity Stereo Shaper | `StereoMatrix` | `fx-fruity-stereo-shaper` |
| Fruity Soft Clipper | `SoftClipper` | `fx-fruity-soft-clipper` |
| Fruity Fast Dist | `Distortion` | `fx-fruity-fast-dist` |
| Fruity Fast LP | `FastLowpass` | `fx-fruity-fast-lp` |
| Fruity Free Filter | `SelectableFilter` | `fx-fruity-free-filter` |
| Fruity Bass Boost | `BassShelf` | `fx-fruity-bass-boost` |
| Fruity Squeeze | `Lofi` | `fx-fruity-squeeze` |
| Fruity Chorus | `Chorus` | `fx-fruity-chorus` |
| Fruity Flanger | `Flanger` | `fx-fruity-flanger` |
| Fruity Phaser | `Phaser` | `fx-fruity-phaser` |
| Frequency Splitter | `BandSplit` | `fx-frequency-splitter` |
| Fruity Multiband Compressor | `MultibandCompressor` | `fx-fruity-multiband-compressor` |
| Maximus | `MultibandMaximizer` | `fx-maximus` |
| Transient Processor | `TransientShaper` | `fx-transient-processor` |
| Transmitter | `TransientSplit` | `fx-transmitter` |
| Soundgoodizer | `OneKnob` | `fx-soundgoodizer` |
| Low Lifter | `BassHarmonics` | `fx-low-lifter` |
| Fruity Waveshaper | `Waveshaper` | `fx-fruity-waveshaper` |
| Fruity Blood Overdrive | `Overdrive` | `fx-fruity-blood-overdrive` |
| Hardcore | `GuitarRack` | `fx-hardcore-11-guitar-fx` |
| Distructor | `DriveChain` | `fx-distructor` |
| Vintage Chorus | `VintageChorus` | `fx-vintage-chorus` |
| Hyper Chorus | `HyperChorus` | `fx-hyper-chorus` |
| Vintage Phaser | `VintagePhaser` | `fx-vintage-phaser` |
| Fruity Flangus | `StackedFlanger` | `fx-fruity-flangus` |
| Multiband Delay | `BandDelay` | `fx-multiband-delay` |
| Spreader | `Spreader` | `fx-spreader` |
| Fruity Stereo Enhancer | `StereoEnhancer` | `fx-fruity-stereo-enhancer` |
| Gross Beat | `VolumeGate` | `fx-gross-beat` |
| Transporter | `TimeTransport` | `fx-transporter` |
| Fruity Scratcher | `Scratch` | `fx-fruity-scratcher` |
| Effector | `PerformanceRack` | `fx-effector-12-fx` |
| Fruity Convolver | `Convolver` | `fx-fruity-convolver` |
| Frequency Shifter | `FrequencyShifter` | `fx-frequency-shifter` |
| Pitch Shifter | `PitchShift` | `fx-pitch-shifter` |
| Pitcher | `PitchCorrect` | `fx-pitcher` |
| Fruity Vocoder | `Vocoder` | `fx-fruity-vocoder` |
| Vocodex | `Vocoder` | `fx-vocodex` |

The family mappings follow [multiband.md](multiband.md), [drive.md](drive.md),
[spatial.md](spatial.md), [performance.md](performance.md) and
[spectral.md](spectral.md). Earlier processors are documented in
[UTILITY-EFFECTS.md](../../UTILITY-EFFECTS.md),
[FILTER-FAMILY.md](../../FILTER-FAMILY.md) and
[MODULATION-EFFECTS.md](../../MODULATION-EFFECTS.md).

These are processor substitutions with the limits stated in those documents,
not settings or acoustic parity. For example, Multiband Delay is the compact
three-band alternative; Gross Beat imports the volume/retrigger subset;
Convolver receives no saved impulse; and both vocoder names start with the
same default bank rather than inferring their source settings. `Exciter`
has no source parity id in the multiband seam, so it gets no import key.

## Deliberately omitted source names

The following 29 names from the effects parity inventory have no registered
kind covering their documented processor or non-audio role:

| Source import keys left out of the effect chain |
| --- |
| Fruity Delay Bank |
| Fruity Filter |
| Fruity 7 Band EQ |
| EQUO |
| Fruity Love Philter |
| LuxeVerb |
| Emphasis |
| Emphasizer |
| Tuner |
| Control Surface |
| Fruity Formula Controller |
| Fruity PanOMatic |
| Fruity Peak Controller |
| Fruity X-Y Controller |
| Fruity X-Y-Z Controller |
| Fruity Send |
| Fruity LSD |
| Patcher |
| Fruity HTML NoteBook |
| Fruity NoteBook |
| Fruity NoteBook 2 |
| Razer Chroma |
| VFX Color Mapper |
| VFX Envelope |
| VFX Level Scaler |
| VFX Keyboard Splitter |
| VFX Key Mapper |
| VFX Sequencer |
| VFX Script |

Other unknown names follow the same unchanged path: `translate` returns
`None`; the mixer omits the audio effect and keeps its exact stored state,
internal name and track/slot location in `Conversion::plugins`. The report
counts this as `Placeholder`, the existing outcome for preserved plugin data,
and says the effect was left out of the chain and its settings were kept.
Fruity Delay Bank is not mapped to a single Delay: its seam calls for a bank,
and that bank is not a registered `EffectKind`.

## Verification

The scoped tests check every source row against an expected kind with empty
state, sanitized defaults and settings-loss notes for all added rows with
empty, arbitrary and truncated state, case/whitespace matching, real import
report propagation, and preserved state/location for every omitted name
plus a fictional unknown effect. Existing decoder tests remain in the same
test group.

Run from Git Bash:

```bash
source scripts/msvc-env.sh
rustfmt --edition 2024 crates/windfall-flp/src/convert/effects.rs
cargo test -p windfall-flp --lib convert::effects
```

Only the importer file and this seam document are owned by this change.

Verified with the commands above: rustfmt completed, and all 16 selected
tests passed (0 failed; 64 filtered out). The importer diff also passes
`git diff --check`.

## Newly registered default mappings

These mappings supersede their entries in the omitted-name list above:

| Source import key | EffectKind |
| --- | --- |
| Fruity Delay Bank | `EchoBank` |
| Fruity 7 Band EQ | `SevenBand` |
| EQUO | `MorphEq` |
| Fruity Love Philter | `FilterBank` |
| LuxeVerb | `LushSpace` |
| Emphasis, Emphasizer | `StageStack` |
| Tuner | `Tuner` |
| Control Surface | `ControlSurface` |
| Fruity PanOMatic | `PanLfo` |
| Fruity Peak Controller | `EnvelopeFollower` |
| Fruity X-Y Controller | `XyPad` |
| Fruity X-Y-Z Controller | `XyzPad` |
| Fruity Send | `SendTap` |

Each uses `kind.default_params().sanitized()`. Settings are still not decoded,
and imports retain the existing settings-loss report. Fruity Formula Controller
remains unsupported because it is not an audio effect; Fruity Filter is not
the filter bank. Fruity LSD, Patcher, notebook names, Razer Chroma and every
VFX name remain unsupported.
