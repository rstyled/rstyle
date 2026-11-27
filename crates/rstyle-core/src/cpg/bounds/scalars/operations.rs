use super::Scalar;
use crate::runtime_bounds::Integer;
use crate::runtime_bounds::contracts::ScalarProfile;
use std::cmp::Ordering;

/// Reduce modulo 2^bits and sign-extend when the target integer is signed. The
/// frontend must have specified one of the modeled widths (8, 16, 32, or 64).
pub(super) fn narrow(n: i128, ty: Integer) -> i128 {
    let modulus = 1i128 << ty.bits;
    let raw = n & (modulus - 1);
    if ty.signed && raw >= modulus / 2 {
        raw - modulus
    } else {
        raw
    }
}

fn compare(order: Ordering, op: &str) -> Option<Scalar> {
    Some(Scalar::Bool(match op {
        "eq" => order.is_eq(),
        "ne" => order.is_ne(),
        "lt" => order.is_lt(),
        "le" => order.is_le(),
        "gt" => order.is_gt(),
        "ge" => order.is_ge(),
        _ => return None,
    }))
}

fn nil_comparison(val: &Scalar, op: &str) -> Option<Scalar> {
    if op != "eq" && op != "ne" {
        return None;
    }
    let nil = match val {
        Scalar::Nil | Scalar::NilFunction => true,
        Scalar::Pointer(v) => v.is_none(),
        Scalar::Map(v) | Scalar::Channel(v) => v.is_none(),
        Scalar::Slice(view) => *view == super::storage::View::nil(),
        Scalar::Callable(_) | Scalar::BoundMethod(_, _) | Scalar::ClosureValue(_) => false,
        _ => return None,
    };
    Some(Scalar::Bool(nil == (op == "eq")))
}

pub(super) fn apply(
    profile: &ScalarProfile,
    op: &str,
    (a, b): (Scalar, Option<Scalar>),
    (width, runtime): (Option<Integer>, bool),
) -> Option<Scalar> {
    use Scalar::*;
    match (a, b) {
        (Pointer(a), Some(Pointer(b)))
            if (op == "eq" || op == "ne") && (a == b || a.is_none() || b.is_none()) =>
        {
            Some(Bool((a == b) == (op == "eq")))
        }
        (Channel(a), Some(Channel(b))) if op == "eq" || op == "ne" => {
            Some(Bool((a == b) == (op == "eq")))
        }
        (v, Some(Nil)) | (Nil, Some(v)) => nil_comparison(&v, op),
        (Int(a), None) => Some(Int(match op {
            "neg" => a.checked_neg()?,
            "bit_not" if width.is_some_and(|t| !t.signed) => narrow(!a, width?),
            "bit_not" => !a,
            op if op == profile.unary_plus => a,
            _ => return None,
        })),
        (Bool(a), None) if op == "not" => Some(Bool(!a)),
        (Bool(a), Some(Bool(b))) => Some(Bool(match op {
            "eq" => a == b,
            "ne" => a != b,
            "and" => a && b,
            "or" => a || b,
            _ => return None,
        })),
        (Text(mut a), Some(Text(b))) if op == "add" && a.len() + b.len() <= 4096 => {
            a.extend(b);
            Some(Text(a))
        }
        (Text(a), Some(Text(b))) => compare(a.cmp(&b), op),
        (Int(a), Some(Int(b))) => integers(op, (a, b), width, runtime),
        _ => None,
    }
}

fn integers(
    op: &str,
    (a, b): (i128, i128),
    width: Option<Integer>,
    runtime: bool,
) -> Option<Scalar> {
    use Scalar::Int;

    if let Some(value) = compare(a.cmp(&b), op) {
        return Some(value);
    }
    let bits = width.map_or(128, |ty| i128::from(ty.bits));
    Some(Int(match op {
        "add" => a.checked_add(b)?,
        "sub" => a.checked_sub(b)?,
        "mul" if runtime => a.wrapping_mul(b),
        "mul" => a.checked_mul(b)?,
        "div" => a.checked_div(b)?,
        "rem" => a.checked_rem(b)?,
        "bit_and" => a & b,
        "bit_or" => a | b,
        "bit_xor" => a ^ b,
        "bit_clear" => a & !b,
        "shl" if !runtime && b >= 0 => {
            if a == 0 {
                0
            } else if b < 127 {
                a.checked_mul(1i128 << b)?
            } else {
                return None;
            }
        }
        "shr" if !runtime && b >= 127 => {
            if a < 0 {
                -1
            } else {
                0
            }
        }
        "shr" if !runtime && b >= 0 => a >> b,
        "shl" if b >= bits => 0,
        "shr" if b >= bits => {
            if a < 0 {
                -1
            } else {
                0
            }
        }
        "shl" if b >= 0 => a << b,
        "shr" if b >= 0 => a >> b,
        _ => return None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_bounds::contracts::Contracts;
    use Scalar::Int;

    fn op(key: &str, (left, right): (i128, i128), ty: Integer, runtime: bool) -> Option<Scalar> {
        apply(
            Contracts::embedded().scalar("go").unwrap(),
            key,
            (Int(left), Some(Int(right))),
            (Some(ty), runtime),
        )
    }

    #[test]
    fn negative_quotients_truncate_and_remainders_retain_the_dividend_sign() {
        let ty = Integer {
            bits: 8,
            signed: true,
        };
        assert_eq!(Some(Int(-2)), op("div", (-7, 3), ty, true));
        assert_eq!(Some(Int(-1)), op("rem", (-7, 3), ty, true));
        assert_eq!(None, op("div", (-7, 0), ty, true));
        assert_eq!(-128, narrow(128, ty));
    }

    #[test]
    fn operand_width_never_masks_shift_distance() {
        let ty = Integer::unsigned(8);
        assert_eq!(Some(Int(0)), op("shl", (1, 8), ty, true));
        assert_eq!(Some(Int(256)), op("shl", (1, 8), ty, false));
        assert_eq!(
            Some(Int(-1)),
            op(
                "shr",
                (-10, 1000),
                Integer {
                    bits: 8,
                    signed: true
                },
                true
            )
        );
        assert_eq!(None, op("shl", (1, -1), ty, true));
    }

    #[test]
    fn unsupported_constant_magnitude_stays_unknown_before_a_later_narrowing() {
        let ty = Integer::unsigned(64);
        assert_eq!(Some(Int(1i128 << 66)), op("shl", (1, 66), ty, false));
        assert_eq!(None, op("shl", (1, 130), ty, false));
        assert_eq!(None, op("mul", (i128::MAX, 2), ty, false));
        let top = i128::from(u64::MAX);
        let Some(Int(product)) = op("mul", (top, top), ty, true) else {
            panic!("modeled unsigned product")
        };
        assert_eq!(1, narrow(product, ty));
    }
}
