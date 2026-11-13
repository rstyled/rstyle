//! How the errors functions can return changed: each function's sources, with every call into the
//! codebase followed to where that callee's errors come from, compared between the runs.

use super::{FunctionChange, Snapshot};
use crate::cpg::failures::{Failure, Source};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// A function whose errors can come from somewhere new, or no longer can.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ErrorChange {
    pub function: String,
    pub name: String,
    pub path: String,
    pub line: usize,
    pub lost: Vec<Source>,
    pub gained: Vec<Source>,
    /// How many of the sources it lost or gained it only passes on, wrapped in a message of its
    /// own: a function that wraps another's change says the same change once more.
    pub wrapped: usize,
}

/// How the errors functions can return changed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Errors {
    /// Whether any function's errors could be read: only Go's are, so far.
    pub traced: bool,
    /// Changed functions that return an error, whose sources were all compared.
    pub followed: usize,
    pub changed: Vec<ErrorChange>,
    /// Changed functions that can fail through something that can't be followed.
    pub untraced: Vec<ErrorChange>,
}

/// How the errors functions in both runs can return changed, and which changed functions can't be
/// followed all the way.
pub(super) fn compare(base: &Snapshot, head: &Snapshot, changes: &[FunctionChange]) -> Errors {
    let (mut before, mut after) = (Resolved::new(base), Resolved::new(head));
    let both = base
        .index
        .keys()
        .filter(|identity| head.index.contains_key(*identity));
    let changed: Vec<ErrorChange> = both
        .filter_map(|identity| difference(identity, head, (&mut before, &mut after)))
        .collect();

    let returning: Vec<&FunctionChange> = changes
        .iter()
        .filter(|change| change.kind != super::ChangeKind::Removed)
        .filter(|change| after.of(&change.function).is_some())
        .collect();
    let untraced = returning
        .iter()
        .filter(|change| {
            after
                .of(&change.function)
                .is_some_and(|found| found.iter().any(is_unknown))
        })
        .map(|change| at(head, &change.function))
        .collect();
    Errors {
        traced: head
            .declared
            .iter()
            .any(|declared| declared.failures.is_some()),
        followed: returning.len(),
        changed,
        untraced,
    }
}

fn difference(
    identity: &str,
    head: &Snapshot,
    (before, after): (&mut Resolved<'_>, &mut Resolved<'_>),
) -> Option<ErrorChange> {
    let (old, new) = (before.of(identity)?, after.of(identity)?);
    let sources = |found: &BTreeSet<Failure>| -> BTreeSet<Source> {
        found.iter().map(|failure| failure.source.clone()).collect()
    };
    let (old_sources, new_sources) = (sources(&old), sources(&new));
    if old_sources == new_sources {
        return None;
    }

    let wrapped = old
        .symmetric_difference(&new)
        .filter(|failure| failure.wrapped > 0)
        .count();
    let lost = old_sources.difference(&new_sources).cloned().collect();
    let gained = new_sources.difference(&old_sources).cloned().collect();
    Some(ErrorChange {
        lost,
        gained,
        wrapped,
        ..at(head, identity)
    })
}

/// A function of `snapshot`, where it's written, with nothing yet said of how its errors changed.
fn at(snapshot: &Snapshot, identity: &str) -> ErrorChange {
    let facts = snapshot.function(identity).map(|(facts, _)| facts);
    ErrorChange {
        function: identity.to_owned(),
        name: facts.map_or_else(|| identity.to_owned(), |facts| facts.name.clone()),
        path: facts.map(|facts| facts.path.clone()).unwrap_or_default(),
        line: facts.map_or(0, |facts| facts.line),
        lost: Vec::new(),
        gained: Vec::new(),
        wrapped: 0,
    }
}

fn is_unknown(failure: &Failure) -> bool {
    failure.source == Source::Unknown
}

/// Each function's sources with its calls into the codebase followed, worked out once.
struct Resolved<'s> {
    snapshot: &'s Snapshot,
    found: BTreeMap<String, Option<BTreeSet<Failure>>>,
}

impl<'s> Resolved<'s> {
    fn new(snapshot: &'s Snapshot) -> Self {
        Self {
            snapshot,
            found: BTreeMap::new(),
        }
    }

    /// Where the error a function returns can come from, or nothing when it returns none.
    fn of(&mut self, identity: &str) -> Option<BTreeSet<Failure>> {
        if let Some(found) = self.found.get(identity) {
            return found.clone();
        }
        // A call back into a function being worked out adds nothing it doesn't already have.
        self.found
            .insert(identity.to_owned(), Some(BTreeSet::new()));
        let local = self
            .snapshot
            .function(identity)
            .and_then(|(_, declared)| declared.failures.clone());
        let resolved = local.map(|local| {
            local
                .into_iter()
                .flat_map(|failure| self.followed(failure))
                .collect()
        });

        self.found.insert(identity.to_owned(), resolved.clone());
        resolved
    }

    /// A call into the codebase, as where its callee's errors come from, one wrapping deeper.
    fn followed(&mut self, failure: Failure) -> Vec<Failure> {
        let Source::Call(callee) = &failure.source else {
            return vec![failure];
        };
        if !self.snapshot.index.contains_key(callee) {
            return vec![failure];
        }
        let inner = self.of(callee).unwrap_or_default();
        inner
            .into_iter()
            .map(|found| Failure {
                wrapped: found.wrapped + failure.wrapped,
                ..found
            })
            .collect()
    }
}
