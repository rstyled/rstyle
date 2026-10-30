//! Independent execution expectations for the declared lz4 byte-window family.
//! Run with pinned rustc --edition=2021 --test, with overflow checks enabled.
//! These tests specify semantics; passing does not establish rstyle admission.

#[path = "arch-readers.rs"]
mod architecture;

fn get_batch(input: &[u8], n: usize) -> u32 {
    u32::from_ne_bytes(input[n..n + 4].try_into().unwrap())
}

fn get_batch_arch(input: &[u8], n: usize) -> usize {
    const USIZE_SIZE: usize = core::mem::size_of::<usize>();
    let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE].try_into().unwrap();
    usize::from_ne_bytes(*arr)
}

// Arithmetic specification deliberately avoids from_*_bytes and TryInto.
fn expected(bytes: &[u8]) -> u64 {
    bytes.iter().enumerate().fold(0, |value, (index, byte)| {
        let significance = if cfg!(target_endian = "little") {
            index
        } else {
            bytes.len() - index - 1
        };
        value | (u64::from(*byte) << (8 * significance))
    })
}

#[test]
fn every_byte_contributes_at_each_window_offset() {
    let width = core::mem::size_of::<usize>();
    assert!(matches!(width, 4 | 8));
    for start in 0..4 {
        for position in 0..width {
            for byte in [1, 2, 0x80, 0xff] {
                let mut input = vec![0; start + width + 3];
                input[start + position] = byte;
                assert_eq!(
                    get_batch_arch(&input, start) as u64,
                    expected(&input[start..start + width])
                );
                assert_eq!(
                    u64::from(get_batch(&input, start)),
                    expected(&input[start..start + 4])
                );
            }
        }
    }
}

#[test]
fn surrounding_bytes_do_not_contribute_and_endian_is_observable() {
    let mut input = [19, 23, 1, 2, 3, 4, 5, 6, 7, 8, 29, 31];
    let width = core::mem::size_of::<usize>();
    let scalar = get_batch(&input, 2);
    let native = get_batch_arch(&input, 2);
    assert_ne!(scalar, get_batch(&input, 1));
    assert_ne!(scalar, scalar.swap_bytes());
    assert_ne!(native, native.swap_bytes());
    input[..2].fill(255);
    input[2 + width..].fill(255);
    assert_eq!(get_batch(&input, 2), scalar);
    assert_eq!(get_batch_arch(&input, 2), native);
    input[6..].fill(0);
    assert_eq!(get_batch(&input, 2), scalar);
}

#[test]
fn wrong_length_is_a_result_failure_before_unwrap() {
    for length in [0, 3, 5, 9] {
        let input = vec![0u8; length];
        let copied: Result<[u8; 4], _> = input.as_slice().try_into();
        let borrowed: Result<&[u8; 4], _> = input.as_slice().try_into();
        assert!(copied.is_err());
        assert!(borrowed.is_err());
        assert!(std::panic::catch_unwind(|| copied.unwrap()).is_err());
        assert!(std::panic::catch_unwind(|| borrowed.unwrap()).is_err());
    }
    let input = [11u8, 12, 13, 14];
    let borrowed: &[u8; 4] = input.as_slice().try_into().unwrap();
    assert_eq!(borrowed.as_ptr(), input.as_ptr());
}

#[test]
fn invalid_windows_and_endpoint_overflow_fail() {
    let input = [0u8; 8];
    for start in [6, 9, usize::MAX - 1, usize::MAX] {
        assert!(std::panic::catch_unwind(|| get_batch(&input, start)).is_err());
        assert!(std::panic::catch_unwind(|| get_batch_arch(&input, start)).is_err());
    }
}

#[test]
fn architecture_compositions_preserve_values_and_saved_base() {
    let input = [19u8, 23, 1, 2, 3, 4, 5, 6, 7, 8, 29, 31];
    let other = [255u8; 12];
    let width = core::mem::size_of::<usize>();
    for start in 0..=input.len() - width {
        let value = expected(&input[start..start + width]) as usize;
        assert_eq!(architecture::batch_arch(&input, start), value);
        assert_eq!(architecture::saved_arch(&input, start), value);
        assert_eq!(architecture::result_local_arch(&input, start), value);
        assert_eq!(architecture::caller_arch(&input, start), value);
    }
    let (snapshot, selected) = architecture::snapshot_arch(&input, &other);
    assert_eq!(snapshot, expected(&input[..width]) as usize);
    assert_ne!(snapshot, expected(&other[..width]) as usize);
    assert_eq!(selected.as_ptr(), other.as_ptr());
}

#[test]
fn architecture_borrow_preserves_offset_identity_and_failure() {
    const WIDTH: usize = core::mem::size_of::<usize>();
    let input = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let view = &input[2..2 + WIDTH];
    let result: Result<&[u8; WIDTH], _> = view.try_into();
    let arr = result.unwrap();
    assert_eq!(arr.as_ptr(), view.as_ptr());
    assert_ne!(arr.as_ptr(), input.as_ptr());
    assert_eq!(arr.as_slice(), view);
    for length in [0, WIDTH - 1, WIDTH + 1] {
        let result: Result<&[u8; WIDTH], _> = input[..length].try_into();
        assert!(result.is_err());
    }
    assert!(std::panic::catch_unwind(|| architecture::wrong_length_arch(&input, 0)).is_err());
}

#[test]
fn architecture_negative_controls_are_semantically_distinguishable() {
    let input = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let native = architecture::batch_arch(&input, 0);
    // The intended admission contract is native little-endian aarch64 only.
    const {
        assert!(cfg!(target_endian = "little"));
        assert!(core::mem::size_of::<usize>() == 8);
    }
    assert_ne!(architecture::endian_arch(&input, 0), native);
    // Equal values do not authorize a different constant initializer authority.
    assert_eq!(architecture::other_size_authority_arch(&input, 0), native);
}
