//! Bits and checksums of a FLAC frame.

/// Collects bits into bytes, the first bit written being the highest of its
/// byte.
#[derive(Debug, Default)]
pub(super) struct BitWriter {
    bytes: Vec<u8>,
    /// The bits that do not fill a byte yet, in the low end.
    pending: u64,
    count: u32,
}

impl BitWriter {
    /// Empties the writer and keeps its memory.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.pending = 0;
        self.count = 0;
    }

    /// Writes the low `bits` bits of `value`, at most 32.
    pub fn put(&mut self, value: u32, bits: u32) {
        debug_assert!(bits <= 32);
        let mask = (1_u64 << bits) - 1;
        self.pending = (self.pending << bits) | (u64::from(value) & mask);
        self.count += bits;
        while self.count >= 8 {
            self.count -= 8;
            self.bytes.push((self.pending >> self.count) as u8);
        }
        self.pending &= (1_u64 << self.count) - 1;
    }

    /// Writes `value` as a two's complement number of `bits` bits.
    pub fn put_signed(&mut self, value: i32, bits: u32) {
        self.put(value as u32, bits);
    }

    /// Writes `zeros` zero bits and then a one.
    pub fn put_unary(&mut self, mut zeros: u32) {
        while zeros >= 32 {
            self.put(0, 32);
            zeros -= 32;
        }
        self.put(1, zeros + 1);
    }

    /// Fills the last byte up with zero bits.
    pub fn align(&mut self) {
        if self.count > 0 {
            self.put(0, 8 - self.count);
        }
    }

    /// The whole bytes written so far.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

const fn crc8_table() -> [u8; 256] {
    let mut table = [0; 256];
    let mut index = 0;
    while index < 256 {
        let mut value = index as u8;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 0x80 != 0 {
                (value << 1) ^ 0x07
            } else {
                value << 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

const fn crc16_table() -> [u16; 256] {
    let mut table = [0; 256];
    let mut index = 0;
    while index < 256 {
        let mut value = (index as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            value = if value & 0x8000 != 0 {
                (value << 1) ^ 0x8005
            } else {
                value << 1
            };
            bit += 1;
        }
        table[index] = value;
        index += 1;
    }
    table
}

const CRC8: [u8; 256] = crc8_table();
const CRC16: [u16; 256] = crc16_table();

/// The checksum of a frame header: polynomial x^8 + x^2 + x + 1.
pub(super) fn crc8(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0, |crc, &byte| CRC8[usize::from(crc ^ byte)])
}

/// The checksum of a whole frame: polynomial x^16 + x^15 + x^2 + 1.
pub(super) fn crc16(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0, |crc, &byte| {
        (crc << 8) ^ CRC16[usize::from((crc >> 8) as u8 ^ byte)]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_fill_bytes_from_the_top() {
        let mut bits = BitWriter::default();
        bits.put(0b101, 3);
        bits.put(0, 0);
        bits.put_signed(-2, 4);
        bits.put_unary(3);
        bits.put(0xDEAD_BEEF, 32);
        bits.align();
        // 101 1110 0001 then the 32 bits, then four bits of padding.
        assert_eq!(bits.bytes(), [0xBC, 0x3B, 0xD5, 0xB7, 0xDD, 0xE0]);
        bits.clear();
        bits.put_unary(70);
        bits.align();
        assert_eq!(bits.bytes(), [0, 0, 0, 0, 0, 0, 0, 0, 0x02]);
    }

    #[test]
    fn the_checksums_match_known_values() {
        // The check values of CRC-8 and CRC-16/UMTS, which are these
        // polynomials with nothing reflected and nothing added.
        assert_eq!(crc8(b"123456789"), 0xF4);
        assert_eq!(crc16(b"123456789"), 0xFEE8);
        assert_eq!(crc8(&[]), 0);
        assert_eq!(crc16(&[]), 0);
    }
}
