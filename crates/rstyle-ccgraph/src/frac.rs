//! Exact rational arithmetic for threshold comparisons.
//!
//! Decimal configuration lexemes are treated as exact rationals, and every
//! gate compares unrounded values. Overflow is a diagnostic error, never a
//! silent wrap or saturation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArithError {
    pub what: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frac {
    num: i128,
    den: i128,
}

impl Frac {
    pub fn new(num: i128, den: i128) -> Result<Self, ArithError> {
        if den == 0 {
            return Err(ArithError {
                what: "zero denominator",
            });
        }
        let overflow = || ArithError {
            what: "rational overflow",
        };
        // Reduce by gcd in unsigned space so i128::MIN never needs negation.
        let num_abs = num.unsigned_abs();
        let den_abs = den.unsigned_abs();
        let divisor = gcd(num_abs, den_abs);
        // divisor >= 1; exact division in u128.
        let num_q = num_abs / divisor;
        let den_q = den_abs / divisor;
        // Value sign: negative iff exactly one part is negative.
        // Thresholds, counts and scores are nonnegative in practice; the
        // checked paths below only guard against wrapping on extremes.
        let negative = (num < 0) != (den < 0);
        let den = i128::try_from(den_q).map_err(|_| overflow())?;
        // A negative numerator may legally be i128::MIN, whose absolute
        // value (2^127) does not fit as a positive i128.
        let num = if negative {
            if num_q == i128::MIN.unsigned_abs() {
                i128::MIN
            } else {
                i128::try_from(num_q)
                    .map_err(|_| overflow())?
                    .checked_neg()
                    .ok_or_else(overflow)?
            }
        } else {
            i128::try_from(num_q).map_err(|_| overflow())?
        };
        // Denominator is positive by construction; num_q/den_q already reduced
        // because the common divisor was removed before applying the sign.
        debug_assert!(den > 0);
        Ok(Self { num, den })
    }

    pub fn zero() -> Self {
        Self { num: 0, den: 1 }
    }

    pub fn one() -> Self {
        Self { num: 1, den: 1 }
    }

    pub fn numerator(&self) -> i128 {
        self.num
    }

    pub fn denominator(&self) -> i128 {
        self.den
    }

    /// Parse a finite decimal lexeme such as `0.8` or `1` exactly.
    pub fn parse_decimal(lexeme: &str) -> Result<Self, ArithError> {
        let text = lexeme.trim();
        if text.is_empty() {
            return Err(ArithError {
                what: "empty decimal",
            });
        }

        let (digits, scale) = split_decimal(text)?;
        let num: i128 = digits.parse().map_err(|_| ArithError {
            what: "bad decimal digits",
        })?;
        let den = pow10(scale)?;

        Self::new(num, den)
    }

    pub fn checked_add(self, other: Self) -> Result<Self, ArithError> {
        let num = self
            .num
            .checked_mul(other.den)
            .and_then(|v| {
                other
                    .num
                    .checked_mul(self.den)
                    .and_then(|w| v.checked_add(w))
            })
            .ok_or(ArithError {
                what: "rational overflow",
            })?;
        let den = self.den.checked_mul(other.den).ok_or(ArithError {
            what: "rational overflow",
        })?;
        Self::new(num, den)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, ArithError> {
        let num = self.num.checked_mul(other.num).ok_or(ArithError {
            what: "rational overflow",
        })?;
        let den = self.den.checked_mul(other.den).ok_or(ArithError {
            what: "rational overflow",
        })?;
        Self::new(num, den)
    }

    pub fn checked_div(self, other: Self) -> Result<Self, ArithError> {
        if other.num == 0 {
            return Err(ArithError {
                what: "division by zero",
            });
        }
        let num = self.num.checked_mul(other.den).ok_or(ArithError {
            what: "rational overflow",
        })?;
        let den = self.den.checked_mul(other.num).ok_or(ArithError {
            what: "rational overflow",
        })?;
        Self::new(num, den)
    }

    /// Exact inclusive comparison `self >= other`, overflow-checked.
    pub fn ge(self, other: Self) -> Result<bool, ArithError> {
        let left = self.num.checked_mul(other.den).ok_or(ArithError {
            what: "rational overflow",
        })?;
        let right = other.num.checked_mul(self.den).ok_or(ArithError {
            what: "rational overflow",
        })?;
        Ok(left >= right)
    }

    /// Display-only approximation; never used for gate decisions.
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

/// Checked `u128` to `i128` narrowing for exact score arithmetic.
pub fn as_i128(value: u128) -> Result<i128, ArithError> {
    i128::try_from(value).map_err(|_| ArithError {
        what: "value exceeds i128",
    })
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    if a == 0 { 1 } else { a }
}

fn split_decimal(text: &str) -> Result<(String, u32), ArithError> {
    let bad = ArithError {
        what: "bad decimal shape",
    };
    let (head, tail) = match text.split_once('.') {
        Some(pair) => pair,
        None => (text, ""),
    };
    if head.is_empty() || !head.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad);
    }
    if !tail.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad);
    }
    if text.contains(['+', '-', 'e', 'E', ' ', '\t']) {
        return Err(bad);
    }

    let scale: u32 = tail.len().try_into().map_err(|_| bad)?;

    Ok((format!("{head}{tail}"), scale))
}

fn pow10(scale: u32) -> Result<i128, ArithError> {
    let mut out: i128 = 1;
    for _ in 0..scale {
        out = out.checked_mul(10).ok_or(ArithError {
            what: "decimal overflow",
        })?;
    }
    Ok(out)
}
