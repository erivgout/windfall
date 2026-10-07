//! MD5, which a FLAC file carries of its audio so that a decoder can tell
//! whether it got every sample right. It is a checksum here and nothing
//! more: MD5 is not safe to use against anyone who wants a collision.

/// The sines of the whole numbers 1 to 64, scaled to 32 bits, as RFC 1321
/// gives them.
const SINES: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

/// How far each of the 64 steps turns its sum to the left.
const TURNS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// An MD5 sum that takes its input a piece at a time.
#[derive(Debug, Clone)]
pub(crate) struct Md5 {
    state: [u32; 4],
    /// Input that has not filled a block of 64 bytes yet.
    block: [u8; 64],
    held: usize,
    /// Bytes taken in so far.
    length: u64,
}

impl Md5 {
    pub fn new() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            block: [0; 64],
            held: 0,
            length: 0,
        }
    }

    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        if self.held > 0 {
            let take = bytes.len().min(64 - self.held);
            self.block[self.held..self.held + take].copy_from_slice(&bytes[..take]);
            self.held += take;
            bytes = &bytes[take..];
            if self.held < 64 {
                return;
            }
            let block = self.block;
            self.mix(&block);
            self.held = 0;
        }
        let (blocks, rest) = bytes.as_chunks::<64>();
        for block in blocks {
            self.mix(block);
        }
        self.block[..rest.len()].copy_from_slice(rest);
        self.held = rest.len();
    }

    /// The sum of everything taken in.
    pub fn finish(mut self) -> [u8; 16] {
        let bits = self.length.wrapping_mul(8);
        // A one bit, then zeros up to eight bytes short of a whole block,
        // then the length.
        let mut padding = [0_u8; 72];
        padding[0] = 0x80;
        let zeros = (119 - self.held) % 64;
        padding[1 + zeros..9 + zeros].copy_from_slice(&bits.to_le_bytes());
        self.update(&padding[..9 + zeros]);

        let mut sum = [0; 16];
        for (bytes, word) in sum.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            *bytes = word.to_le_bytes();
        }
        sum
    }

    fn mix(&mut self, block: &[u8; 64]) {
        let mut words = [0_u32; 16];
        for (word, bytes) in words.iter_mut().zip(block.as_chunks::<4>().0) {
            *word = u32::from_le_bytes(*bytes);
        }
        let [mut a, mut b, mut c, mut d] = self.state;
        for step in 0..64 {
            let (mixed, word) = match step / 16 {
                0 => ((b & c) | (!b & d), step),
                1 => ((d & b) | (!d & c), (5 * step + 1) % 16),
                2 => (b ^ c ^ d, (3 * step + 5) % 16),
                _ => (c ^ (b | !d), (7 * step) % 16),
            };
            let sum = a
                .wrapping_add(mixed)
                .wrapping_add(SINES[step])
                .wrapping_add(words[word]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(sum.rotate_left(TURNS[step]));
        }
        for (state, add) in self.state.iter_mut().zip([a, b, c, d]) {
            *state = state.wrapping_add(add);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn sum(input: &[u8]) -> String {
        let mut md5 = Md5::new();
        md5.update(input);
        hex(&md5.finish())
    }

    #[test]
    fn the_sums_of_rfc_1321_come_out() {
        for (input, expected) in [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ] {
            assert_eq!(sum(input.as_bytes()), expected, "{input:?}");
        }
    }

    #[test]
    fn the_sum_does_not_depend_on_how_the_input_is_cut_up() {
        let input: Vec<u8> = (0..1_000_u32).map(|index| (index * 7 + 3) as u8).collect();
        let whole = sum(&input);
        for piece in [1, 3, 55, 56, 63, 64, 65, 119, 120, 128, 999] {
            let mut md5 = Md5::new();
            for chunk in input.chunks(piece) {
                md5.update(chunk);
            }
            assert_eq!(hex(&md5.finish()), whole, "pieces of {piece}");
        }
        // Every length around the two places where the padding changes shape.
        for length in 50..=130 {
            let mut md5 = Md5::new();
            md5.update(&input[..length]);
            let mut split = Md5::new();
            split.update(&input[..length / 2]);
            split.update(&input[length / 2..length]);
            assert_eq!(md5.finish(), split.finish(), "{length} bytes");
        }
    }
}
