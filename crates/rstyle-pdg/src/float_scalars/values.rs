//! Source decimal rounding is independent of rustc's captured IEEE bit value.
use super::width;
use crate::scalar_constants::ScalarConstant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatValue {
    pub ty: String,
    pub bits: String,
}

impl FloatValue {
    pub fn from_bits(ty: &str, bits: u64) -> Result<Self, String> {
        let digits = match width(ty) {
            Some(32) if bits <= u64::from(u32::MAX) => 8,
            Some(64) => 16,
            _ => return Err("Float bit value exceeds its exact registered width".into()),
        };
        Ok(Self {
            ty: ty.into(),
            bits: format!("0x{bits:0digits$x}"),
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        let bits = self
            .bits
            .strip_prefix("0x")
            .and_then(|value| u64::from_str_radix(value, 16).ok())
            .ok_or("Malformed float bit value")?;
        if *self != Self::from_bits(&self.ty, bits)? {
            return Err("Float bits are not canonical fixed-width lowercase hexadecimal".into());
        }
        Ok(())
    }

    pub fn parse(source: &str, ty: &str) -> Result<Self, String> {
        let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
        let (literal, negative) = match &expression {
            syn::Expr::Lit(value) => (&value.lit, false),
            syn::Expr::Unary(value) if matches!(value.op, syn::UnOp::Neg(_)) => {
                let syn::Expr::Lit(literal) = &*value.expr else {
                    return Err(
                        "Float source permits only a literal or its builtin negation".into(),
                    );
                };
                (&literal.lit, true)
            }
            _ => return Err("Float source permits only a decimal literal".into()),
        };

        let syn::Lit::Float(literal) = literal else {
            return Err("Float source requires an original floating literal".into());
        };
        if !literal.suffix().is_empty() && literal.suffix() != ty {
            return Err("Float literal suffix disagrees with its resolved width".into());
        }
        decimal(literal.base10_digits(), ty, negative)
    }
}

fn decimal(source: &str, ty: &str, negative: bool) -> Result<FloatValue, String> {
    let bits = match ty {
        "f32" => {
            let value: f32 = source
                .parse()
                .map_err(|_| "Invalid binary32 source decimal")?;
            if !value.is_finite() {
                return Err("Float source literal overflows binary32".into());
            }
            u64::from(value.to_bits() ^ if negative { 1 << 31 } else { 0 })
        }
        "f64" => {
            let value: f64 = source
                .parse()
                .map_err(|_| "Invalid binary64 source decimal")?;
            if !value.is_finite() {
                return Err("Float source literal overflows binary64".into());
            }
            value.to_bits() ^ if negative { 1 << 63 } else { 0 }
        }
        _ => return Err("Unregistered float source width".into()),
    };
    FloatValue::from_bits(ty, bits)
}

pub(super) fn constant(record: &ScalarConstant) -> Result<FloatValue, String> {
    let declaration: syn::ItemConst = syn::parse_str(&record.declaration)
        .map_err(|error| format!("Missing original float constant declaration: {error}"))?;
    let syn::Type::Path(ty) = &*declaration.ty else {
        return Err("Float constant requires an explicit primitive width".into());
    };
    if !declaration.generics.params.is_empty()
        || declaration.generics.where_clause.is_some()
        || ty.qself.is_some()
        || !ty.path.is_ident(&record.ty)
        || record.definition.rsplit("::").next() != Some(declaration.ident.to_string().as_str())
    {
        return Err("Float constant definition or original declared width disagrees".into());
    }

    let expression = &declaration.expr;
    let spelling = quote_expression(expression)?;
    let expected = FloatValue::parse(&spelling, &record.ty)?;
    let actual = FloatValue {
        ty: record.ty.clone(),
        bits: record.value.clone(),
    };

    actual.validate()?;
    if actual != expected {
        return Err(
            "Float constant captured bits disagree with independently rounded source".into(),
        );
    }
    Ok(actual)
}

fn quote_expression(expression: &syn::Expr) -> Result<String, String> {
    match expression {
        syn::Expr::Lit(value) => match &value.lit {
            syn::Lit::Float(value) => Ok(value.to_string()),
            _ => Err("Float constant requires a floating literal".into()),
        },
        syn::Expr::Unary(value) if matches!(value.op, syn::UnOp::Neg(_)) => {
            Ok(format!("-{}", quote_expression(&value.expr)?))
        }
        _ => Err("Float constants permit only original literals and unary negation".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_at_the_declared_width_and_preserves_zero_sign() {
        for (source, ty, expected) in [
            ("1.0", "f64", "0x3ff0000000000000"),
            ("-0.0", "f64", "0x8000000000000000"),
            ("-0.0f32", "f32", "0x80000000"),
            ("16_777_217.0f32", "f32", "0x4b800000"),
            ("1.000000059604644775390625f32", "f32", "0x3f800000"),
            ("1.401298464324817e-45f32", "f32", "0x00000001"),
        ] {
            assert_eq!(FloatValue::parse(source, ty).unwrap().bits, expected);
        }
        for (source, ty) in [
            ("1.0f64", "f32"),
            ("1", "f64"),
            ("1e999", "f64"),
            ("f64::NAN", "f64"),
            ("1.0 + 2.0", "f64"),
            ("-(-1.0)", "f64"),
        ] {
            assert!(FloatValue::parse(source, ty).is_err(), "{source}");
        }
    }

    #[test]
    fn bit_encoding_is_fixed_width_and_never_a_json_number() {
        assert_eq!(
            FloatValue::from_bits("f32", 0x8000_0000).unwrap().bits,
            "0x80000000"
        );
        assert!(FloatValue::from_bits("f32", 1u64 << 32).is_err());
        for bits in ["0", "0x0", "0X00000000", "0xFFFFFFFF", "0x0000000000000000"] {
            assert!(
                FloatValue {
                    ty: "f32".into(),
                    bits: bits.into()
                }
                .validate()
                .is_err()
            );
        }
    }

    #[test]
    fn original_constant_source_independently_constrains_captured_bits() {
        let mut record = ScalarConstant {
            definition: "math::kernel::C".into(),
            declaration: "const C: f64 = -0.5;".into(),
            ty: "f64".into(),
            value: "0xbfe0000000000000".into(),
        };
        constant(&record).unwrap();
        record.value = "0x3fe0000000000000".into();
        assert!(constant(&record).is_err());
        record.value = "0xbfe0000000000000".into();
        record.declaration = "const C: f64 = -0.25;".into();
        assert!(constant(&record).is_err());
    }
}
