#![feature(test)]

extern crate rustbucks_vlq;
extern crate test;

#[cfg(test)]
mod tests {

    use rustbucks_vlq::*;
    use test::Bencher;

    macro_rules! bench_encode {
        ($name: ident, $val: expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                let mut buf = [0u8; 20];
                b.iter(|| encode_slice($val, &mut buf));
            }
        };
    }

    bench_encode!(encode_0u8, 0x0u8);
    bench_encode!(encode_0u64, 0x0u64);
    bench_encode!(encode_127u8, 0x7fu8);
    bench_encode!(encode_127u64, 0x7fu64);
    bench_encode!(encode_128u64, 0x80u8);

    bench_encode!(encode_example_128u8, 128u8);
    bench_encode!(encode_example_128u16, 128u16);
    bench_encode!(encode_example_8192u16, 8192u16);
    bench_encode!(encode_example_16383u16, 16383u16);
    bench_encode!(encode_example_16384u16, 16384u16);

    macro_rules! bench_decode {
        ($name: ident, $t: path, $val: expr) => {
            #[bench]
            fn $name(b: &mut Bencher) {
                b.iter(|| {
                    let (_, n) = decode_slice::<$t>(&$val);
                    n
                });
            }
        };
    }

    bench_decode!(decode_0u8, u8, [0]);
    bench_decode!(decode_0u64, u64, [0x7fu8]);
    bench_decode!(decode_127u8, u8, [0x7fu8]);
    bench_decode!(decode_127u64, u8, [0x7fu8]);
    bench_decode!(decode_128u64, u8, [0x81, 0x0]);
    bench_decode!(decode_128u8, u16, [0x81, 0x0]);
    bench_decode!(decode_128u16, u16, [0x81, 0x0]);
    bench_decode!(decode_8192u16, u16, [0xc0u8, 0x0]);
    bench_decode!(decode_16383u16, u16, [0xFF, 0x7F]);
    bench_decode!(decode_16384u16, u16, [0x81, 0x80, 0x00]);
    bench_decode!(decode_16384u32, u32, [0x81, 0x80, 0x00]);
    bench_decode!(decode_16384u64, u64, [0x81, 0x80, 0x00]);
}
