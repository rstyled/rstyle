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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opaque {
    pub kind: OpaqueKind,
    /// Stable identity of the unknown: a loop location, callee or callee input.
    pub site: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpaqueKind {
    /// Body entries of an unproven loop, per entry of its innermost enclosing loop.
    Loop,
    /// Loop-body entries of a callee that could not be summarized, per invocation.
    Call,
    /// A callee size with no caller-side upper bound.
    Input,
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
        }
    }
}
