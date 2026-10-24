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

#[test]
fn checked_get_saves_receiver_before_evaluating_its_index() {
    let first = [7u8];
    let second = [41u8, 43];
    for index in [0, 1, 2, usize::MAX] {
        let mut selected = first.as_slice();
        let mut order = Vec::new();
        let saved = {
            order.push("receiver");
            selected
        }
        .get({
            order.push("index");
            selected = second.as_slice();
            index
        });
        assert_eq!(order, ["receiver", "index"]);
        assert_eq!(selected, second);
        assert_eq!(saved.copied(), if index == 0 { Some(7) } else { None });
        if let Some(byte) = saved {
            assert!(std::ptr::eq(byte, &first[0]));
        }
    }
}
