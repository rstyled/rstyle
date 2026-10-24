// Append tests to the original accel module or the development fixture.
#[cfg(test)]
mod rstyle_borrowed_accel_oracle {
    use super::*;

    #[test]
    fn valid_views_keep_original_bytes_and_addresses() {
        for seed in 0..=u8::MAX {
            for length in 0..=7u8 {
                let bytes = [length, seed, 1, 3, 5, 7, 11, 13];
                let accel = Accel { bytes };
                assert_eq!(accel.len(), usize::from(length));
                assert_eq!(accel.is_empty(), length == 0);
                let actual = accel.needles();
                assert_eq!(actual.len(), usize::from(length));
                assert_eq!(actual.as_ptr(), accel.bytes.as_ptr().wrapping_add(1));
                for (index, byte) in actual.iter().enumerate() {
                    assert_eq!(*byte, bytes[index + 1]);
                }
            }
        }
    }

    #[test]
    fn invalid_metadata_still_reports_length_and_panics_on_view() {
        for length in 8..=u8::MAX {
            let accel = Accel {
                bytes: [length, 17, 19, 23, 29, 31, 37, 41],
            };
            assert_eq!(accel.len(), usize::from(length));
            assert!(!accel.is_empty());
            assert!(std::panic::catch_unwind(|| accel.needles()).is_err());
        }
    }
}
