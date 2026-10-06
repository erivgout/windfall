//! A minimal WAV writer. The bytes of every factory file are part of the
//! repository, so the encoder lives here where nothing can change them by
//! accident.

/// Encodes interleaved signed 24-bit samples as a PCM WAV file.
pub fn encode(sample_rate: u32, channels: u16, samples: &[i32]) -> Vec<u8> {
    const BYTES_PER_SAMPLE: u32 = 3;
    let data_len = samples.len() as u32 * BYTES_PER_SAMPLE;
    // RIFF chunks are word-aligned, so an odd-sized data chunk gets a pad byte.
    let padding = data_len % 2;
    let block_align = channels as u32 * BYTES_PER_SAMPLE;

    let mut bytes = Vec::with_capacity(44 + (data_len + padding) as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len + padding).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * block_align).to_le_bytes());
    bytes.extend_from_slice(&(block_align as u16).to_le_bytes());
    bytes.extend_from_slice(&((BYTES_PER_SAMPLE * 8) as u16).to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes()[..3]);
    }
    bytes.resize(bytes.len() + padding as usize, 0);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_samples_are_laid_out_as_pcm() {
        let bytes = encode(48_000, 2, &[1, -1, 8_388_607, -8_388_608]);
        assert_eq!(bytes.len(), 44 + 12);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 48);
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        // PCM, two channels, 48 kHz, 288000 bytes a second, 6-byte frames, 24 bits.
        assert_eq!(
            &bytes[20..36],
            &[1, 0, 2, 0, 0x80, 0xBB, 0, 0, 0, 0x65, 4, 0, 6, 0, 24, 0]
        );
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 12);
        assert_eq!(
            &bytes[44..],
            &[1, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F, 0, 0, 0x80]
        );
    }

    #[test]
    fn odd_sized_data_is_padded_to_an_even_length() {
        let bytes = encode(48_000, 1, &[5]);
        assert_eq!(bytes.len(), 44 + 4);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 40);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 3);
        assert_eq!(bytes[47], 0);
    }
}
