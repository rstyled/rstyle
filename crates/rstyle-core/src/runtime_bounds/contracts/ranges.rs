//! Range roles: a library call that makes a range of integers counting up from a start to an
//! exclusive stop, by the arguments that hold them, so a `for` over it is a counted scan.
use serde::Deserialize;

/// Where one arity of a range call holds its bounds, counting the arguments from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeForm {
    pub arity: u32,
    /// The first value, zero when absent.
    pub start: Option<u32>,
    /// The exclusive end.
    pub stop: u32,
    /// The distance between values, one when absent. Only a constant of at least one counts up.
    pub step: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RangeEntry {
    #[serde(rename = "basis")]
    _basis: String,
    forms: Vec<RangeForm>,
    pub(super) operations: Vec<String>,
}

impl RangeEntry {
    /// The forms, once each names distinct arguments of its own arity, and arities are distinct.
    pub(super) fn forms(&self) -> Result<Vec<RangeForm>, String> {
        let mut arities: Vec<u32> = self.forms.iter().map(|form| form.arity).collect();
        arities.sort_unstable();
        arities.dedup();
        let placed = self.forms.iter().all(|form| {
            let mut used: Vec<u32> = [form.start, Some(form.stop), form.step]
                .into_iter()
                .flatten()
                .collect();
            let count = used.len();
            used.sort_unstable();
            used.dedup();
            used.len() == count && used.iter().all(|index| *index < form.arity)
        });
        if self.forms.is_empty() || arities.len() != self.forms.len() || !placed {
            return Err(format!("Range role has forms {:?}", self.forms));
        }
        Ok(self.forms.clone())
    }
}

/// A library call that yields at most one item for each item of one of its arguments, as
/// Python's `enumerate` does, so a `for` over it counts what that argument counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Traversal {
    /// The argument counts it has the role at.
    pub arities: Vec<u32>,
    /// The argument whose items it yields, counting from zero.
    pub over: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TraversalEntry {
    #[serde(rename = "basis")]
    _basis: String,
    arities: Vec<u32>,
    over: u32,
    pub(super) operations: Vec<String>,
}

impl TraversalEntry {
    /// The role, once the argument it traverses is one it has at every arity.
    pub(super) fn traversal(&self) -> Result<Traversal, String> {
        if self.arities.is_empty() || self.arities.iter().any(|arity| self.over >= *arity) {
            return Err(format!(
                "Traversal role over {} has arities {:?}",
                self.over, self.arities
            ));
        }
        Ok(Traversal {
            arities: self.arities.clone(),
            over: self.over,
        })
    }
}
