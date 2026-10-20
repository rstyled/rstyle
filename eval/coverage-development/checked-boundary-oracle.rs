// Appended to the exact source-selected natural leaf and development cases.

#[test]
fn valid_unicode_boundaries_follow_the_standard_string_oracle() {
    for text in ["", "ascii", "é", "€", "🦀", "aé€🦀z", "\0\r\n"] {
        for index in (0..=text.len() + 2).chain([usize::MAX]) {
            assert_eq!(
                is_boundary(text.as_bytes(), index),
                text.is_char_boundary(index)
            );
            assert_eq!(
                helper_boundary(text.as_bytes(), index),
                text.is_char_boundary(index)
            );
        }
    }
}

#[test]
fn checked_payloads_keep_saved_origins_and_no_access_on_none() {
    for byte in 0..=255u8 {
        let first = [byte];
        let second = [17, 23, 31];
        assert_eq!(checked_byte(&first, 0), byte);
        assert_eq!(saved_byte(&first, &second, 0), byte);
        for index in [1, 2, usize::MAX] {
            assert_eq!(checked_byte(&first, index), 0);
            assert_eq!(saved_byte(&first, &second, index), 3);
        }
    }
    let mut bytes = [5];
    assert_eq!(mutable_byte(&mut bytes, 0), 42);
    assert_eq!(bytes, [42]);
    assert_eq!(mutable_byte(&mut bytes, usize::MAX), 0);
}
