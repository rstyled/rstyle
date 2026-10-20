// Runtime expectations specified independently from extraction and matching.
#[cfg(test)]
mod byte_comparison_oracle {
    use super::*;

    #[test]
    fn byte_queries_and_eager_defaults() {
        for actual in 0..=u8::MAX {
            for expected in 0..=u8::MAX {
                let equal = actual == expected;
                assert_eq!(byte_matches(actual, expected), equal);
                assert_eq!(saved_comparison(actual, expected), equal);
                assert_eq!(local_option(actual, expected), equal);
                assert_eq!(default_before_capture(actual, expected), actual == 1);
                assert!(none_uses_default(expected));
            }
        }
    }

    #[test]
    fn all_sentinel_payloads_remain_distinct_from_bytes() {
        for end in 0..=u16::MAX {
            for expected in [0, 1, 127, 255] {
                assert!(!end_never_matches(end, expected));
            }
        }
    }
}
