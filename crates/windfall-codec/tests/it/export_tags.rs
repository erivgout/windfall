//! Project tags must change metadata without changing the encoded audio.

use std::fs;

use windfall_codec::{
    AudioTags, Encoder, EncoderSettings, FlacBitDepth, FlacWriter, Mp3Rate, Mp3Settings, Mp3Writer,
    VorbisWriter, WavSampleFormat, WavWriter, decode_file,
};

use crate::common::*;

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn riff_chunks(bytes: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut chunks = Vec::new();
    let mut at = 12;
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(u32_at(bytes, 4) as usize, bytes.len() - 8);
    while at < bytes.len() {
        let size = u32_at(bytes, at + 4) as usize;
        chunks.push((&bytes[at..at + 4], &bytes[at + 8..at + 8 + size]));
        at += 8 + size + size % 2;
    }
    assert_eq!(at, bytes.len());
    chunks
}

fn info_fields(list: &[u8]) -> Vec<(&[u8], &str)> {
    assert_eq!(&list[..4], b"INFO");
    let mut fields = Vec::new();
    let mut at = 4;
    while at < list.len() {
        let size = u32_at(list, at + 4) as usize;
        assert_eq!(list[at + 8 + size - 1], 0);
        fields.push((
            &list[at..at + 4],
            std::str::from_utf8(&list[at + 8..at + 8 + size - 1]).unwrap(),
        ));
        if !size.is_multiple_of(2) {
            assert_eq!(list[at + 8 + size], 0);
        }
        at += 8 + size + size % 2;
    }
    assert_eq!(at, list.len());
    fields
}

#[test]
fn wav_author_is_iart_and_samples_are_unchanged() {
    let dir = TempDir::new("wav-author");
    let samples = [0.0, 0.25, -0.5, 0.75, -1.0];
    for format in [
        WavSampleFormat::Int16,
        WavSampleFormat::Int24,
        WavSampleFormat::Float32,
    ] {
        let plain = dir.path("plain.wav");
        let tagged = dir.path("tagged.wav");
        let mut writer = WavWriter::create(&plain, 48_000, 1, format).unwrap();
        writer.write(&samples).unwrap();
        writer.finalize().unwrap();
        let tags = AudioTags {
            author: "Zoë",
            ..AudioTags::default()
        };
        let mut writer = WavWriter::create_with_tags(&tagged, 48_000, 1, format, &tags).unwrap();
        writer.write(&samples[..2]).unwrap();
        writer.write(&samples[2..]).unwrap();
        writer.finalize().unwrap();

        let plain_bytes = fs::read(&plain).unwrap();
        let tagged_bytes = fs::read(&tagged).unwrap();
        let plain_chunks = riff_chunks(&plain_bytes);
        let tagged_chunks = riff_chunks(&tagged_bytes);
        let list = tagged_chunks
            .iter()
            .find(|(id, _)| *id == b"LIST")
            .unwrap()
            .1;
        assert_eq!(info_fields(list), vec![(b"IART".as_slice(), "Zoë")]);
        let without_list: Vec<_> = tagged_chunks
            .into_iter()
            .filter(|(id, _)| *id != b"LIST")
            .collect();
        assert_eq!(without_list, plain_chunks);
        assert_eq!(
            decode_file(&tagged).unwrap().samples(),
            decode_file(&plain).unwrap().samples()
        );
    }
}

#[test]
fn wav_empty_tags_omit_list_and_keep_the_original_layout() {
    let dir = TempDir::new("wav-no-tags");
    let plain = dir.path("plain.wav");
    let tagged = dir.path("empty-tags.wav");
    let mut writer = WavWriter::create(&plain, 48_000, 1, WavSampleFormat::Int16).unwrap();
    writer.write(&[0.0; 3]).unwrap();
    writer.finalize().unwrap();
    let mut encoder = Encoder::open_with_tags(
        &tagged,
        &EncoderSettings::Wav {
            format: WavSampleFormat::Int16,
        },
        48_000,
        1,
        &AudioTags::default(),
    )
    .unwrap();
    encoder.write(&[0.0; 3]).unwrap();
    encoder.finalize().unwrap();
    let bytes = fs::read(&tagged).unwrap();
    assert_eq!(bytes, fs::read(&plain).unwrap());
    assert_eq!(bytes.len(), 44 + 6);
    assert!(riff_chunks(&bytes).iter().all(|(id, _)| *id != b"LIST"));
}

#[test]
fn encoder_passes_all_wav_tags_with_correct_padding() {
    let dir = TempDir::new("wav-all-tags");
    let path = dir.path("tags.wav");
    let tags = AudioTags {
        title: "Title",
        author: "Windfall",
        genre: "É",
        comments: "Line 1\nLine 2",
    };
    let mut encoder = Encoder::open_with_tags(
        &path,
        &EncoderSettings::Wav {
            format: WavSampleFormat::Float32,
        },
        48_000,
        1,
        &tags,
    )
    .unwrap();
    encoder.write(&[0.25]).unwrap();
    encoder.finalize().unwrap();
    let bytes = fs::read(&path).unwrap();
    let chunks = riff_chunks(&bytes);
    let list = chunks.iter().find(|(id, _)| *id == b"LIST").unwrap().1;
    assert_eq!(
        info_fields(list),
        vec![
            (b"INAM".as_slice(), tags.title),
            (b"IART".as_slice(), tags.author),
            (b"IGNR".as_slice(), tags.genre),
            (b"ICMT".as_slice(), tags.comments),
        ]
    );
    assert_eq!(decode_file(&path).unwrap().samples(), &[0.25]);
}

/// Returns Vorbis comments and the audio frames, checking block lengths and vendor.
fn flac_comments(bytes: &[u8]) -> (Vec<&str>, &[u8]) {
    assert_eq!(&bytes[..4], b"fLaC");
    let mut at = 4;
    let mut comments = Vec::new();
    loop {
        let kind = bytes[at];
        let size = u32::from_be_bytes([0, bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
        let block = &bytes[at + 4..at + 4 + size];
        if kind & 0x7F == 4 {
            let vendor_size = u32_at(block, 0) as usize;
            assert_eq!(&block[4..4 + vendor_size], b"Windfall");
            let count = u32_at(block, 4 + vendor_size);
            let mut pos = 8 + vendor_size;
            for _ in 0..count {
                let len = u32_at(block, pos) as usize;
                comments.push(std::str::from_utf8(&block[pos + 4..pos + 4 + len]).unwrap());
                pos += 4 + len;
            }
            assert_eq!(pos, size);
        }
        at += 4 + size;
        if kind & 0x80 != 0 {
            return (comments, &bytes[at..]);
        }
    }
}

#[test]
fn flac_title_is_a_vorbis_comment_and_empty_fields_are_omitted() {
    let dir = TempDir::new("flac-title");
    let path = dir.path("title.flac");
    let tags = AudioTags {
        title: "Windfall étude",
        ..AudioTags::default()
    };
    let mut writer =
        FlacWriter::create_with_tags(&path, 48_000, 1, FlacBitDepth::Int16, 5, &tags).unwrap();
    writer.write(&[0.25; 10]).unwrap();
    writer.finalize().unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(flac_comments(&bytes).0, ["TITLE=Windfall étude"]);
    assert_eq!(decode_file(&path).unwrap().frames(), 10);
}

#[test]
fn encoder_passes_all_flac_tags_without_changing_audio() {
    let dir = TempDir::new("flac-all-tags");
    let tags = AudioTags {
        title: "Title",
        author: "Zoë",
        genre: "Electronic",
        comments: "Line 1\nLine 2",
    };
    let settings = EncoderSettings::Flac {
        depth: FlacBitDepth::Int24,
        level: 5,
    };
    let mut outputs = Vec::new();
    for (name, tags) in [("plain.flac", AudioTags::default()), ("tagged.flac", tags)] {
        let path = dir.path(name);
        let mut encoder = Encoder::open_with_tags(&path, &settings, 48_000, 1, &tags).unwrap();
        encoder.write(&[0.0, 0.25, -0.5]).unwrap();
        encoder.finalize().unwrap();
        outputs.push(fs::read(&path).unwrap());
    }
    let (plain_comments, plain_audio) = flac_comments(&outputs[0]);
    let (comments, tagged_audio) = flac_comments(&outputs[1]);
    assert!(plain_comments.is_empty());
    assert_eq!(
        comments,
        [
            "TITLE=Title",
            "ARTIST=Zoë",
            "GENRE=Electronic",
            "DESCRIPTION=Line 1\nLine 2"
        ]
    );
    assert_eq!(plain_audio, tagged_audio);
    // STREAMINFO, including the audio MD5, is identical.
    assert_eq!(outputs[0][..42], outputs[1][..42]);
    assert_eq!(
        decode_file(dir.path("plain.flac")).unwrap().samples(),
        decode_file(dir.path("tagged.flac")).unwrap().samples()
    );
}

fn synchsafe_at(bytes: &[u8], at: usize) -> usize {
    bytes[at..at + 4].iter().fold(0, |size, &byte| {
        assert_eq!(byte & 0x80, 0);
        (size << 7) | usize::from(byte)
    })
}

/// Checks ID3v2.4 frame structure and returns UTF-8 fields and the audio bytes.
fn id3_fields(bytes: &[u8]) -> (Vec<(&[u8], &str)>, &[u8]) {
    assert_eq!(&bytes[..6], b"ID3\x04\0\0");
    let end = 10 + synchsafe_at(bytes, 6);
    let mut fields = Vec::new();
    let mut at = 10;
    while at < end {
        let id = &bytes[at..at + 4];
        let size = synchsafe_at(bytes, at + 4);
        assert_eq!(&bytes[at + 8..at + 10], &[0, 0]);
        assert!(at + 10 + size <= end);
        let payload = &bytes[at + 10..at + 10 + size];
        assert_eq!(payload[0], 3, "text must use UTF-8");
        let text = if id == b"COMM" {
            assert_eq!(&payload[1..5], b"eng\0");
            &payload[5..]
        } else {
            &payload[1..]
        };
        fields.push((id, std::str::from_utf8(text).unwrap()));
        at += 10 + size;
    }
    assert_eq!(at, end);
    (fields, &bytes[end..])
}

/// An independent metadata reader must recognize the exported project fields.
fn read_mp3_tags(path: &std::path::Path) -> [String; 4] {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::{MetadataOptions, StandardTag};

    let stream =
        MediaSourceStream::new(Box::new(fs::File::open(path).unwrap()), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .unwrap();
    let metadata = format.metadata();
    let mut tags: [String; 4] = Default::default();
    for tag in &metadata.current().unwrap().media.tags {
        let (field, value) = match &tag.std {
            Some(StandardTag::TrackTitle(value)) => (&mut tags[0], value),
            Some(StandardTag::Artist(value)) => (&mut tags[1], value),
            Some(StandardTag::Genre(value)) => (&mut tags[2], value),
            Some(StandardTag::Comment(value)) => (&mut tags[3], value),
            _ => continue,
        };
        *field = value.as_ref().clone();
    }
    tags
}

#[test]
fn encoder_passes_all_mp3_tags_without_changing_audio_or_gapless_length() {
    let dir = TempDir::new("mp3-all-tags");
    let tags = AudioTags {
        title: "Title",
        author: "Windfall",
        genre: "Electronic",
        comments: "Line 1\nLine 2",
    };
    let audio = tones(48_000, &[440.0, 1000.0], 4097, 0.4);
    for rate in [Mp3Rate::Cbr(192), Mp3Rate::Vbr(2)] {
        let settings = Mp3Settings {
            rate,
            ..Mp3Settings::default()
        };
        let plain = dir.path("plain.mp3");
        let tagged = dir.path("tagged.mp3");
        let mut writer = Mp3Writer::create(&plain, 48_000, 2, settings).unwrap();
        writer.write(audio.samples()).unwrap();
        writer.finalize().unwrap();
        let mut encoder = Encoder::open_with_tags(
            &tagged,
            &EncoderSettings::Mp3 { settings },
            48_000,
            2,
            &tags,
        )
        .unwrap();
        for block in audio.samples().chunks(2 * 137) {
            encoder.write(block).unwrap();
        }
        encoder.finalize().unwrap();

        let bytes = fs::read(&tagged).unwrap();
        let (fields, tagged_audio) = id3_fields(&bytes);
        assert_eq!(
            fields,
            vec![
                (b"TIT2".as_slice(), tags.title),
                (b"TPE1".as_slice(), tags.author),
                (b"TCON".as_slice(), tags.genre),
                (b"COMM".as_slice(), tags.comments),
            ]
        );
        assert_eq!(
            read_mp3_tags(&tagged),
            [tags.title, tags.author, tags.genre, tags.comments]
        );
        // This includes the finalized LAME header at the start of each audio stream.
        assert_eq!(tagged_audio, fs::read(&plain).unwrap());
        let decoded = decode_file(&tagged).unwrap();
        assert_eq!(decoded.frames(), audio.frames());
        assert_eq!(decoded.samples(), decode_file(&plain).unwrap().samples());
    }
}

#[test]
fn mp3_author_only_omits_other_frames() {
    let dir = TempDir::new("mp3-author");
    let path = dir.path("author.mp3");
    // Exercise synchsafe sizes above 127 in both the frame and tag headers.
    let author = "Windfall".repeat(24);
    let tags = AudioTags {
        author: &author,
        ..AudioTags::default()
    };
    let mut writer =
        Mp3Writer::create_with_tags(&path, 48_000, 2, Mp3Settings::default(), &tags).unwrap();
    writer.write(&[0.25; 34]).unwrap();
    writer.finalize().unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        id3_fields(&bytes).0,
        vec![(b"TPE1".as_slice(), tags.author)]
    );
    assert_eq!(read_mp3_tags(&path), ["", tags.author, "", ""]);
}

#[test]
fn mp3_empty_tags_match_the_untagged_writer_byte_for_byte() {
    let dir = TempDir::new("mp3-no-tags");
    let plain = dir.path("plain.mp3");
    let tagged = dir.path("empty-tags.mp3");
    let settings = Mp3Settings::default();
    let samples = tones(48_000, &[440.0, 1000.0], 4097, 0.4);
    let mut writer = Mp3Writer::create(&plain, 48_000, 2, settings).unwrap();
    writer.write(samples.samples()).unwrap();
    writer.finalize().unwrap();
    let mut encoder = Encoder::open_with_tags(
        &tagged,
        &EncoderSettings::Mp3 { settings },
        48_000,
        2,
        &AudioTags::default(),
    )
    .unwrap();
    encoder.write(samples.samples()).unwrap();
    encoder.finalize().unwrap();
    let bytes = fs::read(&tagged).unwrap();
    assert!(!bytes.starts_with(b"ID3"));
    assert_eq!(bytes, fs::read(&plain).unwrap());
    assert_eq!(decode_file(&tagged).unwrap().frames(), samples.frames());
}

#[test]
fn mp3_non_ascii_title_round_trips_as_utf8() {
    let dir = TempDir::new("mp3-utf8");
    let path = dir.path("title.mp3");
    let tags = AudioTags {
        title: "Windfall étude",
        ..AudioTags::default()
    };
    let mut encoder = Encoder::open_with_tags(
        &path,
        &EncoderSettings::Mp3 {
            settings: Mp3Settings::default(),
        },
        48_000,
        2,
        &tags,
    )
    .unwrap();
    encoder.write(&[0.25; 34]).unwrap();
    encoder.finalize().unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(id3_fields(&bytes).0, vec![(b"TIT2".as_slice(), tags.title)]);
    assert_eq!(read_mp3_tags(&path), [tags.title, "", "", ""]);
}

/// Reassembles packets across pages and checks the fixed serial and header flush.
fn vorbis_packets(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    let mut packet = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let header = &bytes[at..at + 27];
        assert_eq!(&header[..5], b"OggS\0");
        assert_eq!(u32_at(header, 14), 0x5746_4C4C);
        assert_eq!(header[5] & 1 != 0, !packet.is_empty());
        let segments = usize::from(header[26]);
        let lacing = &bytes[at + 27..at + 27 + segments];
        let mut body = at + 27 + segments;
        for (index, &segment) in lacing.iter().enumerate() {
            let end = body + usize::from(segment);
            packet.extend_from_slice(&bytes[body..end]);
            body = end;
            if segment < 255 {
                packets.push(std::mem::take(&mut packet));
                if packets.len() == 3 {
                    assert_eq!(index + 1, segments, "audio must start on its own page");
                }
            }
        }
        at = body;
    }
    assert!(packet.is_empty());
    assert!(packets.len() > 3, "expected headers and audio packets");
    for (packet, kind) in packets.iter().zip([1, 3, 5]) {
        assert_eq!(packet[0], kind);
        assert_eq!(&packet[1..7], b"vorbis");
    }
    packets
}

/// Reads UTF-8 comments and the untouched encoder vendor from the comment header.
fn vorbis_comments(packet: &[u8]) -> (&str, Vec<&str>) {
    assert_eq!(&packet[..7], b"\x03vorbis");
    let vendor_size = u32_at(packet, 7) as usize;
    let vendor = std::str::from_utf8(&packet[11..11 + vendor_size]).unwrap();
    let count = u32_at(packet, 11 + vendor_size);
    let mut at = 15 + vendor_size;
    let mut comments = Vec::new();
    for _ in 0..count {
        let size = u32_at(packet, at) as usize;
        comments.push(std::str::from_utf8(&packet[at + 4..at + 4 + size]).unwrap());
        at += 4 + size;
    }
    assert_eq!(&packet[at..], &[1], "expected the comment framing bit");
    (vendor, comments)
}

#[test]
fn encoder_passes_all_vorbis_tags_including_a_non_ascii_title() {
    let dir = TempDir::new("vorbis-all-tags");
    let path = dir.path("tags.ogg");
    let tags = AudioTags {
        title: "Windfall étude",
        author: "Zoë",
        genre: "Electronic",
        comments: "Line 1\nLine 2",
    };
    let mut encoder = Encoder::open_with_tags(
        &path,
        &EncoderSettings::Vorbis { quality: 6.0 },
        48_000,
        2,
        &tags,
    )
    .unwrap();
    encoder.write(&[0.25; 34]).unwrap();
    encoder.finalize().unwrap();
    let packets = vorbis_packets(&fs::read(&path).unwrap());
    assert_eq!(
        vorbis_comments(&packets[1]).1,
        [
            "TITLE=Windfall étude",
            "ARTIST=Zoë",
            "GENRE=Electronic",
            "DESCRIPTION=Line 1\nLine 2",
        ]
    );
    assert_eq!(decode_file(&path).unwrap().frames(), 17);
}

#[test]
fn vorbis_author_only_omits_other_keys() {
    let dir = TempDir::new("vorbis-author");
    let path = dir.path("author.ogg");
    let tags = AudioTags {
        author: "Windfall",
        ..AudioTags::default()
    };
    let mut writer = VorbisWriter::create_with_tags(&path, 48_000, 1, 6.0, &tags).unwrap();
    writer.write(&[0.25; 17]).unwrap();
    writer.finalize().unwrap();
    let packets = vorbis_packets(&fs::read(&path).unwrap());
    assert_eq!(vorbis_comments(&packets[1]).1, ["ARTIST=Windfall"]);
}

#[test]
fn vorbis_empty_tags_match_the_untagged_writer_byte_for_byte() {
    let dir = TempDir::new("vorbis-no-tags");
    let plain = dir.path("plain.ogg");
    let tagged = dir.path("empty-tags.ogg");
    let audio = tones(48_000, &[440.0, 1000.0], 4097, 0.4);
    let mut writer = VorbisWriter::create(&plain, 48_000, 2, 6.0).unwrap();
    writer.write(audio.samples()).unwrap();
    writer.finalize().unwrap();
    let mut encoder = Encoder::open_with_tags(
        &tagged,
        &EncoderSettings::Vorbis { quality: 6.0 },
        48_000,
        2,
        &AudioTags::default(),
    )
    .unwrap();
    encoder.write(audio.samples()).unwrap();
    encoder.finalize().unwrap();
    let bytes = fs::read(&tagged).unwrap();
    assert_eq!(bytes, fs::read(&plain).unwrap());
    let packets = vorbis_packets(&bytes);
    assert!(vorbis_comments(&packets[1]).1.is_empty());
    assert_eq!(decode_file(&tagged).unwrap().frames(), audio.frames());
}

#[test]
fn vorbis_tags_leave_audio_packets_and_vendor_unchanged() {
    let dir = TempDir::new("vorbis-tagged-audio");
    let plain = dir.path("plain.ogg");
    let tagged = dir.path("tagged.ogg");
    let audio = tones(48_000, &[440.0, 1000.0], 16_385, 0.4);
    let mut writer = VorbisWriter::create(&plain, 48_000, 2, 6.0).unwrap();
    writer.write(audio.samples()).unwrap();
    writer.finalize().unwrap();
    let plain_packets = vorbis_packets(&fs::read(&plain).unwrap());
    let (vendor, comments) = vorbis_comments(&plain_packets[1]);
    assert!(!vendor.is_empty());
    assert!(comments.is_empty());

    // Also force a comment header across multiple Ogg pages.
    let long_comment = "x".repeat(70_000);
    for comments in ["Line 1\nLine 2", long_comment.as_str()] {
        let tags = AudioTags {
            title: "Windfall étude",
            author: "Windfall",
            genre: "Electronic",
            comments,
        };
        let mut encoder = Encoder::open_with_tags(
            &tagged,
            &EncoderSettings::Vorbis { quality: 6.0 },
            48_000,
            2,
            &tags,
        )
        .unwrap();
        for block in audio.samples().chunks(2 * 137) {
            encoder.write(block).unwrap();
        }
        encoder.finalize().unwrap();
        let packets = vorbis_packets(&fs::read(&tagged).unwrap());
        let (tagged_vendor, fields) = vorbis_comments(&packets[1]);
        assert_eq!(tagged_vendor, vendor);
        assert_eq!(fields[3], format!("DESCRIPTION={comments}"));
        assert_ne!(packets[1], plain_packets[1]);
        assert_eq!(packets[0], plain_packets[0]);
        assert_eq!(packets[2..], plain_packets[2..]);
        assert_eq!(
            decode_file(&tagged).unwrap().samples(),
            decode_file(&plain).unwrap().samples()
        );
    }
}
