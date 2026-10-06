#!/usr/bin/env bash
# Regenerates the audio fixtures in this folder with ffmpeg.
#
#   bash crates/windfall-codec/tests/fixtures/generate.sh
#
# Every fixture is a sine tone made here from a formula, so the files are ours
# to release as CC0. The tones are 0.5 of full scale and start at phase zero,
# which is what tests/it/common.rs rebuilds to compare against. Stereo files
# carry 440 Hz on the left and 1000 Hz on the right so that swapped channels
# show up. The six-channel file carries 300, 400 ... 800 Hz in channel order.
#
# The tests run against the committed files, not against a fresh run of this
# script: encoders differ between ffmpeg versions.
set -euo pipefail
cd "$(dirname "$0")"

run() {
  ffmpeg -hide_banner -loglevel error -y "$@"
}

# tone <rate> <seconds> <frequency>...   prints an ffmpeg source, one frequency per channel
tone() {
  local rate="$1" seconds="$2" exprs="" frequency
  shift 2
  for frequency in "$@"; do
    exprs+="${exprs:+|}0.5*sin(2*PI*${frequency}*t)"
  done
  printf 'aevalsrc=%s:s=%s:d=%s' "$exprs" "$rate" "$seconds"
}

plain=(-map_metadata -1 -fflags +bitexact -flags:a +bitexact)

# WAV in every sample format the decoder has to handle.
run -f lavfi -i "$(tone 22050 0.1 440)"      "${plain[@]}" -c:a pcm_u8    wav_u8_22k_mono.wav
run -f lavfi -i "$(tone 44100 0.1 440 1000)" "${plain[@]}" -c:a pcm_s16le wav_s16_44k_stereo.wav
run -f lavfi -i "$(tone 48000 0.1 440 1000)" "${plain[@]}" -c:a pcm_s24le wav_s24_48k_stereo.wav
run -f lavfi -i "$(tone 44100 0.1 440 1000)" "${plain[@]}" -c:a pcm_s32le wav_s32_44k_stereo.wav
run -f lavfi -i "$(tone 48000 0.1 440 1000)" "${plain[@]}" -c:a pcm_f32le wav_f32_48k_stereo.wav
run -f lavfi -i "$(tone 44100 0.1 440)"      "${plain[@]}" -c:a pcm_f64le wav_f64_44k_mono.wav

# WAV with unusual headers: six channels in an extensible header, a LIST chunk
# ahead of the audio, and the unknown-length header ffmpeg writes to a pipe.
run -f lavfi -i "$(tone 48000 0.1 300 400 500 600 700 800)" "${plain[@]}" \
  -af "aformat=channel_layouts=5.1" -c:a pcm_s16le wav_s16_48k_surround.wav
run -f lavfi -i "$(tone 44100 0.1 440 1000)" -metadata title="Windfall test tone" \
  -metadata comment="Self-generated sine, CC0" -c:a pcm_s16le wav_with_list_chunk.wav
run -f lavfi -i "$(tone 44100 0.1 440 1000)" "${plain[@]}" -c:a pcm_s16le -f wav pipe:1 \
  > wav_unknown_length.wav

# AIFF (big-endian) and AIFF-C holding little-endian samples.
run -f lavfi -i "$(tone 44100 0.1 440 1000)" "${plain[@]}" -c:a pcm_s16be aiff_s16_44k_stereo.aiff
run -f lavfi -i "$(tone 48000 0.1 440 1000)" "${plain[@]}" -c:a pcm_s24be aiff_s24_48k_stereo.aiff
run -f lavfi -i "$(tone 44100 0.1 440 1000)" "${plain[@]}" -c:a pcm_s16le -f aiff aifc_sowt_44k_stereo.aifc

# FLAC.
run -f lavfi -i "$(tone 44100 0.25 440 1000)" "${plain[@]}" -c:a flac -sample_fmt s16 flac_s16_44k_stereo.flac
run -f lavfi -i "$(tone 48000 0.25 440)"      "${plain[@]}" -c:a flac -sample_fmt s32 \
  -bits_per_raw_sample 24 flac_s24_48k_mono.flac

# MP3: MPEG-1 stereo and MPEG-2 mono.
run -f lavfi -i "$(tone 44100 0.25 440 1000)" "${plain[@]}" -c:a libmp3lame -b:a 128k mp3_44k_stereo.mp3
run -f lavfi -i "$(tone 22050 0.25 440)"      "${plain[@]}" -c:a libmp3lame -b:a 64k  mp3_22k_mono.mp3

# Ogg Vorbis, in 20 ms pages so that even these short files span several.
run -f lavfi -i "$(tone 44100 0.25 440 1000)" "${plain[@]}" -c:a libvorbis -q:a 5   -page_duration 20000 vorbis_44k_stereo.ogg
run -f lavfi -i "$(tone 48000 0.25 440)"      "${plain[@]}" -c:a libvorbis -q:a 5   -page_duration 20000 vorbis_48k_mono.ogg

ls -l
