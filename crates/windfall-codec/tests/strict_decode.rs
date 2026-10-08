//! Authored native IEEE floating-point WAVs, without encoder sanitation.
use windfall_codec::{
    CodecError, DecodeOptions, decode_bytes_strict_with, decode_bytes_with,
    decode_file_strict_with, decode_file_with,
};

fn wav(bits: u16, values: &[f64]) -> Vec<u8> {
    let bytes = u32::from(bits / 8) * values.len() as u32;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&3_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&48_000_u32.to_le_bytes());
    wav.extend_from_slice(&(48_000 * u32::from(bits / 8)).to_le_bytes());
    wav.extend_from_slice(&(bits / 8).to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    for &value in values {
        if bits == 32 {
            wav.extend_from_slice(&(value as f32).to_le_bytes());
        } else {
            wav.extend_from_slice(&value.to_le_bytes());
        }
    }
    wav
}
struct FileFixture(std::path::PathBuf);
impl Drop for FileFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn native(bytes: &[u8], label: &str) -> FileFixture {
    let path = std::env::temp_dir().join(format!(
        "windfall-strict-{}-{label}.wav",
        std::process::id()
    ));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(bytes).unwrap();
    FileFixture(path)
}
#[test]
fn r1_native_float32_float64_nonfinite_refused_before_ordinary_sanitation() {
    let options = DecodeOptions {
        max_decoded_bytes: 8,
    };
    let mut rejected = Vec::new();
    for bits in [32, 64] {
        for (index, bad) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
            .into_iter()
            .enumerate()
        {
            let bytes = wav(bits, &[0.25, bad]);
            let file = native(&bytes, &format!("bad-{bits}-{index}"));
            assert_eq!(
                decode_bytes_with(&bytes, Some("wav"), &options)
                    .unwrap()
                    .samples(),
                &[0.25, 0.0]
            );
            assert_eq!(
                decode_file_with(&file.0, &options).unwrap().samples(),
                &[0.25, 0.0]
            );
            rejected.push((
                bits,
                index,
                matches!(
                    decode_bytes_strict_with(&bytes, Some("wav"), &options),
                    Err(CodecError::Corrupt(_))
                ),
                matches!(
                    decode_file_strict_with(&file.0, &options),
                    Err(CodecError::Corrupt(_))
                ),
            ));
        }
    }
    assert!(
        rejected.iter().all(|case| case.2 && case.3),
        "strict native/memory rejection matrix: {rejected:?}"
    );
}
#[test]
fn strict_finite_native_float_formats_and_same_byte_limits() {
    for bits in [32, 64] {
        let bytes = wav(bits, &[0.25, -0.5]);
        let file = native(&bytes, &format!("finite-{bits}"));
        let options = DecodeOptions {
            max_decoded_bytes: 8,
        };
        let ordinary = decode_file_with(&file.0, &options).unwrap();
        assert_eq!(
            decode_file_strict_with(&file.0, &options)
                .unwrap()
                .samples(),
            ordinary.samples()
        );
        assert_eq!(
            decode_bytes_strict_with(&bytes, None, &options)
                .unwrap()
                .samples(),
            ordinary.samples()
        );
        let limited = DecodeOptions {
            max_decoded_bytes: 4,
        };
        assert!(matches!(
            decode_file_strict_with(&file.0, &limited),
            Err(CodecError::TooLarge { limit_bytes: 4 })
        ));
        assert!(matches!(
            decode_bytes_strict_with(&bytes, None, &limited),
            Err(CodecError::TooLarge { limit_bytes: 4 })
        ));
    }
}
