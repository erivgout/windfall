# Stretch validation and integration notes

Measured on 2026-10-07, Windows 11, Intel Core i9-14900F, Rust 1.99.0,
48 kHz. This machine was shared with other repository verification. No
listening evaluation was possible. The figures describe synthetic fixtures,
not a guarantee of transparent sound or callback deadlines on all music/devices.

## Reproduction

In this worktree, source `scripts/msvc-env.sh` before each Cargo session:

```bash
cargo test -p windfall-stretch
cargo test -p windfall-stretch --release
cargo clippy -p windfall-stretch --all-targets -- -D warnings
cargo fmt -p windfall-stretch -- --check
cargo build -p windfall-stretch --target wasm32-unknown-unknown
cargo test -p windfall-stretch --release -- --ignored --nocapture realtime_factors
cargo test -p windfall-stretch --release -- --ignored --nocapture quality_figures
WINDFALL_STRETCH_RENDER_DIR="$TEMP/windfall-stretch-review" cargo test -p windfall-stretch --release -- --ignored render_examples
python crates/windfall-stretch/tools/inspect_examples.py "$TEMP/windfall-stretch-review"
```

`inspect_examples.py` uses Python 3.12, NumPy 2.5.3, SciPy 1.18.1 and
Matplotlib 3.11.2. WAVs, CSV statistics and spectrograms are temporary artifacts
outside the repository. FFmpeg `showspectrumpic` was also used for the song sweep.
The inspection verified 273 WAVs: finite samples and exact transformed lengths.
The independently measured 440 Hz examples, including ratio 4 and +/-12 shifts,
had worst frequency error **0.02589 cent** and RMS change **0.073 dB**.

## Final checks

Debug and release suites each pass 7 unit tests and 47 integration tests.
Four ignored tests are the two measurement runs, an exploratory probe, and the
WAV renderer; measurements and renderer were run separately and passed.
Clippy with `-D warnings`, formatting check and wasm32 build also pass.

### Debug

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 47 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 54.35s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Release

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 47 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 52.64s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Signal measurements

Streaming checks cover sines at 110, 440, 1000 and 3520 Hz, a five-note chord
at 440/554.37/659.26/880/1108.73 Hz, and time ratios 0.5 through 2.
All stationary partial amplitude assertions are 0.5 dB; no cheaper-preset
exceptions remain. Frequency assertions are 0.1 cent for sines and 0.25 cent
for the chord. Pitch checks include +/-12 semitones with unchanged duration.
Offline lengths are `round(frames * clamped_ratio)`, including tiny buffers.

| Measurement | Fast | Standard | High |
|---|---:|---:|---:|
| Window / hop, ms | 100 / 15 | 120 / 15 | 120 / 10 |
| Input + output latency, frames | 2400 + 2400 | 2880 + 2880 | 2880 + 2880 |
| Sine time-stretch worst level change, ratios 0.5â€“2 | -0.010 dB | -0.010 dB | -0.010 dB |
| Sine pitch/combined worst level change in figure sweep | -0.205 dB | -0.178 dB | -0.178 dB |
| Sine worst frequency error in figure sweep (rounded) | 0.0001 cent | 0.0002 cent | 0.0002 cent |
| Unity noise maximum absolute sample error | 4.7684e-7 | 4.4703e-7 | 3.2783e-7 |
| Click pre-echo, worst at ratios 0.75â€“1.5, before -2 ms | -29.4 dB | -29.7 dB | -28.7 dB |
| Click energy in -2 to +10 ms, minimum | 99.89% | 99.89% | 99.87% |
| Largest click peak displacement in that sweep | 9 frames | 13 frames | 19 frames |
| White noise level range, ratios 0.75â€“1.5 | -1.25 to -0.83 dB | -1.27 to -0.86 dB | -1.31 to -0.86 dB |
| White noise third-octave ripple, worst | 0.19 dB | 0.21 dB | 0.19 dB |
| White noise spectral-flatness range (input 0.558) | 0.433â€“0.539 | 0.439â€“0.532 | 0.440â€“0.529 |

Stereo fixture checks bound balance error to 0.25 dB and inter-channel phase
error to 0.02 radians, including simultaneous stretch and pitch changes.
Identical mono channels remain within 1e-4. No offset is added above the
0.004 * max(RMS, 0.1) test bound on noise, squares and drums.
Silence, DC, squares, full-scale impulses, +24 dB noise, subnormals, NaNs,
infinities and maximum floats produce finite, bounded streaming output.
Subnormal input becomes exact silence. These checks include optional formants.

Repeated ratio/pitch sweeps have adjacent-sample steps below 1.5 times the
highest test sine's normal step, with 10 ms RMS windows above -3 dB and below
+1.5 dB. Large parameter jumps stay above -6 dB and below +2 dB. These bounds
allow audible level variation. A dedicated near-EOF automation regression
caught a 0.484 sample jump: the silence gate now follows the smoothed clock,
keeping that fixture's largest step below 0.08 across all presets.

Streaming output is bit-identical for different host block partitions and
repeat runs on the same build. Counting-allocator coverage includes first
processing, parameter updates, reset, seek and flush and observes **zero**
allocation, reallocation or free calls. Construction, prepare and drop are
worker-thread operations. FFT/STFT tests independently check transforms and
window normalization. The processor contains no lock or blocking operations.

Unity seek priming matches continuous playback within 3e-5 samples. At changed
ratios/pitches, regenerated phases are not sample-identical: checks bound RMS
to 0.5 dB, partial levels to 1 dB, and onset displacement to 48 frames (1 ms).
Seek priming has finite memory; replaying automation history is a host concern.

## CPU measurements

Factors are seconds of output per elapsed second, using 256-frame blocks,
20 seconds of chord plus noise per run and the best of three runs. Block
maxima are the minimum of those three maxima; seek figures are best of five.
No allocation occurs inside the measured processing loop. These are wall-clock
measurements, so Windows scheduling and concurrent tasks materially affect them.

The less contended rerun measured stereo ratio 1.25 with +3 semitones:

| Preset | Factor | Mean share of one core | Largest block, ms | Seek, ms | Factor with formants | Block with formants, ms | Seek with formants, ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| Fast | 67x | 1.49% | 0.409 | 1.743 | 47x | 0.505 | 2.451 |
| Standard | 57x | 1.75% | 0.377 | 2.311 | 41x | 0.536 | 3.252 |
| High | 39x | 2.56% | 0.476 | 3.084 | 27x | 1.140 | 4.660 |

Mono factors for the same combined setting were 101x, 68x and 35x, respectively;
with formants 63x, 41x and 28x. An earlier contended pass measured stereo
24x/31x/14x for the combined setting, with block spikes up to 10.616 ms.
Other settings in the rerun still had multi-millisecond scheduling outliers.
The 256-frame callback period is 5.333 ms: average throughput is ample, but
these measurements do **not** prove callback deadline safety. Spectra are
processed in bursts rather than computation being split over a hop. The gate
clock correction changes only a reciprocal on the per-sample path; these CPU
figures were captured immediately before that near-EOF correction.

## Known sound limits

- Close/dense partials interfere. A two-tone 20 Hz gap near 1 kHz at ratio 0.75
  measured up to 8.69 cents of error; isolated chord results cannot be generalized
  to dense low voicings. High improves update density rather than frequency resolution.
- Stretched attacks gain energy: approximately +1.38 dB at ratio 1.5 and +1.98 dB
  at ratio 2. Click energy at ratio 0.5 falls approximately 4 dB.
- Noise can sound phased and loses energy: about -3 dB with +7 semitones, and
  flatness falls from 0.558 to about 0.38 at ratio 2 or shifted noise.
- The transformed path attenuates sub-bass below 20 Hz; unity bypasses the rolloff.
  Pitch changes glide over 500 ms, ratio changes over 50 ms. Formant preservation
  uses an approximate voiced-spectrum envelope with bounded gain, not vocal tracking.
- Large stretches smear attacks. Visual review of source/changed sine, low-chord,
  click and noise spectra showed straight isolated tone tracks and localized
  clicks at modest ratios; the Fast low chord at ratio 4 shows stronger modulation.
  The song sweep's harmonic tracks move smoothly and falls silent when its input
  is exhausted. There was no listening test and no promise of mastering quality.
- Residual spectral ringing after input silence is truncated by an 8 ms gate.
  New arbitrary seek phases need a host crossfade at nonunity. Tempo estimation
  has half/double-beat ambiguity and correlation scores are not probabilities.

## Dependencies and licenses

The crate is GPL-3.0-or-later. Direct dependencies are `windfall-core` (same
license) and `rustfft 6.4.1` (MIT OR Apache-2.0). RustFFT's resolved runtime
closure is `num-complex 0.4.6`, `num-integer 0.1.47`, `num-traits 0.2.19`,
`primal-check 0.3.4`, `transpose 0.2.3` and `strength_reduce 0.2.4`, each
MIT OR Apache-2.0; build helper `autocfg 1.5.1` is Apache-2.0 OR MIT.
Package license declarations were checked locally. The MIT option is compatible
with GPL-3.0 and its notices are retained in `LICENSE-THIRD-PARTY`, alongside
Signalsmith Stretch and Signalsmith Linear's MIT notices. There is no C/C++
FFI or non-Rust runtime dependency, and the wasm32 build passes.

## Public API and integration proposal

```rust
Stretcher::new(channels: usize, sample_rate: u32) -> Stretcher
Stretcher::with_quality(channels: usize, sample_rate: u32, quality: Quality) -> Stretcher
prepare(&mut self, channels: usize, sample_rate: u32, quality: Quality)
set_time_ratio(&mut self, ratio: f64)
set_pitch_semitones(&mut self, semitones: f64)
set_formant_preservation(&mut self, preserve: bool)
input_frames_needed(&self, output_frames: usize) -> usize
process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) -> usize
latency(&self) -> Latency { input: usize, output: usize }
reset(&mut self)
seek_frames(&self) -> usize
seek(&mut self, pre_roll: &[&[f32]])
flush(&mut self, output: &mut [&mut [f32]])
stretch(buffer: &AudioBuffer, time_ratio: f64, pitch_semitones: f64, quality: Quality) -> AudioBuffer
stretched_frames(frames: usize, time_ratio: f64) -> usize
tempo_ratio(original_bpm: f64, target_bpm: f64) -> Option<f64>
beat_length_ratio(frames: usize, sample_rate: u32, beats: f64, target_bpm: f64) -> Option<f64>
estimate_tempo(buffer: &AudioBuffer) -> Option<Vec<TempoCandidate>>
```

Read-only getters expose format, quality, target controls and formant flag;
`Quality::ALL`, `block_seconds`, `interval_seconds` expose the presets.
`Latency::input_frames(ratio)` and `output_frames(ratio)` compute compensation.
`TempoCandidate` exposes BPM and correlation confidence. Ratios returned by
fitting helpers are unclamped, so the host should reject unsupported fits
before configuring a stretcher.

Add a backward-compatible audio-clip mode such as `stretch: None |
{ ratio, preserve_pitch, quality, preserve_formants }`. Keep existing tape-style
pitch playback as the default; an explicit spectral mode treats pitch as
independent semitones. A known loop beat count or original BPM derives the ratio
for fit-to-project-tempo. Avoid silently changing current clip timing semantics.

In `clips.rs`, retain one preallocated processor per playing stretched clip and
planar input/output scratch per slot. Prepare sample-rate/channel/quality changes
on the control thread, transfer ownership through the engine's existing queue,
and retire/drop processors through its garbage mechanism. Use returned input
consumption to advance the source, and prime at the offset plus input-frame
latency before hearing that source position. Apply output latency to clip
scheduling and mixer delay compensation. Worker-side seek priming and a short
host crossfade cover rebuilt phase. Render offline with the same steady settings.

Budget roughly **2% of one core per stereo Standard clip**, **3% for High**, and
**4% with High formants**, before graph/mixing overhead, on this test CPU; reserve
additional headroom and verify actual callback peaks. Stagger spectral hops or
cache offline renders for high polyphony, since aligned bursts and seek priming
can exceed the callback budget despite good averages. No engine, model or UI
wiring is part of this crate change.
