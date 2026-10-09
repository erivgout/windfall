# FLP insert EQ and stereo import seam

`crates/windfall-flp/src/convert/mixer.rs` imports insert processing through
`MixerTrackPatch.processing` into the existing `MixerTrack.processing`
(`windfall_dsp::TrackParams`). This applies to the master and regular inserts
and consumes no effect slots. An otherwise unused insert with these controls
set is retained by the mixer's used-insert selection.

## Stereo controls

- `Insert::polarity_reversed()` is one insert switch. It sets both
  `invert_left` and `invert_right`.
- `Insert::channels_swapped()` sets `swap`.
- `stereo_separation` uses `(raw / 64.0).clamp(-1.0, 1.0)`: source `-64`
  becomes `-1`, `0` becomes `0`, and `+64` becomes `+1`. Intermediate values
  map linearly. In `TrackParams`, `-1` doubles the side signal, `0` preserves
  stereo, and `+1` sums toward mono. The repository documents the source's
  numeric range but does not document its sign's listening direction. That
  direction remains unverified, and nonzero separation produces an
  `Approximated` report line explaining the chosen map.

## Three-band EQ

The three `InsertEqBand` values map in order to the low shelf, mid bell and
high shelf. Gains are hundredths of a dB: `gain_db = raw / 100.0`, clamped to
each track band's `-24..24` dB range. Thus the documented source range
`-1800..1800` maps to `-18..18` dB, and `625` becomes `6.25` dB.

When any band has nonzero gain, `eq_enabled` is enabled, bands with nonzero
imported gain are enabled, and bands with zero or absent gain are disabled.
An insert with absent or zero gains otherwise keeps `TrackParams` defaults,
including its default enabled switches. Zero-gain bands leave audio unchanged.

Frequency and width are still dropped. A repository search found two
plugin-specific frequency curves in `convert/effects.rs`: Fruity Parametric EQ
uses `10 * 1600^(position^0.6)`, while Fruity Parametric EQ 2 uses
`20 * 1000^position`, with `position` derived from `raw / 65536`. That module
also estimates plugin width differently for bells and shelves. None of these
rules establishes the built-in insert EQ's frequency or width mapping, despite
the shared numeric scale. No insert frequency curve or width-to-Q rule is
inferred from them.

Frequency and Q remain at the track defaults: low `100 Hz` and
`1/sqrt(2)`, mid `1000 Hz` and `1`, high `8000 Hz` and `1/sqrt(2)`.
A nonzero EQ gain, or a stored frequency or width, produces an `Approximated`
report line saying frequency and width were not decoded and default frequencies
and Q were kept. The stale report claiming tracks have no polarity inversion,
channel swap, separation or three-band EQ is removed.

An insert with none of these settings keeps default processing and adds no
processing report line. Routing, faders, sends and effect-slot import continue
through their existing paths. This seam imports saved insert values; it adds
no insert-processing automation conversion or new DSP behavior.

## Validation

The `convert::mixer` library tests cover polarity on both channels, swap,
separation endpoints, intermediate values and clamping, low/mid/high dB gains,
gain clamping and band enables, neutral defaults with no report line, undecoded
EQ shape reporting, absence of the old unsupported sentence, and master import.

Run from Git Bash:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-flp --lib convert::mixer
```
