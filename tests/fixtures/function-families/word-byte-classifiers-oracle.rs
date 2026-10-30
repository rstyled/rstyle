#[cfg(test)]
mod rstyle_word_byte_classifiers_oracle {
    use super::{Unit, UnitKind};
    use core::sync::atomic::{AtomicUsize, Ordering};

    // Independent bitset for the specified ASCII alphabet. No generated table,
    // const builder, callback lowering or captured graph supplies these values.
    const ASCII_WORD: [u64; 4] = [0x03ff000000000000, 0x07fffffe87fffffe, 0, 0];

    fn expected(byte: u8) -> bool {
        ASCII_WORD[usize::from(byte) / 64] & (1u64 << (byte % 64)) != 0
    }

    #[test]
    fn complete_original_table_and_tagged_caller_classify_every_byte() {
        for byte in 0..=u8::MAX {
            let word = expected(byte);
            assert_eq!(crate::util::utf8::is_word_byte(byte), word, "{byte}");
            assert_eq!(Unit::u8(byte).is_word_byte(), word, "{byte}");
            assert_eq!(Unit::u8(byte).as_u8(), Some(byte));
        }
    }

    #[test]
    fn every_sentinel_payload_remains_distinct_from_a_byte() {
        for sentinel in 0..=u16::MAX {
            let unit = Unit(UnitKind::EOI(sentinel));
            assert_eq!(unit.as_u8(), None);
            assert!(!unit.is_word_byte(), "sentinel={sentinel}");
        }
        for sentinel in 0..=256 {
            assert!(!Unit::eoi(sentinel).is_word_byte());
        }
    }

    static ORDER: AtomicUsize = AtomicUsize::new(0);

    fn default() -> bool {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 0);
        true
    }

    fn callback(byte: u8) -> bool {
        assert_eq!(ORDER.fetch_add(1, Ordering::SeqCst), 1);
        expected(byte)
    }

    #[test]
    fn named_callback_follows_eager_default_once_only_on_some() {
        for byte in 0..=u8::MAX {
            ORDER.store(0, Ordering::SeqCst);
            assert_eq!(Some(byte).map_or(default(), callback), expected(byte));
            assert_eq!(ORDER.load(Ordering::SeqCst), 2);
        }
        ORDER.store(0, Ordering::SeqCst);
        assert!(None::<u8>.map_or(default(), callback));
        assert_eq!(ORDER.load(Ordering::SeqCst), 1);
    }
}
