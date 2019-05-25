extern crate num_traits;

use num_traits::{NumCast, PrimInt};

pub fn encode_vec<N: PrimInt>(n: N) -> Vec<u8> {
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

#[inline(never)]
pub fn decode_slice<N: PrimInt>(slice: &[u8]) -> (usize, N) {
    let zero: N = NumCast::from(0).unwrap();

    let mut val = zero;
    let mut i = 0;
    loop {
        let byte = slice[i];
        let msb = (byte & 0x80u8) != 0;
        let data = NumCast::from(byte & 0x7f).unwrap();;
        val = val.unsigned_shl(7) | data;

        i += 1;

        if !msb {
            return (i, val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use suppositions::generators::*;
    use suppositions::*;

    macro_rules! test_encode {
        ($name: ident, $expected: expr, $input: expr) => {
            #[test]
            fn $name() {
                assert_eq!(&$expected, &*encode_vec($input));
            }
        };
    }

    macro_rules! test_decode {
        ($name: ident, $input: expr, $expected: expr) => {
            #[test]
            fn $name() {
                let (_sz, n) = decode_slice(&$input);
                assert_eq!($expected, n);
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

    test_decode!(decode_0x00000000u8, [0], 0u8);
    test_decode!(decode_0x0000007fu8, [0x7fu8], 0x0000007Fu8);
    test_decode!(decode_0x0000007fu64, [0x7fu8], 0x0000007Fu64);
    test_decode!(decode_0x00000080u8, [0x81, 0x0], 0x00000080u8);
    test_decode!(decode_0x00000080u16, [0x81, 0x0], 0x00000080u16);
    test_decode!(decode_0x00002000u16, [0xc0u8, 0x0], 0x00002000u16);
    test_decode!(decode_0x00003fffu16, [0xFF, 0x7F], 0x00003FFFu16);
    test_decode!(decode_0x0000400016, [0x81, 0x80, 0x00], 0x00004000u16);
    test_decode!(decode_0x001fffffu32, [0xFF, 0xFF, 0x7F], 0x001FFFFFu32);
    test_decode!(
        decode_0x00200000u32,
        [0x81, 0x80, 0x80, 0x00],
        0x00200000u32
    );
    test_decode!(
        decode_0x08000000u32,
        [0xC0, 0x80, 0x80, 0x00],
        0x08000000u32
    );
    test_decode!(
        decode_0x0fffffffu32,
        [0xFF, 0xFF, 0xFF, 0x7F],
        0x0FFFFFFFu32
    );

    #[test]
    fn should_round_trip_u8() {
        property(u8s()).check(|v| {
            let bs = encode_vec(v);
            let (_, v2) = decode_slice(&bs);
            assert_eq!(v, v2, "Slice: {:?}", bs);
        });
    }

    #[test]
    fn should_round_trip_u16() {
        property(u16s()).check(|v| {
            let bs = encode_vec(v);
            let (_, v2) = decode_slice(&bs);
            assert_eq!(v, v2, "Slice: {:?}", bs);
        });
    }
    #[test]
    fn should_round_trip_u32() {
        property(u32s()).check(|v| {
            let bs = encode_vec(v);
            let (_, v2) = decode_slice(&bs);
            assert_eq!(v, v2, "Slice: {:?}", bs);
        });
    }
    #[test]
    fn should_round_trip_u64s() {
        property(u64s()).check(|v| {
            let bs = encode_vec(v);
            let (_, v2) = decode_slice(&bs);
            assert_eq!(v, v2, "Slice: {:?}", bs);
        });
    }

    #[test]
    fn should_return_encoded_length_u8s() {
        property(u8s()).check(|v| {
            let bs = encode_vec(v);
            let (sz, _) = decode_slice::<u8>(&bs);
            assert_eq!(bs.len(), sz, "Slice: {:?}", bs);
        });
    }
    #[test]
    fn should_return_encoded_length_u16s() {
        property(u16s()).check(|v| {
            let bs = encode_vec(v);
            let (sz, _) = decode_slice::<u16>(&bs);
            assert_eq!(bs.len(), sz, "Slice: {:?}", bs);
        });
    }
    #[test]
    fn should_return_encoded_length_u32s() {
        property(u32s()).check(|v| {
            let bs = encode_vec(v);
            let (sz, _) = decode_slice::<u32>(&bs);
            assert_eq!(bs.len(), sz, "Slice: {:?}", bs);
        });
    }
    #[test]
    fn should_return_encoded_length_u64s() {
        property(u64s()).check(|v| {
            let bs = encode_vec(v);
            let (sz, _) = decode_slice::<u64>(&bs);
            assert_eq!(bs.len(), sz, "Slice: {:?}", bs);
        });
    }
}
