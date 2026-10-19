//! Finite scalar-domain coverage without expanding ranges into individual values.

#[cfg(test)]
mod tests;

use crate::{HirNode, scalar_patterns::IntegerRange};

type Interval = (u128, u128);

struct Domain {
    boolean: bool,
    signed: bool,
    limits: Interval,
}

/// A closing unguarded arm after which no scalar value can remain unmatched.
/// Existing final-wildcard matches use a direct fast path without value parsing.
/// Pattern shape, source correspondence and constant resolution are checked by
/// eligibility separately; unsupported or malformed values never prove coverage.
pub(crate) fn closing_arm(node: &HirNode, width: Option<u64>) -> Option<usize> {
    let (scrutinee, arms) = node.children.split_first()?;
    if arms.last().is_some_and(|arm| {
        arm.kind == "match_arm" && arm.children.len() == 2 && arm.children[0].kind == "pat_wild"
    }) {
        return Some(arms.len() - 1);
    }

    let domain = Domain::new(scrutinee.ty.as_ref()?.normalized.as_deref()?, width)?;
    let mut intervals = Vec::new();

    for (index, arm) in arms.iter().enumerate() {
        if arm.kind != "match_arm" || arm.children.len() != 2 {
            continue;
        }
        intervals.extend(domain.pattern(&arm.children[0])?);
        if covers(&mut intervals, domain.limits) {
            return Some(index);
        }
    }
    None
}

impl Domain {
    fn new(ty: &str, width: Option<u64>) -> Option<Self> {
        if ty == "bool" {
            return Some(Self {
                boolean: true,
                signed: false,
                limits: (0, 1),
            });
        }
        if !crate::concrete_types::scalar(ty) {
            return None;
        }

        let signed = ty.starts_with('i');
        let bits = match ty {
            "isize" | "usize" => width?,
            _ => ty.strip_prefix(['i', 'u'])?.parse().ok()?,
        };

        if !matches!(bits, 8 | 16 | 32 | 64 | 128) {
            return None;
        }

        let limits = if bits == 128 {
            (0, u128::MAX)
        } else if signed {
            let midpoint = 1u128 << 127;
            let half = 1u128 << (bits - 1);
            (midpoint - half, midpoint + half - 1)
        } else {
            (0, (1u128 << bits) - 1)
        };

        Some(Self {
            boolean: false,
            signed,
            limits,
        })
    }

    fn value(&self, source: &str) -> Option<u128> {
        let expression: syn::Expr = syn::parse_str(source).ok()?;
        if self.boolean {
            return match expression {
                syn::Expr::Lit(literal) => match literal.lit {
                    syn::Lit::Bool(value) => Some(u128::from(value.value)),
                    _ => None,
                },
                _ => None,
            };
        }

        let (negative, magnitude) = integer(&expression)?;
        let value = if self.signed {
            let midpoint = 1u128 << 127;
            if negative {
                midpoint.checked_sub(magnitude)?
            } else {
                midpoint.checked_add(magnitude)?
            }
        } else if negative && magnitude != 0 {
            return None;
        } else {
            magnitude
        };

        (self.limits.0 <= value && value <= self.limits.1).then_some(value)
    }

    fn pattern(&self, pattern: &HirNode) -> Option<Vec<Interval>> {
        match pattern.kind.as_str() {
            "pat_wild" => Some(vec![self.limits]),
            "pat_literal" | "pat_constant" => {
                let value = self.value(pattern.lit.as_deref()?)?;
                Some(vec![(value, value)])
            }
            "pat_range" if !self.boolean => {
                let range: IntegerRange = serde_json::from_str(pattern.lit.as_deref()?).ok()?;
                self.range(&range).map(|interval| vec![interval])
            }
            "pat_or" => pattern
                .children
                .iter()
                .try_fold(Vec::new(), |mut values, child| {
                    values.extend(self.pattern(child)?);
                    Some(values)
                }),
            _ => None,
        }
    }

    fn range(&self, range: &IntegerRange) -> Option<Interval> {
        let start = range
            .start
            .as_deref()
            .map(|v| self.value(v))
            .unwrap_or(Some(self.limits.0))?;
        let end = match range.end.as_deref() {
            Some(value) if !range.inclusive => self.value(value)?.checked_sub(1)?,
            Some(value) => self.value(value)?,
            None => self.limits.1,
        };
        (start <= end).then_some((start, end))
    }
}

fn integer(expression: &syn::Expr) -> Option<(bool, u128)> {
    match expression {
        syn::Expr::Lit(literal) => match &literal.lit {
            syn::Lit::Int(value) => Some((false, value.base10_parse().ok()?)),
            _ => None,
        },
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            let (negative, value) = integer(&unary.expr)?;
            (!negative).then_some((true, value))
        }
        _ => None,
    }
}

fn covers(intervals: &mut [Interval], (minimum, maximum): Interval) -> bool {
    intervals.sort_unstable();
    let mut next = minimum;
    for &(start, end) in intervals.iter() {
        if start > next {
            return false;
        }
        if end >= maximum {
            return true;
        }
        next = next.max(end + 1);
    }
    false
}
