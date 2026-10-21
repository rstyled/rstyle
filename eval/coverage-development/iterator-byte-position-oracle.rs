#[cfg(test)]
mod rstyle_iterator_byte_position_oracle {
    use super::Accel;
    use std::vec::Vec;

    fn membership(bytes: [u8; 8]) {
        let accel = Accel { bytes };
        let mut expected = [0u128; 2];
        for index in 1..=usize::from(bytes[0]) {
            let byte = usize::from(bytes[index]);
            expected[byte / 128] |= 1u128 << (byte % 128);
        }
        for query in 0..=u8::MAX {
            let byte = usize::from(query);
            assert_eq!(
                accel.contains(query),
                expected[byte / 128] & (1u128 << (byte % 128)) != 0,
                "bytes={bytes:?}, query={query}"
            );
        }
    }

    #[test]
    fn only_active_payload_bytes_count_including_duplicates_and_every_query() {
        for length in 0..=7 {
            for seed in 0..=u8::MAX {
                let mut bytes = [length, 0, 0, 0, 0, 0, 0, 0];
                for (index, byte) in bytes.iter_mut().enumerate().skip(1) {
                    *byte = seed.wrapping_add((index as u8).wrapping_mul(37));
                }
                membership(bytes);
                membership([length, seed, seed, seed, seed, seed, seed, seed]);
            }
        }
        membership([0, 1, 2, 3, 4, 5, 6, 7]);
        membership([1, 9, 255, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn invalid_header_panics_before_returning_any_membership_result() {
        for length in 8..=u8::MAX {
            let accel = Accel {
                bytes: [length, 17, 17, 17, 17, 17, 17, 17],
            };
            for query in [0, 17, 255] {
                assert!(std::panic::catch_unwind(|| accel.contains(query)).is_err());
            }
        }
    }

    #[test]
    fn pinned_position_visits_only_the_prefix_through_the_first_match() {
        for bytes in [&[][..], &[7][..], &[7, 9, 7, 3, 255, 9][..]] {
            let mut expected = [None; 256];
            for index in (0..bytes.len()).rev() {
                expected[usize::from(bytes[index])] = Some(index);
            }
            for query in 0..=u8::MAX {
                let mut visited = Vec::new();
                let actual = bytes.iter().position(|&byte| {
                    visited.push(byte);
                    byte == query
                });
                assert_eq!(actual, expected[usize::from(query)]);
                let end = actual.map_or(bytes.len(), |index| index + 1);
                assert_eq!(visited, bytes[..end]);
            }
        }
    }
}
