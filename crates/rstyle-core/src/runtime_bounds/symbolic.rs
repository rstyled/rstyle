use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bound {
    Const(u64),
    Input(String),
    /// An explicitly unknown factor. A bound containing one is still a true upper
    /// bound, but only relative to that unknown quantity.
    Opaque(Opaque),
    Add(Vec<Bound>),
    Mul(Vec<Bound>),
    Max(Vec<Bound>),
    /// `⌊log₂ max(x, 1)⌋ + 1`: the bit length of `x`, at least one. It is monotone
    /// and integer-valued, and `log₂ x < log(x) <= log₂ x + 1` for every `x >= 1`, so
    /// substituting upper bounds for its argument keeps it an upper bound.
    Log(Box<Bound>),
}

/// Growth order of a bound, compared lexicographically: `n^polynomial * log^logarithmic`.
/// A ranking heuristic for multivariate bounds, not a total order on runtimes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Order {
    pub polynomial: usize,
    pub logarithmic: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opaque {
    pub kind: OpaqueKind,
    /// Identity of the unknown: a loop location and header node, callee or callee
    /// input. A site that names several copies (for example an inlined closure)
    /// stands for the maximum over them.
    pub site: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OpaqueKind {
    /// Executions of an unproven loop's header (iterations of its cycle) per entry
    /// of its innermost enclosing loop; everything inside that cycle runs at most
    /// once per iteration, so its body entries and nested work are bounded by it.
    Loop,
    /// Loop-body entries of a callee that could not be summarized, per invocation.
    Call,
    /// Workspace invocations made by a callee that could not be summarized, per
    /// invocation.
    Invocations,
    /// A callee size with no caller-side upper bound.
    Input,
    /// Library steps of an operation without a contract, or of a callee that could not
    /// be summarized, per invocation.
    Library,
}

impl OpaqueKind {
    /// Short name used when rendering `?kind(site)`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Call => "call",
            Self::Invocations => "invocations",
            Self::Input => "input",
            Self::Library => "library",
        }
    }
}

impl Bound {
    pub fn opaque(kind: OpaqueKind, site: impl Into<String>) -> Self {
        Self::Opaque(Opaque {
            kind,
            site: site.into(),
        })
    }

    /// Whether no opaque factor occurs anywhere in the bound.
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Const(_) | Self::Input(_) => true,
            Self::Opaque(_) => false,
            Self::Add(items) | Self::Mul(items) | Self::Max(items) => {
                items.iter().all(Self::is_complete)
            }
            Self::Log(value) => value.is_complete(),
        }
    }

    /// `log(value)`, folding constants. Logarithms of logarithms stay explicit.
    pub fn log(value: Self) -> Self {
        match value {
            Self::Const(value) => Self::Const(bit_length(value)),
            value => Self::Log(Box::new(value)),
        }
    }

    /// The bound's value at concrete inputs, saturating at `u64::MAX`; `None` when an
    /// input has no value or an opaque factor occurs.
    pub fn evaluate(&self, input: &dyn Fn(&str) -> Option<u64>) -> Option<u64> {
        let all = |items: &[Self]| -> Option<Vec<u64>> {
            items.iter().map(|item| item.evaluate(input)).collect()
        };
        match self {
            Self::Const(value) => Some(*value),
            Self::Input(name) => input(name),
            Self::Opaque(_) => None,
            Self::Add(items) => Some(all(items)?.into_iter().fold(0, u64::saturating_add)),
            Self::Mul(items) => Some(all(items)?.into_iter().fold(1, u64::saturating_mul)),
            Self::Max(items) => Some(all(items)?.into_iter().max().unwrap_or(0)),
            Self::Log(value) => value.evaluate(input).map(bit_length),
        }
    }

    /// Polynomial degree in inputs, then the number of logarithmic factors. Constants
    /// and opaque factors have order zero: an opaque size never raises the ranking.
    pub fn order(&self) -> Order {
        match self {
            Self::Const(_) | Self::Opaque(_) => Order::default(),
            Self::Input(_) => Order {
                polynomial: 1,
                logarithmic: 0,
            },
            Self::Add(items) | Self::Max(items) => {
                items.iter().map(Self::order).max().unwrap_or_default()
            }
            Self::Mul(items) => items
                .iter()
                .map(Self::order)
                .fold(Order::default(), |a, b| Order {
                    polynomial: a.polynomial + b.polynomial,
                    logarithmic: a.logarithmic + b.logarithmic,
                }),
            // `log(n^k * log n)` is still `Θ(log n)`.
            Self::Log(value) if value.order() == Order::default() => Order::default(),
            Self::Log(_) => Order {
                polynomial: 0,
                logarithmic: 1,
            },
        }
    }

    pub(super) fn add(values: Vec<Self>) -> Self {
        let values: Vec<_> = values
            .into_iter()
            .filter(|v| *v != Self::Const(0))
            .collect();
        match values.len() {
            0 => Self::Const(0),
            1 => values.into_iter().next().unwrap(),
            _ => Self::Add(values),
        }
    }

    pub(super) fn mul(left: Self, right: Self) -> Self {
        match (&left, &right) {
            (Self::Const(0), _) | (_, Self::Const(0)) => Self::Const(0),
            (Self::Const(1), _) => right,
            (_, Self::Const(1)) => left,
            _ => Self::Mul(vec![left, right]),
        }
    }

    pub(super) fn size(&self) -> usize {
        match self {
            Self::Const(_) | Self::Input(_) | Self::Opaque(_) => 1,
            Self::Add(v) | Self::Mul(v) | Self::Max(v) => {
                1 + v.iter().map(Self::size).sum::<usize>()
            }
            Self::Log(value) => 1 + value.size(),
        }
    }
}

/// Bit length of `max(value, 1)`: the value of `log` at a constant.
fn bit_length(value: u64) -> u64 {
    u64::from(u64::BITS - value.max(1).leading_zeros())
}

/// Infix rendering: sums inside products are parenthesized; opaque factors render as
/// `?kind(site)`.
impl std::fmt::Display for Bound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&render(self))
    }
}

fn render(bound: &Bound) -> String {
    match bound {
        Bound::Const(value) => value.to_string(),
        Bound::Input(name) => name.clone(),
        Bound::Opaque(opaque) => format!("?{}({})", opaque.kind.label(), opaque.site),
        Bound::Add(values) => joined(values, " + ", |_| false),
        Bound::Mul(values) => joined(values, " * ", |value| matches!(value, Bound::Add(_))),
        Bound::Max(values) => format!("max({})", joined(values, ", ", |_| false)),
        Bound::Log(value) => format!("log({})", render(value)),
    }
}

fn joined(values: &[Bound], separator: &str, grouped: fn(&Bound) -> bool) -> String {
    values
        .iter()
        .map(|value| match render(value) {
            text if grouped(value) => format!("({text})"),
            text => text,
        })
        .collect::<Vec<_>>()
        .join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> Bound {
        Bound::Input(name.into())
    }

    fn order(polynomial: usize, logarithmic: usize) -> Order {
        Order {
            polynomial,
            logarithmic,
        }
    }

    #[test]
    fn constant_logarithms_fold_to_bit_lengths() {
        let folded = |value| Bound::log(Bound::Const(value));
        assert_eq!(folded(0), Bound::Const(1));
        assert_eq!(folded(1), Bound::Const(1));
        assert_eq!(folded(2), Bound::Const(2));
        assert_eq!(folded(3), Bound::Const(2));
        assert_eq!(folded(4), Bound::Const(3));
        assert_eq!(folded(1023), Bound::Const(10));
        assert_eq!(folded(1024), Bound::Const(11));
        assert_eq!(folded(u64::MAX), Bound::Const(64));
        // Every value is at least log₂ x and at most log₂ x + 1.
        for value in 1..=4096_u64 {
            let Bound::Const(bits) = folded(value) else {
                unreachable!()
            };
            assert!(1 << (bits - 1) <= value && value < 1 << bits, "{value}");
        }
    }

    #[test]
    fn order_ranks_polynomial_degree_before_logarithms() {
        let n = || input("n");
        let log_n = || Bound::log(n());
        assert_eq!(n().order(), order(1, 0));
        assert_eq!(log_n().order(), order(0, 1));
        assert_eq!(Bound::Mul(vec![n(), log_n()]).order(), order(1, 1));
        assert_eq!(Bound::log(Bound::Mul(vec![n(), n()])).order(), order(0, 1));
        assert_eq!(Bound::log(Bound::Const(9)).order(), Order::default());
        let constant_log = Bound::Log(Box::new(Bound::opaque(OpaqueKind::Input, "f::n")));
        assert_eq!(constant_log.order(), Order::default());
        assert_eq!(Bound::Max(vec![]).order(), Order::default());

        let ranked = [
            Bound::Mul(vec![n(), n()]),
            Bound::Mul(vec![n(), log_n()]),
            n(),
            Bound::Mul(vec![log_n(), log_n()]),
            log_n(),
            Bound::Const(7),
        ];
        assert!(
            ranked
                .windows(2)
                .all(|pair| pair[0].order() > pair[1].order())
        );
    }

    #[test]
    fn logarithms_render_and_keep_opaque_factors_visible() {
        let bound = Bound::Mul(vec![
            Bound::Add(vec![input("n"), Bound::Const(1)]),
            Bound::log(Bound::Max(vec![input("m"), input("k")])),
        ]);
        assert_eq!(bound.to_string(), "(n + 1) * log(max(m, k))");
        assert_eq!(bound.size(), 8);
        assert!(bound.is_complete());

        let opaque = Bound::log(Bound::opaque(OpaqueKind::Loop, "f.rs:1:1#0"));
        assert_eq!(opaque.to_string(), "log(?loop(f.rs:1:1#0))");
        assert!(!opaque.is_complete());
    }

    #[test]
    fn evaluation_saturates_and_rejects_unknown_values() {
        let values = |name: &str| match name {
            "n" => Some(1000),
            "big" => Some(u64::MAX),
            _ => None,
        };
        let n_log_n = Bound::Mul(vec![input("n"), Bound::log(input("n"))]);
        assert_eq!(n_log_n.evaluate(&values), Some(10_000));
        let huge = Bound::Add(vec![
            input("big"),
            Bound::Mul(vec![input("big"), input("n")]),
        ]);
        assert_eq!(huge.evaluate(&values), Some(u64::MAX));
        assert_eq!(Bound::Max(vec![]).evaluate(&values), Some(0));
        assert_eq!(Bound::log(input("missing")).evaluate(&values), None);
        let opaque = Bound::Add(vec![input("n"), Bound::opaque(OpaqueKind::Call, "f")]);
        assert_eq!(opaque.evaluate(&values), None);
    }
}
