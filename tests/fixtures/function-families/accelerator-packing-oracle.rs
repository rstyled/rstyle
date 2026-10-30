#[cfg(test)]
mod rstyle_accelerator_packing_oracle {
    use super::Accel;

    fn expected(bytes: &[u8]) -> u32 {
        bytes
            .iter()
            .enumerate()
            .map(|(index, byte)| {
                let position = if cfg!(target_endian = "little") {
                    index
                } else {
                    3 - index
                };
                u32::from(*byte) * (1u32 << (8 * position))
            })
            .sum()
    }

    fn check(bytes: [u8; 8]) {
        let actual = Accel { bytes }.as_accel_tys();
        assert_eq!(actual, [expected(&bytes[0..4]), expected(&bytes[4..8])]);
    }

    #[test]
    fn all_header_payload_and_padding_bytes_contribute_with_native_weights() {
        for fill in [0, 1, 127, 255] {
            for position in 0..8 {
                for byte in 0..=255 {
                    let mut bytes = [fill; 8];
                    bytes[position] = byte;
                    check(bytes);
                }
            }
        }
        for bit in 0..64 {
            let mut bytes = [0; 8];
            bytes[bit / 8] = 1 << (bit % 8);
            check(bytes);
        }
        check([0, 1, 2, 3, 4, 5, 6, 7]);
        check([255, 254, 253, 252, 251, 250, 249, 248]);
    }
}
