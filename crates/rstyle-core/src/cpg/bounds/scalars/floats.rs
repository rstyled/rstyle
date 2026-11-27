//! Evaluate explicitly rounded floating values and operations whose mathematical result
//! is exactly representable. Inexact intermediate products or sums can participate in
//! fused operations, so their values stay unknown without an explicit rounding boundary.
use super::{Node, Scalar, TypeKind, Walk};
use crate::cpg::model::Type;
use crate::runtime_bounds::Integer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Binary {
    significand: i128,
    exponent: i32,
}

impl Binary {
    fn new(n: i128, e: i32) -> Self {
        if n == 0 {
            return Self {
                significand: 0,
                exponent: 0,
            };
        }
        let shift = n.trailing_zeros();
        Self {
            significand: n >> shift,
            exponent: e + shift as i32,
        }
    }
    fn of(v: f64) -> Option<Self> {
        if !v.is_finite() {
            return None;
        }
        let bits = v.to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let mut n = i128::from(bits & ((1u64 << 52) - 1));
        if e != 0 {
            n += 1i128 << 52;
        }
        if v.is_sign_negative() {
            n = -n;
        }
        Some(Self::new(n, e.max(1) - 1023 - 52))
    }
    fn mul(self, b: Self) -> Option<Self> {
        Some(Self::new(
            self.significand.checked_mul(b.significand)?,
            self.exponent + b.exponent,
        ))
    }
    fn add(self, b: Self) -> Option<Self> {
        if self.significand == 0 {
            return Some(b);
        }
        if b.significand == 0 {
            return Some(self);
        }
        let e = self.exponent.min(b.exponent);
        let scale = |a: Self| {
            let shift = u32::try_from(a.exponent - e).ok()?;
            if shift >= 127 {
                return None;
            }
            a.significand.checked_mul(1i128 << shift)
        };
        Some(Self::new(scale(self)?.checked_add(scale(b)?)?, e))
    }
}

fn round(x: f64, bits: u32) -> Option<f64> {
    let y = match bits {
        32 => f64::from(x as f32),
        64 => x,
        _ => return None,
    };
    (y.is_finite() || !x.is_finite()).then_some(y)
}

fn binary(op: &str, (a, b): (f64, f64), bits: u32, rounded: bool) -> Option<f64> {
    let out = match (bits, op) {
        (32, "add") => f64::from(a as f32 + b as f32),
        (32, "sub") => f64::from(a as f32 - b as f32),
        (32, "mul") => f64::from(a as f32 * b as f32),
        (32, "div") if b != 0. => f64::from(a as f32 / b as f32),
        (64, "add") => a + b,
        (64, "sub") => a - b,
        (64, "mul") => a * b,
        (64, "div") if b != 0. => a / b,
        _ => return None,
    };
    if !out.is_finite() {
        return None;
    }
    if rounded {
        return Some(out);
    }
    // The exactness proof below normalizes the sign of zero. For a negative
    // zero operand, keep its potential contribution to the sign of an
    // unfenced zero result unknown.
    if out == 0. && [a, b].iter().any(|x| x.to_bits() == (-0.0f64).to_bits()) {
        return None;
    }
    let result = Binary::of(out)?;
    let left = Binary::of(a)?;
    let right = Binary::of(b)?;
    let exact = match op {
        "add" => left.add(right)? == result,
        "sub" => left.add(Binary::of(-b)?)? == result,
        "mul" => left.mul(right)? == result,
        "div" => result.mul(right)? == left,
        _ => false,
    };
    exact.then_some(out)
}

fn integer(v: f64, ty: Integer) -> Option<i128> {
    let v = v.trunc();
    let edge = 2f64.powi(i32::from(ty.bits) - i32::from(ty.signed));
    (v.is_finite() && v < edge && (if ty.signed { v >= -edge } else { v >= 0. }))
        .then_some(v as i128)
}

fn converted_integer(n: i128, bits: u32) -> Option<f64> {
    match bits {
        32 => Some(f64::from(n as f32)),
        64 => Some(n as f64),
        _ => None,
    }
}

fn component(s: &str, bits: u32) -> Option<f64> {
    if s.len() > 4096 {
        return None;
    }
    if let Some((n, d)) = s.split_once('/') {
        let n = n.parse::<i64>().ok()?;
        let d = d.parse::<u64>().ok()?;
        if n.unsigned_abs() > 1 << 53 || d > 1 << 53 || !d.is_power_of_two() {
            return None;
        }
        return round(n as f64 / d as f64, bits);
    }
    let v = match bits {
        32 => f64::from(s.parse::<f32>().ok()?),
        64 => s.parse::<f64>().ok()?,
        _ => return None,
    };
    v.is_finite().then_some(v)
}

impl Walk<'_, '_, '_> {
    pub(super) fn complex_width(&self, ty: &Type) -> Option<u32> {
        let key = ty.key.as_deref()?;
        if key == self.profile.complex64 {
            return Some(32);
        }
        (key == self.profile.complex128).then_some(64)
    }

    pub(super) fn floating_literal(&self, node: &Node, s: &str) -> Option<Scalar> {
        let ty = self.facts.ty(node)?;
        let bits = if ty.kind == TypeKind::Float {
            ty.bits?
        } else {
            if ty.key.as_deref() != Some(&self.profile.untyped_float) {
                return None;
            }
            let n = s.parse::<i64>().ok()?;
            if n.unsigned_abs() > 1 << 53 {
                return None;
            }
            return Some(Scalar::Real(n as f64));
        };
        Some(Scalar::Real(component(s, bits)?))
    }

    pub(super) fn complex_constant(&self, node: &Node) -> Option<Scalar> {
        let fact = self
            .facts
            .tables
            .extension(&self.profile.byte_constants)?
            .get(&node.id)?;
        if fact.get("kind")?.as_str()? != "complex" {
            return None;
        }
        let bits = self.complex_width(self.facts.ty(node)?)?;
        Some(Scalar::Complex(
            component(fact.get("real")?.as_str()?, bits)?,
            component(fact.get("imag")?.as_str()?, bits)?,
        ))
    }

    pub(super) fn convert_numeric(&self, v: &Scalar, to: &Type) -> Option<Scalar> {
        match (v, to.kind) {
            (Scalar::Int(n), TypeKind::Float) => {
                Some(Scalar::Real(converted_integer(*n, to.bits?)?))
            }
            (Scalar::Real(x), TypeKind::Float) => Some(Scalar::Real(round(*x, to.bits?)?)),
            (Scalar::Real(x), TypeKind::Integer) => {
                Some(Scalar::Int(integer(*x, self.facts.integer(Some(to))?)?))
            }
            (Scalar::Int(n), TypeKind::Opaque) => Some(Scalar::Complex(
                converted_integer(*n, self.complex_width(to)?)?,
                0.,
            )),
            (Scalar::Real(x), TypeKind::Opaque) => {
                Some(Scalar::Complex(round(*x, self.complex_width(to)?)?, 0.))
            }
            (Scalar::Complex(x, y), TypeKind::Opaque) => {
                let bits = self.complex_width(to)?;
                Some(Scalar::Complex(round(*x, bits)?, round(*y, bits)?))
            }
            _ => None,
        }
    }

    pub(super) fn floating_op(
        &self,
        node: &Node,
        op: &str,
        values: (Scalar, Option<Scalar>),
    ) -> Option<Scalar> {
        use Scalar::{Bool, Complex, Real};
        let ty = self.facts.ty(node)?;
        let rounded = self.round_at == Some(&node.id);
        match values {
            (Real(x), None) if op == "neg" => Some(Real(-x)),
            (v @ (Real(_) | Complex(_, _)), None) if op == self.profile.unary_plus => Some(v),
            (Complex(a, b), None) if op == "neg" => Some(Complex(-a, -b)),
            (Real(a), Some(Real(b))) if ty.kind == TypeKind::Bool => Some(Bool(match op {
                "eq" => a == b,
                "ne" => a != b,
                "lt" => a < b,
                "le" => a <= b,
                "gt" => a > b,
                "ge" => a >= b,
                _ => return None,
            })),
            (Complex(a, b), Some(Complex(c, d))) if op == "eq" || op == "ne" => {
                Some(Bool((a == c && b == d) == (op == "eq")))
            }
            (Real(a), Some(Real(b))) => Some(Real(binary(op, (a, b), ty.bits?, rounded)?)),
            (Complex(a, b), Some(Complex(c, d))) => {
                let bits = self.complex_width(ty)?;
                let part = |op, x, y| binary(op, (x, y), bits, false);
                Some(match op {
                    "add" | "sub" => Complex(
                        binary(op, (a, c), bits, rounded)?,
                        binary(op, (b, d), bits, rounded)?,
                    ),
                    "mul" => Complex(
                        part("sub", part("mul", a, c)?, part("mul", b, d)?)?,
                        part("add", part("mul", a, d)?, part("mul", b, c)?)?,
                    ),
                    "div"
                        if b.to_bits() == 0
                            && d.to_bits() == 0
                            && (0.0..1e12).contains(&a)
                            && !a.is_sign_negative()
                            && (1e-12..1e12).contains(&c) =>
                    {
                        Complex(part("div", a, c)?, 0.)
                    }
                    _ => return None,
                })
            }
            _ => None,
        }
    }
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn math_call(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        if op == self.profile.not_a_number && node.children.len() == 1 {
            return Some(Scalar::Real(f64::NAN));
        }
        if op == self.profile.infinity && node.children.len() == 2 {
            let Scalar::Int(sign) = self.value(self.child(node, 1)?, depth)? else {
                return None;
            };
            return Some(Scalar::Real(if sign < 0 {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }));
        }
        if op == self.profile.sign_bit && node.children.len() == 2 {
            let Scalar::Real(x) = self.value(self.child(node, 1)?, depth)? else {
                return None;
            };
            return (!x.is_nan()).then_some(Scalar::Bool(x.is_sign_negative()));
        }
        if op == self.profile.copy_sign && node.children.len() == 3 {
            let Scalar::Real(x) = self.value(self.child(node, 1)?, depth)? else {
                return None;
            };
            let Scalar::Real(y) = self.value(self.child(node, 2)?, depth)? else {
                return None;
            };
            return (!y.is_nan()).then_some(Scalar::Real(x.copysign(y)));
        }
        None
    }

    pub(super) fn complex_builtin(
        &mut self,
        node: &'a Node,
        op: &str,
        depth: usize,
    ) -> Option<Scalar> {
        if op == self.profile.complex_value && node.children.len() == 3 {
            let bits = self.complex_width(self.facts.ty(node)?)?;
            let mut parts = [0.; 2];
            for (i, out) in parts.iter_mut().enumerate() {
                *out = match self.value(self.child(node, i + 1)?, depth)? {
                    Scalar::Int(n) => converted_integer(n, bits)?,
                    Scalar::Real(x) => round(x, bits)?,
                    _ => return None,
                };
            }
            return Some(Scalar::Complex(parts[0], parts[1]));
        }
        if node.children.len() != 2 {
            return None;
        }
        let Scalar::Complex(x, y) = self.value(self.child(node, 1)?, depth)? else {
            return None;
        };
        if op == self.profile.real_component {
            return Some(Scalar::Real(x));
        }
        (op == self.profile.imaginary_component).then_some(Scalar::Real(y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inexact_intermediates_require_an_explicit_rounding_boundary() {
        assert_eq!(binary("add", (16777216., 1.), 32, false), None);
        assert_eq!(binary("add", (16777216., 1.), 32, true), Some(16777216.));
        assert_eq!(binary("div", (4., 3.), 64, false), None);
        assert_eq!(binary("mul", (0.5, 0.25), 32, false), Some(0.125));
    }
    #[test]
    fn zero_signs_and_underflow_require_a_proven_rounding_boundary() {
        assert_eq!(binary("mul", (-0., 2.), 64, false), None);
        assert_eq!(
            binary("mul", (-0., 2.), 64, true).unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(binary("div", (f64::from_bits(1), 2.), 64, false), None);
        assert_eq!(binary("sub", (1., 1.), 64, false).unwrap().to_bits(), 0);
    }
    #[test]
    fn out_of_range_float_to_integer_stays_unknown() {
        assert_eq!(integer(256., Integer::unsigned(8)), None);
        assert_eq!(integer(-0.9, Integer::unsigned(8)), Some(0));
    }
}
