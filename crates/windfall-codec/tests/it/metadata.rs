//! Feeds the decoder files whose tags, pictures and other metadata are huge
//! or claim to be. Windfall reads none of it, so none of it may cost memory:
//! every file here has to decode like its plain twin and hold about as much
//! heap while doing so, however large the metadata says it is.

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use windfall_codec::{
    CodecError, DecodeOptions, decode_bytes, decode_bytes_with, decode_file, probe_bytes,
    probe_file,
};
use windfall_core::AudioBuffer;

use crate::common::*;
use crate::heap::peak_heap;

/// Size of the metadata in the hostile files.
const HUGE: usize = 8 * 1024 * 1024;

/// Most extra heap a file may take over its twin without the hostile
/// metadata. It is far below `HUGE` and leaves room for the reader buffers
/// that differ a little from file to file.
const SLACK: usize = 256 * 1024;

/// Most heap that metadata small enough to be let through may take, however
/// it is packed.
const SMALL_METADATA_CEILING: usize = 1024 * 1024;

/// Most heap the comment headers of an Ogg file may take. A page cannot be
/// cut down, so up to two pages of them get read, and comments of a few
/// bytes take about thirty times their size in memory.
const OGG_COMMENT_CEILING: usize = 4 * 1024 * 1024;

/// What decoding gave and the most heap any way of opening the file held.
struct Opened {
    audio: Result<AudioBuffer, CodecError>,
    peak: usize,
}

/// Decodes and probes `bytes` from memory and from a file, checks that all
/// of it agrees, and measures the heap each call holds.
fn open(bytes: &[u8], file_name: &str) -> Opened {
    // Tests run side by side and some share file names, so each call gets a
    // folder of its own.
    static OPENED: AtomicUsize = AtomicUsize::new(0);
    let dir = TempDir::new(&format!(
        "metadata-{}",
        OPENED.fetch_add(1, Ordering::Relaxed)
    ));
    let path = dir.path(file_name);
    fs::write(&path, bytes).unwrap();

    // The first run fills the tables the decoders build once per process, so
    // that they do not count against whichever file happens to come first.
    let _ = decode_bytes(bytes, None);

    let (audio, from_memory) = peak_heap(|| decode_bytes(bytes, None));
    let (from_file, from_disk) = peak_heap(|| decode_file(&path));
    let (info, probing_memory) = peak_heap(|| probe_bytes(bytes, None));
    let (file_info, probing_disk) = peak_heap(|| probe_file(&path));

    match (&audio, &from_file) {
        (Ok(memory), Ok(disk)) => {
            assert!(
                same_audio(memory, disk),
                "{file_name}: file and bytes differ"
            );
        }
        (Err(_), Err(_)) => {}
        _ => panic!("{file_name}: file and bytes disagree: {audio:?} / {from_file:?}"),
    }
    assert_eq!(info.is_ok(), file_info.is_ok(), "{file_name}: probing");
    if let (Ok(audio), Ok(info)) = (&audio, &info) {
        assert_eq!(audio.sample_rate(), info.sample_rate, "{file_name}");
        assert_eq!(audio.channels(), info.channels, "{file_name}");
    }

    Opened {
        audio,
        peak: from_memory
            .max(from_disk)
            .max(probing_memory)
            .max(probing_disk),
    }
}

fn same_audio(left: &AudioBuffer, right: &AudioBuffer) -> bool {
    left.sample_rate() == right.sample_rate()
        && left.channels() == right.channels()
        && left.samples() == right.samples()
}

/// Checks that `hostile` decodes to exactly the audio of `plain`, the same
/// file without the oversized metadata, and takes no more heap than it plus
/// `SLACK`.
fn assert_decodes_like(hostile: &[u8], plain: &[u8], file_name: &str) {
    let plain = open(plain, file_name);
    let hostile = open(hostile, file_name);
    let expected = plain.audio.expect("the plain file decodes");
    let decoded = hostile
        .audio
        .unwrap_or_else(|error| panic!("{file_name}: {error}"));
    assert!(
        same_audio(&decoded, &expected),
        "{file_name}: the audio changed"
    );
    assert!(
        hostile.peak <= plain.peak + SLACK,
        "{file_name}: held {} bytes, the plain file {}",
        hostile.peak,
        plain.peak
    );
}

/// Checks that opening `hostile` takes no more heap than `plain` plus
/// `SLACK`, whether it decodes or is turned down.
fn assert_costs_like(hostile: &[u8], plain: &[u8], file_name: &str) {
    let plain = open(plain, file_name);
    let hostile = open(hostile, file_name);
    assert!(
        hostile.peak <= plain.peak + SLACK,
        "{file_name}: held {} bytes, the plain file {}",
        hostile.peak,
        plain.peak
    );
}

/// Cuts `hostile` off after `keep` bytes, before its audio begins. Nothing
/// can be decoded from that, so both the seeking and the forward-only pass
/// run over the metadata before giving up.
fn assert_cut_off_is_refused_cheaply(hostile: &[u8], keep: usize, plain: &[u8], file_name: &str) {
    let plain = open(plain, file_name);
    let cut = open(&hostile[..keep], file_name);
    assert!(cut.audio.is_err(), "{file_name}: decoded without audio");
    assert!(
        cut.peak <= plain.peak + SLACK,
        "{file_name}: held {} bytes cut off, the plain file {}",
        cut.peak,
        plain.peak
    );
}

fn padded(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

/// One RIFF chunk: little-endian length, padded to an even size.
fn riff_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    padded(bytes)
}

/// One AIFF chunk: big-endian length, padded to an even size.
fn aiff_chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((body.len() as u32).to_be_bytes());
    bytes.extend(body);
    padded(bytes)
}

fn wave(chunks: &[Vec<u8>]) -> Vec<u8> {
    let body = chunks.concat();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((body.len() as u32 + 4).to_le_bytes());
    bytes.extend(b"WAVE");
    bytes.extend(body);
    bytes
}

fn aiff(form: &[u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
    let body = chunks.concat();
    let mut bytes = b"FORM".to_vec();
    bytes.extend((body.len() as u32 + 4).to_be_bytes());
    bytes.extend(form);
    bytes.extend(body);
    bytes
}

/// Declares a RIFF or AIFF file as long as a length field can say. Used
/// where bytes are added behind the chunks, so that the readers do not stop
/// at the end the header first gave.
fn without_end(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes[4..8].copy_from_slice(&[0xFF; 4]);
    bytes
}

/// The first 16 bytes of every `fmt ` chunk, for mono at 44.1 kHz.
fn wave_format(tag: u16, bits: u16) -> Vec<u8> {
    let block = bits.div_ceil(8);
    let mut body = tag.to_le_bytes().to_vec();
    body.extend(1_u16.to_le_bytes());
    body.extend(44_100_u32.to_le_bytes());
    body.extend((44_100 * u32::from(block)).to_le_bytes());
    body.extend(block.to_le_bytes());
    body.extend(bits.to_le_bytes());
    body
}

/// A 16-bit mono PCM `fmt ` chunk.
fn pcm_fmt() -> Vec<u8> {
    riff_chunk(b"fmt ", &wave_format(1, 16))
}

const RAMP: [i16; 4] = [0x4000, 0x2000, -0x2000, -0x4000];

fn wave_data() -> Vec<u8> {
    let samples: Vec<u8> = RAMP
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    riff_chunk(b"data", &samples)
}

/// An `INFO` list holding one title of `length` bytes.
fn info_list(length: usize) -> Vec<u8> {
    let mut body = b"INFO".to_vec();
    body.extend(riff_chunk(b"INAM", &vec![b'A'; length]));
    riff_chunk(b"LIST", &body)
}

/// The `COMM` body of a 16-bit mono AIFF at 44.1 kHz holding `RAMP`.
fn aiff_common() -> Vec<u8> {
    let mut body = 1_u16.to_be_bytes().to_vec();
    body.extend((RAMP.len() as u32).to_be_bytes());
    body.extend(16_u16.to_be_bytes());
    // 44100 as an 80-bit float.
    body.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    body
}

fn aiff_sound() -> Vec<u8> {
    // No offset, no block size.
    let mut body = vec![0; 8];
    body.extend(RAMP.iter().flat_map(|sample| sample.to_be_bytes()));
    aiff_chunk(b"SSND", &body)
}

fn plain_aiff() -> Vec<u8> {
    aiff(
        b"AIFF",
        &[aiff_chunk(b"COMM", &aiff_common()), aiff_sound()],
    )
}

fn synchsafe(value: usize) -> [u8; 4] {
    assert!(value < 1 << 28);
    [
        (value >> 21) as u8 & 0x7F,
        (value >> 14) as u8 & 0x7F,
        (value >> 7) as u8 & 0x7F,
        value as u8 & 0x7F,
    ]
}

/// An ID3v2.4 tag of `length` bytes in all, holding one attached picture.
fn id3v2_tag(length: usize) -> Vec<u8> {
    let picture = length - 20;
    let mut bytes = b"ID3\x04\x00\x00".to_vec();
    bytes.extend(synchsafe(length - 10));
    bytes.extend(b"APIC");
    bytes.extend(synchsafe(picture));
    bytes.extend([0, 0]);
    let mut body = b"\x00image/png\x00\x03\x00".to_vec();
    body.resize(picture, 0x55);
    bytes.extend(body);
    bytes
}

/// An ID3v2.4 tag of `length` bytes in all, filled with text frames of two
/// bytes each.
fn id3v2_tag_of_frames(length: usize) -> Vec<u8> {
    let mut bytes = b"ID3\x04\x00\x00".to_vec();
    bytes.extend(synchsafe(length - 10));
    while bytes.len() + 12 <= length {
        bytes.extend(b"TXXX");
        bytes.extend(synchsafe(2));
        // No flags, then a Latin-1 text with an empty description.
        bytes.extend([0; 4]);
    }
    bytes.resize(length, 0);
    bytes
}

/// A FLAC metadata block, not marked as the last one.
fn flac_block(kind: u8, body: &[u8]) -> Vec<u8> {
    assert!(body.len() < 1 << 24);
    let mut bytes = vec![kind];
    bytes.extend(&(body.len() as u32).to_be_bytes()[1..]);
    bytes.extend(body);
    bytes
}

/// The body of a FLAC `PICTURE` block holding `length` bytes of image.
fn flac_picture(length: usize) -> Vec<u8> {
    let mut body = 3_u32.to_be_bytes().to_vec();
    body.extend(9_u32.to_be_bytes());
    body.extend(b"image/png");
    // No description, then width, height, depth and palette size.
    body.extend([0; 20]);
    body.extend((length as u32).to_be_bytes());
    body.resize(body.len() + length, 0x55);
    body
}

/// Puts `blocks` right behind the stream info block that opens a FLAC file.
fn flac_with(fixture: &[u8], blocks: &[Vec<u8>]) -> Vec<u8> {
    const STREAM_INFO_END: usize = 4 + 4 + 34;
    assert_eq!(&fixture[..5], b"fLaC\x00", "stream info is not last");
    let mut bytes = fixture[..STREAM_INFO_END].to_vec();
    bytes.extend(blocks.concat());
    bytes.extend(&fixture[STREAM_INFO_END..]);
    bytes
}

/// Where the audio frames of a FLAC file begin.
fn flac_audio_start(bytes: &[u8]) -> usize {
    let mut at = 4;
    loop {
        let length = u32::from_be_bytes([0, bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        let last = bytes[at] & 0x80 != 0;
        at += 4 + length as usize;
        if last {
            return at;
        }
    }
}

struct OggPage {
    flags: u8,
    granule: u64,
    serial: u32,
    sequence: u32,
    lacing: Vec<u8>,
    body: Vec<u8>,
}

const OGG_CONTINUED: u8 = 0x01;

impl OggPage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = b"OggS\x00".to_vec();
        bytes.push(self.flags);
        bytes.extend(self.granule.to_le_bytes());
        bytes.extend(self.serial.to_le_bytes());
        bytes.extend(self.sequence.to_le_bytes());
        bytes.extend([0; 4]);
        bytes.push(self.lacing.len() as u8);
        bytes.extend(&self.lacing);
        bytes.extend(&self.body);
        let checksum = ogg_checksum(&bytes);
        bytes[22..26].copy_from_slice(&checksum.to_le_bytes());
        bytes
    }

    /// Whether the last packet on the page carries on in the next one.
    fn ends_mid_packet(&self) -> bool {
        self.lacing.last() == Some(&255)
    }
}

fn ogg_pages(bytes: &[u8]) -> Vec<OggPage> {
    let mut pages = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let header = &bytes[at..at + 27];
        assert_eq!(&header[..4], b"OggS");
        let segments = usize::from(header[26]);
        let lacing = bytes[at + 27..at + 27 + segments].to_vec();
        let length: usize = lacing.iter().map(|&value| usize::from(value)).sum();
        let body_start = at + 27 + segments;
        pages.push(OggPage {
            flags: header[5],
            granule: u64::from_le_bytes(header[6..14].try_into().unwrap()),
            serial: u32::from_le_bytes(header[14..18].try_into().unwrap()),
            sequence: u32::from_le_bytes(header[18..22].try_into().unwrap()),
            lacing,
            body: bytes[body_start..body_start + length].to_vec(),
        });
        at = body_start + length;
    }
    pages
}

/// The packets that end on `pages`, which must begin and end between packets.
fn ogg_packets(pages: &[OggPage]) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    let mut packet = Vec::new();
    for page in pages {
        let mut at = 0;
        for &segment in &page.lacing {
            packet.extend(&page.body[at..at + usize::from(segment)]);
            at += usize::from(segment);
            if segment < 255 {
                packets.push(std::mem::take(&mut packet));
            }
        }
    }
    assert!(packet.is_empty());
    packets
}

/// Lays `packets` out on full pages of a stream, numbered from `sequence`.
fn ogg_paginate(packets: &[Vec<u8>], serial: u32, mut sequence: u32) -> Vec<OggPage> {
    let mut lacing = Vec::new();
    for packet in packets {
        lacing.extend(std::iter::repeat_n(255_u8, packet.len() / 255));
        lacing.push((packet.len() % 255) as u8);
    }
    let body = packets.concat();

    let mut pages = Vec::new();
    let mut at = 0;
    let mut continued = false;
    for lacing in lacing.chunks(255) {
        let length: usize = lacing.iter().map(|&value| usize::from(value)).sum();
        let page = OggPage {
            flags: if continued { OGG_CONTINUED } else { 0 },
            // Header pages carry no position; a page no packet ends on says so.
            granule: if lacing.iter().all(|&value| value == 255) {
                u64::MAX
            } else {
                0
            },
            serial,
            sequence,
            lacing: lacing.to_vec(),
            body: body[at..at + length].to_vec(),
        };
        continued = page.ends_mid_packet();
        pages.push(page);
        at += length;
        sequence += 1;
    }
    pages
}

/// A Vorbis comment header packet holding one comment of `length` bytes.
fn vorbis_comment_packet(length: usize) -> Vec<u8> {
    let mut packet = b"\x03vorbis".to_vec();
    packet.extend(8_u32.to_le_bytes());
    packet.extend(b"Windfall");
    packet.extend(1_u32.to_le_bytes());
    packet.extend((length as u32).to_le_bytes());
    let mut comment = b"COMMENT=".to_vec();
    comment.resize(length, b'x');
    packet.extend(comment);
    // The framing bit.
    packet.push(1);
    packet
}

/// A Vorbis comment header packet of about `length` bytes, filled with
/// comments of two bytes each.
fn tiny_vorbis_comments(length: usize) -> Vec<u8> {
    let count = (length - 16) / 6;
    let mut packet = b"\x03vorbis".to_vec();
    packet.extend(0_u32.to_le_bytes());
    packet.extend((count as u32).to_le_bytes());
    for _ in 0..count {
        packet.extend(2_u32.to_le_bytes());
        packet.extend(b"A=");
    }
    // The framing bit.
    packet.push(1);
    packet
}

/// Rebuilds an Ogg Vorbis file with other header packets behind its
/// identification header. `headers` is given the file's setup header and
/// returns the packets to write. Returns the file and the offset at which
/// its audio pages begin.
fn ogg_with_headers(
    fixture: &[u8],
    headers: impl FnOnce(Vec<u8>) -> Vec<Vec<u8>>,
) -> (Vec<u8>, usize) {
    let pages = ogg_pages(fixture);
    // The identification header has the first page to itself. The comment
    // and setup headers follow on pages of their own, before any audio.
    let headers_end = 1 + pages[1..]
        .iter()
        .position(|page| page.granule != 0 && page.granule != u64::MAX)
        .expect("the file has audio pages");
    let mut original = ogg_packets(&pages[1..headers_end]);
    assert_eq!(original.len(), 2, "comment and setup headers");
    let setup = original.pop().unwrap();

    let serial = pages[0].serial;
    let mut bytes = pages[0].to_bytes();
    let mut sequence = 1;
    for page in ogg_paginate(&headers(setup), serial, sequence) {
        bytes.extend(page.to_bytes());
        sequence += 1;
    }
    let audio_start = bytes.len();
    for page in &pages[headers_end..] {
        let page = OggPage {
            flags: page.flags,
            granule: page.granule,
            serial,
            sequence,
            lacing: page.lacing.clone(),
            body: page.body.clone(),
        };
        bytes.extend(page.to_bytes());
        sequence += 1;
    }
    (bytes, audio_start)
}

/// Rebuilds an Ogg Vorbis file around a new comment header packet.
fn ogg_with_comment(fixture: &[u8], comment: Vec<u8>) -> (Vec<u8>, usize) {
    ogg_with_headers(fixture, |setup| vec![comment, setup])
}

fn read_fixture(name: &str) -> Vec<u8> {
    fs::read(fixture(name)).unwrap()
}

#[test]
fn a_huge_wav_info_field_is_not_loaded() {
    // One frame of audio behind a title of 8 MiB.
    let audio = riff_chunk(b"data", &0x4000_i16.to_le_bytes());
    let bytes = wave(&[pcm_fmt(), info_list(HUGE), audio.clone()]);
    let plain = wave(&[pcm_fmt(), audio]);

    // The limit on decoded audio is met, and says nothing about the title.
    let one_sample = DecodeOptions {
        max_decoded_bytes: 4,
    };
    let _ = decode_bytes(&bytes, None);
    let (decoded, peak) = peak_heap(|| decode_bytes_with(&bytes, None, &one_sample));
    assert_eq!(decoded.unwrap().samples(), [0.5]);
    let (_, plain_peak) = peak_heap(|| decode_bytes_with(&plain, None, &one_sample));
    assert!(
        peak <= plain_peak + SLACK,
        "held {peak} bytes, the plain file {plain_peak}"
    );

    assert_decodes_like(&bytes, &plain, "huge-info.wav");
}

#[test]
fn huge_wav_lists_cost_nothing_wherever_they_sit() {
    let plain = wave(&[pcm_fmt(), wave_data()]);

    // Several lists, each of them huge.
    let twice = wave(&[pcm_fmt(), info_list(HUGE), info_list(HUGE), wave_data()]);
    assert_decodes_like(&twice, &plain, "two-lists.wav");

    // A list of odd length, which is followed by a padding byte.
    let mut odd = b"INFO".to_vec();
    odd.extend(riff_chunk(b"INAM", &vec![b'A'; HUGE + 1]));
    odd.pop();
    let odd = wave(&[pcm_fmt(), riff_chunk(b"LIST", &odd), wave_data()]);
    assert_decodes_like(&odd, &plain, "odd-list.wav");

    // A list that says it is far longer than the file.
    let mut overstated = info_list(64);
    overstated[4..8].copy_from_slice(&0x7FFF_FFF0_u32.to_le_bytes());
    overstated[16..20].copy_from_slice(&0x7FFF_FF00_u32.to_le_bytes());
    let mut bytes = wave(&[pcm_fmt(), overstated, wave_data()]);
    bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_costs_like(&bytes, &plain, "overstated-list.wav");

    // Many small lists that add up.
    let mut chunks = vec![pcm_fmt()];
    chunks.extend(std::iter::repeat_n(info_list(2_000), HUGE / 2_000));
    chunks.push(wave_data());
    assert_decodes_like(&wave(&chunks), &plain, "many-lists.wav");

    let cut = wave(&[pcm_fmt(), info_list(HUGE), wave_data()]);
    assert_cut_off_is_refused_cheaply(&cut, cut.len() - 16, &plain, "cut-list.wav");
}

#[test]
fn wav_chunks_that_throw_the_reader_off_course_hide_nothing() {
    // Symphonia reads some chunks without regard to their stated length and
    // looks for the next chunk where it happens to stop. A huge list put
    // exactly there must be found all the same.
    let plain = wave(&[pcm_fmt(), wave_data()]);
    let hostile_list = info_list(HUGE);

    // An extensible format chunk longer than the 40 bytes that are read: the
    // list's header sits in its last 8 bytes.
    let mut extensible = wave_format(0xFFFE, 16);
    extensible.extend(22_u16.to_le_bytes());
    extensible.extend(16_u16.to_le_bytes());
    extensible.extend(4_u32.to_le_bytes());
    extensible.extend([
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ]);
    extensible.extend(&hostile_list[..8]);
    let mut bytes = without_end(wave(&[riff_chunk(b"fmt ", &extensible)]));
    bytes.extend(&hostile_list[8..]);
    bytes.extend(wave_data());
    assert_costs_like(&bytes, &plain, "long-extensible.wav");

    // A mu-law format chunk whose extra bytes lie beyond its stated end.
    let mut mu_law = wave_format(7, 8);
    mu_law.extend(6_u16.to_le_bytes());
    let mut bytes = without_end(wave(&[riff_chunk(b"fmt ", &mu_law)]));
    bytes.extend([0; 6]);
    bytes.extend(&hostile_list);
    bytes.extend(wave_data());
    assert_costs_like(&bytes, &plain, "mu-law-extra.wav");

    // The same for ADPCM, which Windfall cannot decode but whose header is
    // read before that is known.
    let mut adpcm = wave_format(2, 4);
    adpcm[12..14].copy_from_slice(&256_u16.to_le_bytes());
    adpcm.extend(32_u16.to_le_bytes());
    adpcm.extend([0; 2]);
    let mut bytes = without_end(wave(&[riff_chunk(b"fmt ", &adpcm)]));
    bytes.extend([0; 30]);
    bytes.extend(&hostile_list);
    bytes.extend(wave_data());
    assert_costs_like(&bytes, &plain, "adpcm-extra.wav");

    // Behind a list of odd length Symphonia skips one byte too many.
    let mut small = b"INFO".to_vec();
    small.extend(riff_chunk(b"INAM", b"odd"));
    small.pop();
    let mut bytes = without_end(wave(&[pcm_fmt(), riff_chunk(b"LIST", &small)]));
    bytes.push(0);
    bytes.extend(&hostile_list);
    bytes.extend(wave_data());
    assert_costs_like(&bytes, &plain, "odd-list-then-list.wav");
}

#[test]
fn huge_aiff_chunks_are_not_loaded() {
    let plain = plain_aiff();
    let common = || aiff_chunk(b"COMM", &aiff_common());
    let huge = vec![b'A'; HUGE];

    for id in [b"ANNO", b"NAME", b"AUTH", b"(c) ", b"APPL"] {
        let name = format!("huge-{}.aiff", String::from_utf8_lossy(id).trim());
        let name = name.replace(['(', ')'], "");

        let before = aiff(b"AIFF", &[common(), aiff_chunk(id, &huge), aiff_sound()]);
        assert_decodes_like(&before, &plain, &name);
        // Past the audio, where only the seeking pass looks.
        let after = aiff(b"AIFF", &[common(), aiff_sound(), aiff_chunk(id, &huge)]);
        assert_decodes_like(&after, &plain, &name);

        let keep = before.len() - aiff_sound().len();
        assert_cut_off_is_refused_cheaply(&before, keep, &plain, &name);
    }

    // Many small annotations that add up.
    let mut chunks = vec![common()];
    chunks.extend(std::iter::repeat_n(
        aiff_chunk(b"ANNO", &[b'A'; 2_000]),
        HUGE / 2_000,
    ));
    chunks.push(aiff_sound());
    assert_decodes_like(&aiff(b"AIFF", &chunks), &plain, "many-annotations.aiff");
}

#[test]
fn huge_id3_tags_inside_aiff_are_not_loaded() {
    let plain = plain_aiff();
    let common = || aiff_chunk(b"COMM", &aiff_common());

    let tagged = aiff(
        b"AIFF",
        &[
            common(),
            aiff_chunk(b"ID3 ", &id3v2_tag(HUGE)),
            aiff_sound(),
        ],
    );
    assert_decodes_like(&tagged, &plain, "huge-id3.aiff");

    // A ten-byte chunk holding only the tag's header. The tag it announces
    // runs on into the chunk behind it, whose header doubles as the header
    // of a picture frame: its length reads as 8 MiB to the chunk reader and
    // as 2 MiB to the tag reader.
    let mut frame = vec![0, 0];
    frame.extend(b"\x00image/png\x00\x03\x00");
    frame.resize(0x007F_7F7E, 0x55);
    let spilled = aiff(
        b"AIFF",
        &[
            common(),
            aiff_chunk(b"ID3 ", &id3v2_tag(HUGE)[..10]),
            aiff_chunk(b"APIC", &frame),
            aiff_sound(),
        ],
    );
    assert_decodes_like(&spilled, &plain, "spilled-id3.aiff");
}

#[test]
fn aiff_tables_sized_by_a_count_are_not_built() {
    let plain = plain_aiff();
    let common = || aiff_chunk(b"COMM", &aiff_common());

    // Two bytes are enough to ask for 65535 markers or comments.
    for id in [b"MARK", b"COMT"] {
        let name = format!("counted-{}.aiff", String::from_utf8_lossy(id));
        let counted = aiff_chunk(id, &[0xFF, 0xFF]);
        let bytes = aiff(b"AIFF", &[common(), counted, aiff_sound()]);
        assert_decodes_like(&bytes, &plain, &name);
    }
}

#[test]
fn aiff_chunks_that_throw_the_reader_off_course_hide_nothing() {
    let plain = plain_aiff();
    let hostile = aiff_chunk(b"ANNO", &vec![b'A'; HUGE]);

    // Only 18 bytes of a common chunk are read, however long it says it is.
    // The annotation's header fills the 8 bytes behind them.
    let mut long_common = aiff_common();
    long_common.extend(&hostile[..8]);
    let mut bytes = without_end(aiff(b"AIFF", &[aiff_chunk(b"COMM", &long_common)]));
    bytes.extend(&hostile[8..]);
    bytes.extend(aiff_sound());
    assert_decodes_like(&bytes, &plain, "long-common.aiff");

    // In AIFF-C the chunk ends with the codec's name, read by its own length
    // byte: 9 here, which leads 4 bytes past the chunk's stated end.
    let mut named = aiff_common();
    named.extend(b"NONE");
    named.extend(b"\x09abcde");
    let mut bytes = without_end(aiff(b"AIFC", &[aiff_chunk(b"COMM", &named)]));
    bytes.extend(b"fghi");
    bytes.extend(&hostile);
    bytes.extend(aiff_sound());
    assert_decodes_like(&bytes, &plain, "long-codec-name.aifc");
}

#[test]
fn a_huge_id3_tag_in_front_of_an_mp3_is_not_loaded() {
    for file in ["mp3_44k_stereo.mp3", "mp3_22k_mono.mp3"] {
        let plain = read_fixture(file);

        let mut tagged = id3v2_tag(HUGE);
        tagged.extend(&plain);
        assert_decodes_like(&tagged, &plain, file);
        assert_cut_off_is_refused_cheaply(&tagged, HUGE, &plain, file);
        assert_cut_off_is_refused_cheaply(&tagged, HUGE / 2, &plain, file);

        // Behind some junk, and two in a row.
        let mut later = vec![0; 300];
        later.extend(id3v2_tag(HUGE));
        later.extend(id3v2_tag(HUGE));
        later.extend(&plain);
        assert_decodes_like(&later, &plain, file);
    }
}

#[test]
fn a_huge_id3_tag_in_front_of_other_formats_is_not_loaded() {
    for file in [
        "wav_s16_44k_stereo.wav",
        "aiff_s16_44k_stereo.aiff",
        "flac_s16_44k_stereo.flac",
        "vorbis_44k_stereo.ogg",
    ] {
        let plain = read_fixture(file);
        let mut tagged = id3v2_tag(HUGE);
        tagged.extend(&plain);
        assert_decodes_like(&tagged, &plain, file);
    }
}

#[test]
fn huge_flac_metadata_blocks_are_not_loaded() {
    const APPLICATION: u8 = 2;
    const SEEK_TABLE: u8 = 3;
    const VORBIS_COMMENT: u8 = 4;
    const PICTURE: u8 = 6;

    let mut comment = 0_u32.to_le_bytes().to_vec();
    comment.extend(1_u32.to_le_bytes());
    comment.extend((HUGE as u32).to_le_bytes());
    comment.extend(b"COMMENT=");
    comment.resize(12 + HUGE, b'x');

    // 18 bytes to a seek point: sample number, offset, block length. The
    // points have to rise to be kept.
    let mut seek_table = Vec::with_capacity(HUGE);
    for point in 1..=(HUGE / 18) as u64 {
        seek_table.extend(point.to_be_bytes());
        seek_table.extend(point.to_be_bytes());
        seek_table.extend([0; 2]);
    }

    let mut application = b"wndf".to_vec();
    application.resize(HUGE, 0x55);

    for file in ["flac_s16_44k_stereo.flac", "flac_s24_48k_mono.flac"] {
        let plain = read_fixture(file);
        let cases = [
            ("picture", vec![flac_block(PICTURE, &flac_picture(HUGE))]),
            ("application", vec![flac_block(APPLICATION, &application)]),
            ("comment", vec![flac_block(VORBIS_COMMENT, &comment)]),
            ("seek table", vec![flac_block(SEEK_TABLE, &seek_table)]),
            // Blocks top out at 16 MiB each, but there can be any number.
            (
                "many pictures",
                vec![flac_block(PICTURE, &flac_picture(2_000)); HUGE / 2_000],
            ),
        ];
        for (what, blocks) in cases {
            let hostile = flac_with(&plain, &blocks);
            let name = format!("{what}-{file}").replace(' ', "-");
            assert_decodes_like(&hostile, &plain, &name);
            let keep = flac_audio_start(&hostile);
            assert_cut_off_is_refused_cheaply(&hostile, keep, &plain, &name);
        }
    }
}

#[test]
fn a_huge_vorbis_comment_is_not_loaded() {
    for file in ["vorbis_44k_stereo.ogg", "vorbis_48k_mono.ogg"] {
        let plain = read_fixture(file);
        let (hostile, audio_start) = ogg_with_comment(&plain, vorbis_comment_packet(HUGE));
        assert_decodes_like(&hostile, &plain, file);
        assert_cut_off_is_refused_cheaply(&hostile, audio_start, &plain, file);

        // A comment that just spills over one page.
        let (hostile, _) = ogg_with_comment(&plain, vorbis_comment_packet(70_000));
        assert_decodes_like(&hostile, &plain, file);
    }
}

#[test]
fn small_metadata_does_not_change_the_audio() {
    let plain_wave = wave(&[pcm_fmt(), wave_data()]);
    let tagged = wave(&[pcm_fmt(), info_list(40), wave_data(), info_list(40)]);
    assert_decodes_like(&tagged, &plain_wave, "small-info.wav");

    let tagged = aiff(
        b"AIFF",
        &[
            aiff_chunk(b"NAME", b"A short ramp"),
            aiff_chunk(b"COMM", &aiff_common()),
            aiff_chunk(b"ANNO", b"made for a test"),
            aiff_chunk(b"ID3 ", &id3v2_tag(400)),
            aiff_sound(),
            aiff_chunk(b"(c) ", b"CC0"),
        ],
    );
    assert_decodes_like(&tagged, &plain_aiff(), "small-tags.aiff");

    let plain = read_fixture("mp3_44k_stereo.mp3");
    let mut tagged = id3v2_tag(400);
    tagged.extend(&plain);
    assert_decodes_like(&tagged, &plain, "small-tag.mp3");

    let plain = read_fixture("flac_s16_44k_stereo.flac");
    let tagged = flac_with(&plain, &[flac_block(6, &flac_picture(400))]);
    assert_decodes_like(&tagged, &plain, "small-picture.flac");

    let plain = read_fixture("vorbis_44k_stereo.ogg");
    let (tagged, _) = ogg_with_comment(&plain, vorbis_comment_packet(400));
    assert_decodes_like(&tagged, &plain, "small-comment.ogg");
}

#[test]
fn comment_headers_in_ogg_add_up_only_so_far() {
    let plain = read_fixture("vorbis_44k_stereo.ogg");
    let ceiling = open(&plain, "plain.ogg").peak + OGG_COMMENT_CEILING;

    // 8 MiB of comment headers, none of them longer than a page. One is
    // made of two-byte comments, which cost the most memory per byte, the
    // other of one long comment each.
    let tiny_comments = tiny_vorbis_comments(20_000);
    let one_comment = vorbis_comment_packet(40_000);
    for comment in [tiny_comments, one_comment] {
        let (hostile, _) = ogg_with_headers(&plain, |setup| {
            let mut headers = vec![setup];
            headers.extend(std::iter::repeat_n(comment.clone(), HUGE / comment.len()));
            headers
        });
        let hostile = open(&hostile, "many-comments.ogg");
        let decoded = hostile.audio.expect("the file decodes");
        assert!(same_audio(&decoded, &decode_bytes(&plain, None).unwrap()));
        assert!(
            hostile.peak <= ceiling,
            "held {} bytes of at most {ceiling}",
            hostile.peak
        );
    }
}

#[test]
fn a_huge_packet_among_ogg_audio_is_not_put_together() {
    let plain = read_fixture("vorbis_44k_stereo.ogg");
    let mut pages = ogg_pages(&plain);
    let serial = pages[0].serial;

    // In the middle of the audio, one packet of 8 MiB. Symphonia joins a
    // packet before it knows what is in it.
    let middle = pages.len() / 2;
    let junk = ogg_paginate(&[vec![0x55; HUGE]], serial, middle as u32);
    let after = pages.split_off(middle);
    pages.extend(junk);
    pages.extend(after);

    let mut hostile = Vec::new();
    for (sequence, page) in pages.iter_mut().enumerate() {
        page.sequence = sequence as u32;
        hostile.extend(page.to_bytes());
    }
    assert_costs_like(&hostile, &plain, "huge-packet.ogg");
}

#[test]
fn metadata_that_is_let_through_stays_small() {
    // Each file carries about as much metadata as is let through, packed
    // the way that costs the most memory: many tags with next to nothing in
    // them.
    const LET_THROUGH: usize = 16 * 1024 - 64;

    let check = |tagged: &[u8], plain: &[u8], file_name: &str| {
        let plain = open(plain, file_name);
        let tagged = open(tagged, file_name);
        let expected = plain.audio.expect("the plain file decodes");
        let decoded = tagged
            .audio
            .unwrap_or_else(|error| panic!("{file_name}: {error}"));
        assert!(same_audio(&decoded, &expected), "{file_name}");
        assert!(
            tagged.peak <= plain.peak + SMALL_METADATA_CEILING,
            "{file_name}: held {} bytes, the plain file {}",
            tagged.peak,
            plain.peak
        );
    };

    let mut fields = b"INFO".to_vec();
    while fields.len() + 8 <= LET_THROUGH {
        fields.extend(riff_chunk(b"INAM", b""));
    }
    let tagged = wave(&[pcm_fmt(), riff_chunk(b"LIST", &fields), wave_data()]);
    check(
        &tagged,
        &wave(&[pcm_fmt(), wave_data()]),
        "empty-fields.wav",
    );

    let common = || aiff_chunk(b"COMM", &aiff_common());
    for id in [b"NAME", b"APPL"] {
        let mut chunks = vec![common()];
        chunks.extend(std::iter::repeat_n(aiff_chunk(id, b"abcd"), 400));
        chunks.push(aiff_sound());
        check(&aiff(b"AIFF", &chunks), &plain_aiff(), "small-chunks.aiff");
    }
    let tag = aiff_chunk(b"ID3 ", &id3v2_tag_of_frames(LET_THROUGH));
    let tagged = aiff(b"AIFF", &[common(), tag, aiff_sound()]);
    check(&tagged, &plain_aiff(), "short-frames.aiff");

    let plain = read_fixture("mp3_44k_stereo.mp3");
    let mut tagged = id3v2_tag_of_frames(LET_THROUGH);
    tagged.extend(&plain);
    check(&tagged, &plain, "short-frames.mp3");

    // The fixture's own comment block takes a little of the budget.
    let plain = read_fixture("flac_s16_44k_stereo.flac");
    let comments = tiny_vorbis_comments(LET_THROUGH - 2_200);
    let tagged = flac_with(&plain, &[flac_block(4, &comments[7..comments.len() - 1])]);
    check(&tagged, &plain, "short-comments.flac");

    // A cue sheet: 396 bytes, then one track of 36 bytes with 255 index
    // points of 12 bytes after another.
    let mut cue_sheet = vec![0; 395];
    cue_sheet.push(4);
    for track in 1..=4 {
        cue_sheet.extend([0; 8]);
        cue_sheet.push(track);
        cue_sheet.extend([0; 26]);
        cue_sheet.push(255);
        cue_sheet.extend([0; 255 * 12]);
    }
    let tagged = flac_with(&plain, &[flac_block(5, &cue_sheet)]);
    check(&tagged, &plain, "cue-sheet.flac");
}

#[test]
fn a_file_cannot_start_with_tags_without_end() {
    let plain = read_fixture("mp3_44k_stereo.mp3");
    let empty_tag = b"ID3\x04\x00\x00\x00\x00\x00\x00";

    // A handful of tags is odd but harmless.
    let mut tagged = empty_tag.repeat(20);
    tagged.extend(&plain);
    assert_decodes_like(&tagged, &plain, "twenty-tags.mp3");

    // The probe keeps an entry for every tag it meets, ten times the size
    // of an empty tag. 4 MiB of them, back to back and with a byte between.
    for gap in [0, 1] {
        let mut tag = empty_tag.to_vec();
        tag.resize(empty_tag.len() + gap, 0);
        let mut tagged = tag.repeat(HUGE / 2 / tag.len());
        tagged.extend(&plain);

        let opened = open(&tagged, "endless-tags.mp3");
        assert!(matches!(opened.audio, Err(CodecError::Corrupt(_))));
        let plain_peak = open(&plain, "plain.mp3").peak;
        assert!(
            opened.peak <= plain_peak + SLACK,
            "held {} bytes, the plain file {plain_peak}",
            opened.peak
        );
    }
}

/// What a file of random chunks is built from.
struct Soup {
    rng: Rng,
    bytes: Vec<u8>,
    /// Offsets at which a reader that misjudges a chunk might look for the
    /// next one.
    landings: Vec<usize>,
}

impl Soup {
    fn new(seed: u64, opening: &[u8]) -> Self {
        Self {
            rng: Rng::new(seed),
            bytes: opening.to_vec(),
            landings: Vec::new(),
        }
    }

    /// Adds a chunk header and body, with the padding byte or without, and
    /// notes where the readers could end up behind it.
    fn chunk(&mut self, id: &[u8; 4], length: [u8; 4], body: &[u8], lands_at: &[usize]) {
        let start = self.bytes.len() + 8;
        self.bytes.extend(id);
        self.bytes.extend(length);
        self.bytes.extend(body);
        self.landings.extend(lands_at.iter().map(|at| start + at));
        self.landings
            .extend((0..3).map(|extra| start + body.len() + extra));
        if body.len() % 2 == 1 && self.rng.below(4) > 0 {
            self.bytes.push(0);
        }
    }

    fn noise(&mut self, most: usize) -> Vec<u8> {
        let length = self.rng.below(most + 1);
        (0..length).map(|_| self.rng.next() as u8).collect()
    }

    /// Writes `bomb` over the file at some of the noted offsets and at a
    /// few arbitrary ones.
    fn plant(&mut self, bomb: &[u8]) {
        for _ in 0..self.rng.below(4) {
            let at = match self.rng.below(4) {
                0 => self.rng.below(self.bytes.len() + 1),
                _ if self.landings.is_empty() => continue,
                _ => self.landings[self.rng.below(self.landings.len())],
            };
            let end = at + bomb.len();
            if self.bytes.len() < end {
                self.bytes.resize(end, 0);
            }
            self.bytes[at..end].copy_from_slice(bomb);
        }
    }
}

/// Opens `bytes` as the app would and returns the most heap that took.
fn peak_of_opening(bytes: &[u8]) -> usize {
    let (_, decoding) = peak_heap(|| decode_bytes(bytes, None));
    let (_, probing) = peak_heap(|| probe_bytes(bytes, None));
    decoding.max(probing)
}

#[test]
fn wav_chunks_in_any_order_hide_no_list() {
    // A list that claims 5 MiB makes Symphonia set aside 4 MiB before it
    // reads a byte of it. Such lists are written wherever a reader might
    // look for a chunk, among chunks of every kind that Symphonia reads by
    // rules of their own.
    let mut bomb = b"LIST".to_vec();
    bomb.extend(0x0050_0000_u32.to_le_bytes());
    bomb.extend(b"INFOINAM");
    bomb.extend(0x004F_FFF0_u32.to_le_bytes());

    let plain = wave(&[pcm_fmt(), wave_data()]);
    let _ = decode_bytes(&plain, None);
    let ceiling = peak_of_opening(&plain) + SMALL_METADATA_CEILING;

    for seed in 1..=4_000 {
        let mut soup = Soup::new(seed, b"RIFF\xFF\xFF\xFF\xFFWAVE");
        for _ in 0..1 + soup.rng.below(6) {
            match soup.rng.below(8) {
                0 => {
                    // PCM or float, in each of the three lengths.
                    let tag = [1, 3][soup.rng.below(2)];
                    let mut body = wave_format(tag, 32);
                    body.resize([16, 18, 40][soup.rng.below(3)], 0);
                    soup.chunk(b"fmt ", (body.len() as u32).to_le_bytes(), &body, &[]);
                }
                1 => {
                    // A-law, mu-law and ADPCM state a count of extra bytes.
                    let tag = [2, 6, 7, 0x11][soup.rng.below(4)];
                    let extra = [0, 2, 9, 32, 40][soup.rng.below(5)];
                    let mut body = wave_format(tag, if tag == 2 || tag == 0x11 { 4 } else { 8 });
                    body[12..14].copy_from_slice(&256_u16.to_le_bytes());
                    body.extend((extra as u16).to_le_bytes());
                    let stated = [18, 20, 18 + extra][soup.rng.below(3)];
                    body.resize(stated.max(18), 0);
                    soup.chunk(
                        b"fmt ",
                        (stated as u32).to_le_bytes(),
                        &body,
                        &[18 + extra, 18 + extra + 1],
                    );
                }
                2 => {
                    // Extensible, at its proper length and longer.
                    let mut body = wave_format(0xFFFE, 16);
                    body.extend(22_u16.to_le_bytes());
                    body.extend(16_u16.to_le_bytes());
                    body.extend(4_u32.to_le_bytes());
                    body.extend([
                        1, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71,
                    ]);
                    body.resize(40 + [0, 8, 9, 20][soup.rng.below(4)], 0);
                    soup.chunk(b"fmt ", (body.len() as u32).to_le_bytes(), &body, &[40, 41]);
                }
                3 | 4 => {
                    // A small list, of even or odd length, with fields or
                    // of another kind.
                    let mut body = [b"INFO", b"adtl"][soup.rng.below(2)].to_vec();
                    for _ in 0..soup.rng.below(3) {
                        let text = soup.noise(9);
                        body.extend(riff_chunk(b"ICMT", &text));
                    }
                    body.truncate(body.len() - soup.rng.below(2).min(body.len() - 4));
                    soup.chunk(b"LIST", (body.len() as u32).to_le_bytes(), &body, &[]);
                }
                5 => soup.chunk(b"fact", 4_u32.to_le_bytes(), &[0; 4], &[]),
                _ => {
                    let body = soup.noise(40);
                    soup.chunk(b"junk", (body.len() as u32).to_le_bytes(), &body, &[]);
                }
            }
        }
        soup.plant(&bomb);
        soup.bytes.extend(wave_data());

        let peak = peak_of_opening(&soup.bytes);
        assert!(
            peak <= ceiling,
            "seed {seed}: held {peak} of {ceiling} bytes"
        );
    }
}

#[test]
fn aiff_chunks_in_any_order_hide_no_text() {
    // As for WAV: annotations that claim 5 MiB, a marker table that claims
    // 65535 entries and a tag that claims 5 MiB, wherever a reader might
    // look for a chunk.
    let mut annotation = b"ANNO".to_vec();
    annotation.extend(0x0050_0000_u32.to_be_bytes());
    let mut markers = b"MARK".to_vec();
    markers.extend(2_u32.to_be_bytes());
    markers.extend([0xFF, 0xFF]);
    let mut tag = b"ID3 ".to_vec();
    tag.extend(20_u32.to_be_bytes());
    tag.extend(b"ID3\x04\x00\x00");
    tag.extend(synchsafe(0x0050_0000));
    tag.extend(b"APIC");
    tag.extend(synchsafe(0x004F_FFF0));
    tag.extend([0, 0]);
    let bombs = [annotation, markers, tag];

    let plain = plain_aiff();
    let _ = decode_bytes(&plain, None);
    let ceiling = peak_of_opening(&plain) + SMALL_METADATA_CEILING;

    for seed in 1..=4_000 {
        let compressed = seed % 2 == 0;
        let opening: &[u8] = if compressed {
            b"FORM\xFF\xFF\xFF\xF0AIFC"
        } else {
            b"FORM\xFF\xFF\xFF\xF0AIFF"
        };
        let mut soup = Soup::new(seed, opening);
        for _ in 0..1 + soup.rng.below(6) {
            match soup.rng.below(8) {
                0 | 1 => {
                    // The common chunk, at its proper length or another,
                    // in AIFF-C with a codec name of any length.
                    let mut body = aiff_common();
                    let mut read = 18;
                    if compressed {
                        body.extend([b"NONE", b"sowt", b"fl32"][soup.rng.below(3)]);
                        let name = soup.noise(12);
                        body.push(name.len() as u8);
                        body.extend(&name);
                        read = 23 + (name.len() | 1);
                    }
                    let stated = [body.len(), 18, read, body.len() + 7][soup.rng.below(4)];
                    body.resize(stated.max(body.len()), 0);
                    soup.chunk(
                        b"COMM",
                        (stated as u32).to_be_bytes(),
                        &body,
                        &[18, read, read + 1],
                    );
                }
                2 => {
                    let id = [b"NAME", b"AUTH", b"(c) ", b"ANNO", b"APPL"][soup.rng.below(5)];
                    let body = soup.noise(20);
                    soup.chunk(id, (body.len() as u32).to_be_bytes(), &body, &[]);
                }
                3 => {
                    // Markers and comments, with a count that may not match.
                    let id = [b"MARK", b"COMT"][soup.rng.below(2)];
                    let mut body = (soup.rng.below(4) as u16).to_be_bytes().to_vec();
                    body.extend(soup.noise(30));
                    let lands_at: Vec<usize> = (2..body.len()).collect();
                    soup.chunk(id, (body.len() as u32).to_be_bytes(), &body, &lands_at);
                }
                4 => {
                    // A tag chunk whose tag is as long as the chunk, or
                    // shorter, or longer.
                    let mut body = b"ID3\x04\x00\x00".to_vec();
                    let frames = soup.noise(30);
                    let claimed = frames.len() + [0, 0, 7, 300][soup.rng.below(4)];
                    body.extend(synchsafe(claimed));
                    body.extend(&frames);
                    body.truncate(
                        body.len() - soup.rng.below(2) * soup.rng.below(8).min(frames.len()),
                    );
                    soup.chunk(
                        b"ID3 ",
                        (body.len() as u32).to_be_bytes(),
                        &body,
                        &[10 + claimed, 11 + claimed],
                    );
                }
                5 => {
                    let mut body = vec![0; 8];
                    body.extend(RAMP.iter().flat_map(|sample| sample.to_be_bytes()));
                    soup.chunk(b"SSND", (body.len() as u32).to_be_bytes(), &body, &[]);
                }
                _ => {
                    let body = soup.noise(40);
                    soup.chunk(b"junk", (body.len() as u32).to_be_bytes(), &body, &[]);
                }
            }
        }
        let bomb = &bombs[soup.rng.below(bombs.len())];
        soup.plant(bomb);
        soup.bytes.extend(aiff_sound());

        let peak = peak_of_opening(&soup.bytes);
        assert!(
            peak <= ceiling,
            "seed {seed}: held {peak} of {ceiling} bytes"
        );
    }
}

#[test]
fn flac_blocks_in_any_order_hide_no_picture() {
    // A picture block that claims 5 MiB, with a media type as long.
    let mut bomb = flac_block(6, &[]);
    bomb[1..4].copy_from_slice(&0x0050_0000_u32.to_be_bytes()[1..]);
    bomb.extend(3_u32.to_be_bytes());
    bomb.extend(0x004F_FFF0_u32.to_be_bytes());

    let plain = read_fixture("flac_s16_44k_stereo.flac");
    let stream_info = &plain[..4 + 4 + 34];
    let audio = &plain[flac_audio_start(&plain)..];
    let _ = decode_bytes(&plain, None);
    let ceiling = peak_of_opening(&plain) + SMALL_METADATA_CEILING;

    for seed in 1..=2_000 {
        let mut soup = Soup::new(seed, stream_info);
        let blocks = 1 + soup.rng.below(8);
        for block in 0..blocks {
            let kind = [1, 2, 3, 4, 5, 6, 9, 127][soup.rng.below(8)];
            let last = if block + 1 == blocks || soup.rng.below(12) == 0 {
                0x80
            } else {
                0
            };
            let body = soup.noise(60);
            soup.landings.push(soup.bytes.len());
            soup.bytes.extend(flac_block(kind | last, &body));
            soup.landings.push(soup.bytes.len());
        }
        soup.plant(&bomb);
        soup.bytes.extend(audio);

        let peak = peak_of_opening(&soup.bytes);
        assert!(
            peak <= ceiling,
            "seed {seed}: held {peak} of {ceiling} bytes"
        );
    }
}
