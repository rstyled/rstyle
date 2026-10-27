//! Closed nonnegative intervals; absent or overflowing endpoints remain unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Interval {
    lower: u128,
    upper: u128,
}

impl Interval {
    pub(super) fn byte_digit() -> Self {
        Self {
            lower: u128::from(b'0'),
            upper: u128::from(b'9'),
        }
    }

    pub(super) fn constant(value: i128) -> Option<Self> {
        let value = u128::try_from(value).ok()?;
        Some(Self {
            lower: value,
            upper: value,
        })
    }

    pub(super) fn binary(self, operator: &str, other: Self) -> Option<Self> {
        let (lower, upper) = match operator {
            "+" => (
                self.lower.checked_add(other.lower)?,
                self.upper.checked_add(other.upper)?,
            ),
            "-" => (
                self.lower.checked_sub(other.upper)?,
                self.upper.checked_sub(other.lower)?,
            ),
            "*" => (
                self.lower.checked_mul(other.lower)?,
                self.upper.checked_mul(other.upper)?,
            ),
            _ => return None,
        };
        Some(Self { lower, upper })
    }

    pub(super) fn fits(self, ty: &str, width: Option<u32>) -> bool {
        let bits = match ty {
            "u8" => 8,
            "u16" => 16,
            "u32" => 32,
            "u64" => 64,
            "u128" => 128,
            "usize" => match width {
                Some(width) => width,
                None => return false,
            },
            _ => return false,
        };
        match bits {
            1..=127 => self.upper < (1u128 << bits),
            128 => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_bounds_are_independent_of_the_observed_sample() {
        let digit = Interval::byte_digit()
            .binary("-", Interval::constant(48).unwrap())
            .unwrap();
        assert_eq!(digit, Interval { lower: 0, upper: 9 });
        let two = digit
            .binary("*", Interval::constant(10).unwrap())
            .unwrap()
            .binary("+", digit)
            .unwrap();
        assert_eq!(
            two,
            Interval {
                lower: 0,
                upper: 99
            }
        );
        let three = two
            .binary("*", Interval::constant(10).unwrap())
            .unwrap()
            .binary("+", digit)
            .unwrap();
        assert_eq!(
            three,
            Interval {
                lower: 0,
                upper: 999
            }
        );
        assert!(two.fits("u8", None));
        assert!(!three.fits("u8", None));
        assert!(three.fits("u16", None));
    }

    #[test]
    fn underflow_overflow_signed_and_unknown_width_stay_unproved() {
        assert!(
            Interval::constant(47)
                .unwrap()
                .binary("-", Interval::constant(48).unwrap())
                .is_none()
        );
        assert!(
            Interval {
                lower: 0,
                upper: u128::MAX
            }
            .binary("+", Interval::constant(1).unwrap())
            .is_none()
        );
        assert!(!Interval::byte_digit().fits("i8", None));
        assert!(!Interval::byte_digit().fits("usize", None));
        assert!(Interval::constant(-1).is_none());
    }
}
