// Appended inside the unchanged original util::wire module; no copied functions.
#[doc(hidden)]
pub(crate) fn __rstyle_wire_arithmetic_oracle() -> (u64, u64, u64) {
    fn error(error: DeserializeError, expected: &'static str) {
        match error.0 {
            DeserializeErrorKind::ArithmeticOverflow { what } => {
                assert_eq!(what.len(), expected.len());
                assert_eq!(what.as_ptr(), expected.as_ptr());
            }
            _ => panic!("wrong original error variant"),
        }
    }
    fn case(a: usize, b: usize, counts: &mut [u64; 3]) {
        for what in ["", "wire addition", "wire multiplication"] {
            for (actual, mathematical) in [
                (add(a, b, what), (a as u128) + (b as u128)),
                (mul(a, b, what), (a as u128) * (b as u128)),
            ] {
                if mathematical <= usize::MAX as u128 {
                    assert_eq!(actual.unwrap(), mathematical as usize);
                    counts[1] += 1;
                } else {
                    error(actual.unwrap_err(), what);
                    counts[2] += 1;
                }
                counts[0] += 1;
            }
        }
    }
    assert_eq!(usize::BITS, 64);
    let mut counts = [0; 3];
    for what in ["", "first static message", "second static message"] {
        error(DeserializeError::arithmetic_overflow(what), what);
        counts[0] += 1;
    }
    for a in 0..=255 {
        for b in 0..=255 {
            case(a, b, &mut counts);
        }
    }
    let edge = [
        0,
        1,
        2,
        3,
        255,
        256,
        65535,
        65536,
        u32::MAX as usize,
        (u32::MAX as usize) + 1,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
        usize::MAX - 2,
        usize::MAX - 1,
        usize::MAX,
    ];
    for a in edge {
        for b in edge {
            case(a, b, &mut counts);
        }
    }
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..8192 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = state as usize;
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        case(a, state as usize, &mut counts);
    }
    (counts[0], counts[1], counts[2])
}
