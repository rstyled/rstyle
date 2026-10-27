//! Independent value and borrowing expectations over the unchanged public API.
use lz4_flex::block::{uncompressed_size, DecompressError};

fn check(input: &[u8]) {
    let observed = uncompressed_size(input);
    if input.len() < 4 {
        assert!(matches!(
            observed,
            Err(DecompressError::ExpectedAnotherByte)
        ));
        return;
    }
    // Arithmetic specification deliberately avoids from_le_bytes and try_into.
    let expected = input[..4]
        .iter()
        .enumerate()
        .map(|(lane, byte)| usize::from(*byte) * 256usize.pow(lane as u32))
        .sum::<usize>();
    let (size, rest) = observed.expect("a complete length header must succeed");
    assert_eq!(size, expected, "header={:?}", &input[..4]);
    assert_eq!(rest, &input[4..]);
    assert_eq!(rest.len(), input.len() - 4);
    assert_eq!(
        rest.as_ptr(),
        input[4..].as_ptr(),
        "rest must borrow the original suffix"
    );
}

fn main() {
    let mut cases = 0u64;
    for length in 0..4 {
        for byte in 0..=255u16 {
            check(&vec![byte as u8; length]);
            cases += 1;
        }
    }
    // Distinguish every byte value and each little-endian lane independently.
    for lane in 0..4 {
        for byte in 0..=255u16 {
            for trailing in [0, 1, 4, 17] {
                let mut input = vec![0xa5; 4 + trailing];
                input[..4].fill(0);
                input[lane] = byte as u8;
                check(&input);
                cases += 1;
            }
        }
    }
    for header in [
        [0, 0, 0, 0],
        [1, 0, 0, 0],
        [255, 0, 0, 0],
        [0, 1, 0, 0],
        [255, 255, 0, 0],
        [0, 0, 1, 0],
        [255, 255, 255, 127],
        [0, 0, 0, 128],
        [255, 255, 255, 255],
        [1, 2, 3, 4],
    ] {
        for trailing in 0..=64 {
            let mut input = header.to_vec();
            input.extend((0..trailing).map(|position| (position * 37) as u8));
            check(&input);
            cases += 1;
        }
    }
    // Offset the input within backing storage: retain the selected slice origin.
    for offset in 0..=16 {
        let mut storage = vec![0xcc; offset];
        storage.extend([0x78, 0x56, 0x34, 0x12, 9, 8, 7]);
        check(&storage[offset..]);
        cases += 1;
    }
    println!("{{\"cases\":{cases},\"mismatches\":0,\"scope\":\"unchanged public library native execution\"}}");
}
