# Codec test fixtures

Short sine tones in each format the decoder reads. `generate.sh` made all of
them with ffmpeg from a formula. They contain nothing recorded, sampled or
copied from anywhere.

The files in this folder are released under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/): no rights
reserved.

| File | What it covers |
|---|---|
| `wav_u8_22k_mono.wav` | 8-bit unsigned PCM, 22.05 kHz, mono |
| `wav_s16_44k_stereo.wav` | 16-bit PCM |
| `wav_s24_48k_stereo.wav` | 24-bit PCM in an extensible header |
| `wav_s32_44k_stereo.wav` | 32-bit integer PCM |
| `wav_f32_48k_stereo.wav` | 32-bit float |
| `wav_f64_44k_mono.wav` | 64-bit float |
| `wav_s16_48k_surround.wav` | six channels, a different tone on each |
| `wav_with_list_chunk.wav` | a `LIST` chunk between the format and the audio |
| `wav_unknown_length.wav` | the `0xFFFFFFFF` lengths of a WAV written to a pipe |
| `aiff_s16_44k_stereo.aiff`, `aiff_s24_48k_stereo.aiff` | big-endian AIFF |
| `aifc_sowt_44k_stereo.aifc` | AIFF-C holding little-endian samples |
| `flac_s16_44k_stereo.flac`, `flac_s24_48k_mono.flac` | FLAC at 16 and 24 bits |
| `mp3_44k_stereo.mp3`, `mp3_22k_mono.mp3` | MPEG-1 and MPEG-2 Layer III, with encoder delay to trim |
| `vorbis_44k_stereo.ogg`, `vorbis_48k_mono.ogg` | Ogg Vorbis in many small pages |

Every tone peaks at 0.5 and starts at phase zero. Mono files carry 440 Hz.
Stereo files carry 440 Hz on the left and 1000 Hz on the right. The
six-channel file carries 300, 400, 500, 600, 700 and 800 Hz in channel order.
`tests/it/fixtures.rs` holds the expected sample rate, length and codec of
each file and rebuilds the tones to compare against.

To regenerate, run `bash crates/windfall-codec/tests/fixtures/generate.sh`
with ffmpeg on the `PATH`. Encoders change between ffmpeg versions, so the
tests run against the committed files and a regenerated lossy file may need
its expected length updated.
