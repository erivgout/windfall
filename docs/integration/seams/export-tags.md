# Windfall audio export tags

The desktop export job passes the current project settings snapshot to
`Encoder::open_with_tags` for the mix and each stem. `ProjectSettings.name` is
the title; author, genre, and comments use their existing settings fields.
The project info panel remains the editor for these values.

WAV writes a `LIST/INFO` chunk before `data`, mapping title to `INAM`, author
to `IART`, genre to `IGNR`, and comments to `ICMT`. Values are UTF-8 bytes with
a terminating NUL. Each INFO subchunk is padded to an even length; its size
excludes the padding. The RIFF size includes the metadata and padding, and the
writer's audio-size limit accounts for that space.

FLAC uses the existing final Vorbis comment metadata block, mapping title to
`TITLE`, author to `ARTIST`, genre to `GENRE`, and comments to `DESCRIPTION`.
Values use UTF-8, and the encoder vendor string remains `Windfall`.

Empty fields are omitted. With all four fields empty, WAV omits the entire
LIST chunk and keeps its existing layout; FLAC keeps its vendor-only comment
block. Empty metadata does not prevent audio export. The existing untagged
writer and encoder entry points retain their behavior. MP3 writes an ID3v2.4
tag before the audio, using UTF-8 `TIT2` for title, `TPE1` for author, `TCON`
for genre, and `COMM` for comments with language `eng` and an empty content
description; empty fields are omitted, all-empty tags write no ID3, and the
LAME gapless header remains at the first audio byte after the tag.
Ogg Vorbis writes comment keys `TITLE` for title, `ARTIST` for author, `GENRE`
for genre, and `DESCRIPTION` for comments; empty fields are omitted, all four
fields empty produce identical bytes to the untagged writer, and the libvorbis
vendor string is left alone.

Sample encoding, dither, and the temporary-file handover are unchanged.

Codec tests in `tests/it/export_tags.rs` check an author-only WAV and identical
sample bytes at every WAV bit depth, the no-tags WAV layout, all four WAV tags
and padding, a title-only FLAC, and all four FLAC comments with identical audio
frames and STREAMINFO. The encoder tests exercise tag forwarding too.

Run in Git Bash from the repository root:

```bash
source scripts/msvc-env.sh
TS_RS_EXPORT_DIR=target/ts-rs-discard cargo test -p windfall-codec --quiet
```
