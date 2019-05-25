extern crate num_traits;

use num_traits::{NumCast, PrimInt};

pub fn encode_vec<N: PrimInt + std::fmt::Debug>(n: N) -> Vec<u8> {
    let mask = NumCast::from(0x7f).unwrap();
    let zero = NumCast::from(0).unwrap();
    let mut out = Vec::new();

    if n == zero {
        out.push(0x0);
        return out;
    }

    let nbits = std::mem::size_of::<N>() * 8;
    let has_rem = (nbits % 7) != 0;
    let places = (nbits / 7) + (if has_rem { 1 } else { 0 });
    let it = (0..places)
        .rev()
        .map(|i| {
            let shift = i * 7;
            let byte = (n.unsigned_shr(shift as u32) & mask).to_u8().unwrap();
            (i, byte)
        })
        .skip_while(|&(_, byte)| byte == 0)
        .map(|(i, byte)| if i != 0 { 0x80u8 | byte } else { byte });

    out.extend(it);

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_encode {
        ($name: ident, $expected: expr, $input: expr) => {
            #[test]
            fn $name() {
                assert_eq!(&$expected, &*encode_vec($input));
            }
        };
    }

    test_encode!(encode_0x00000000u8, [0], 0u8);
    test_encode!(encode_0x0000007fu8, [0x7fu8], 0x0000007Fu8);
    test_encode!(encode_0x0000007fu64, [0x7fu8], 0x0000007Fu64);
    test_encode!(encode_0x00000080u8, [0x81, 0x0], 0x00000080u8);
    test_encode!(encode_0x00000080u16, [0x81, 0x0], 0x00000080u16);
    test_encode!(encode_0x00002000u16, [0xc0u8, 0x0], 0x00002000u16);
    test_encode!(encode_0x00003fffu16, [0xFF, 0x7F], 0x00003FFFu16);
    test_encode!(encode_0x0000400016, [0x81, 0x80, 0x00], 0x00004000u16);
    test_encode!(encode_0x001fffffu32, [0xFF, 0xFF, 0x7F], 0x001FFFFFu32);
    test_encode!(
        encode_0x00200000u32,
        [0x81, 0x80, 0x80, 0x00],
        0x00200000u32
    );
    test_encode!(
        encode_0x08000000u32,
        [0xC0, 0x80, 0x80, 0x00],
        0x08000000u32
    );
    test_encode!(
        encode_0x0fffffffu32,
        [0xFF, 0xFF, 0xFF, 0x7F],
        0x0FFFFFFFu32
    );
}
