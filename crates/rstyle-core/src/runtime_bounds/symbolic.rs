use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bound {
    Const(u64),
    Input(String),
    Add(Vec<Bound>),
    Mul(Vec<Bound>),
    Max(Vec<Bound>),
}

impl Bound {
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
            Self::Const(_) | Self::Input(_) => 1,
            Self::Add(v) | Self::Mul(v) | Self::Max(v) => {
                1 + v.iter().map(Self::size).sum::<usize>()
            }
        }
    }
}
