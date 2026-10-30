//! Appended to the development corpus and exact natural gcd by the CLI test.
//! The small-input oracle enumerates divisors instead of using Euclid's method.

#[test]
fn divisor_enumeration_specifies_complete_reductions() {
    for a in 0..32u128 {
        for b in 0..32u128 {
            let expected = if a == 0 && b == 0 {
                0
            } else {
                (1..=a.max(b))
                    .filter(|divisor| a % divisor == 0 && b % divisor == 0)
                    .max()
                    .unwrap()
            };
            assert_eq!(sequential(a, b), expected);
            assert_eq!(cyclic_merge(a, b), expected);
            assert_eq!(after_loop(a, b), expected.max(1));
            assert_eq!(before_loop(a, b), expected.max(1));
            assert_eq!(gcd(a, b), expected.max(1));
            if b != 0 {
                assert_eq!(tuple_step(a, b), (b, a % b));
            }
        }
    }
}

#[test]
fn wide_values_and_simultaneous_assignment_preserve_old_operands() {
    for (a, b, expected) in [
        (u128::MAX, u128::MAX, u128::MAX),
        (u128::MAX, u128::MAX - 1, 1),
        (1u128 << 127, 1u128 << 126, 1u128 << 126),
        (0, u128::MAX, u128::MAX),
    ] {
        assert_eq!(sequential(a, b), expected);
        assert_eq!(after_loop(a, b), expected);
        assert_eq!(before_loop(a, b), expected);
        assert_eq!(gcd(a, b), expected);
        assert_eq!(tuple_step(a, b), (b, a % b));
    }
    assert_eq!(tuple_step(17, 5), (5, 2));
    assert!(std::panic::catch_unwind(|| tuple_step(17, 0)).is_err());
    let mut value = 17;
    memory_target(&mut value, 5);
    assert_eq!(value, 5);
}
