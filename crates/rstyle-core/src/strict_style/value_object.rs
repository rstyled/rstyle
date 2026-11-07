//! Plain values that travel together through several functions under one set of names, which
//! is what a small type is for. Only parameters are read, so it can't say the values really belong
//! together: what the callers do with them decides that.

use crate::facts::{FnFacts, Scalar};
use crate::report::Diagnostic;
use crate::rules::{Measure, diagnostic};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const RULE: &str = "value_object";

/// Fewer values than this have nothing to travel with.
const MIN_PARAMETERS: usize = 2;
/// Fewer functions than this could share a set of names by chance.
const MIN_FUNCTIONS: usize = 3;

/// Names that say nothing about what a value holds, so a group of only these describes nothing.
const PLACEHOLDERS: [&str; 28] = [
    "foo", "bar", "baz", "qux", "tmp", "temp", "val", "var", "arg", "args", "param", "params",
    "obj", "item", "elem", "el", "p1", "p2", "p3", "a1", "a2", "a3", "v1", "v2", "x1", "x2", "y1",
    "y2",
];

/// A parameter as a group reads it: by name and the plain value it holds.
type Parameter<'f> = (&'f str, Scalar);
type Group<'f> = BTreeSet<Parameter<'f>>;

/// Whether a name says what its value holds, which a letter or a placeholder doesn't.
fn says_something(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.chars().count() > 1 && !PLACEHOLDERS.contains(&lower.as_str())
}

/// Each distinct set of plain parameters, and the functions with exactly that set.
fn functions_by_parameters(facts: &[FnFacts]) -> BTreeMap<Group<'_>, Vec<usize>> {
    let mut found: BTreeMap<Group<'_>, Vec<usize>> = BTreeMap::new();
    for (index, function) in facts.iter().enumerate() {
        let parameters: Group<'_> = function
            .param_list
            .iter()
            .filter(|parameter| !parameter.name.starts_with('_'))
            .filter_map(|parameter| Some((parameter.name.as_str(), parameter.scalar?)))
            .collect();
        if parameters.len() >= MIN_PARAMETERS {
            found.entry(parameters).or_default().push(index);
        }
    }
    found
}

/// What any two of those sets have in common, which is all a group can be.
fn shared<'f>(sets: &[&Group<'f>]) -> BTreeSet<Group<'f>> {
    let mut found = BTreeSet::new();
    for (position, first) in sets.iter().enumerate() {
        for second in &sets[position + 1..] {
            let both: Group<'f> = first.intersection(second).copied().collect();
            let names_something = both.iter().any(|(name, _)| says_something(name));
            if both.len() >= MIN_PARAMETERS && names_something {
                found.insert(both);
            }
        }
    }
    for set in sets {
        if set.iter().any(|(name, _)| says_something(name)) {
            found.insert((*set).clone());
        }
    }
    found
}

/// Each group in at least `MIN_FUNCTIONS` functions that no larger such group holds.
fn largest_groups<'f>(
    by_parameters: &BTreeMap<Group<'f>, Vec<usize>>,
) -> Vec<(Group<'f>, Vec<usize>)> {
    let sets: Vec<&Group<'f>> = by_parameters.keys().collect();
    let frequent: Vec<(Group<'f>, Vec<usize>)> = shared(&sets)
        .into_iter()
        .filter_map(|group| {
            let holders = by_parameters
                .iter()
                .filter(|(set, _)| group.is_subset(set))
                .flat_map(|(_, functions)| functions.iter().copied());
            let mut holders: Vec<usize> = holders.collect();
            holders.sort_unstable();
            (holders.len() >= MIN_FUNCTIONS).then_some((group, holders))
        })
        .collect();

    let held_by_more = |group: &Group<'f>| {
        frequent
            .iter()
            .any(|(other, _)| group != other && group.is_subset(other))
    };
    let kept = frequent.iter().filter(|(group, _)| !held_by_more(group));
    kept.cloned().collect()
}

pub(super) fn add(facts: &[FnFacts], found: &mut [Vec<Diagnostic>]) {
    for (group, functions) in largest_groups(&functions_by_parameters(facts)) {
        let names: Vec<String> = group.iter().map(|(name, _)| format!("`{name}`")).collect();
        let actual = functions.len();
        for index in functions {
            let message = format!(
                "parameters {} travel together here and in {} other functions; a small type could \
                 hold them",
                names.join(", "),
                actual - 1,
            );
            let measure = Measure {
                actual,
                limit: MIN_FUNCTIONS - 1,
            };
            found[index].push(diagnostic(&facts[index], RULE, message, measure));
        }
    }
}

#[cfg(test)]
mod tests;
