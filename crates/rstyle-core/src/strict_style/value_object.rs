//! Plain values that travel together through several functions under one set of names, which
//! is what a small type is for. Only parameters are read, so it can't say the values really belong
//! together: what the callers do with them decides that.

use crate::config::{ValueObjectConfig, Word};
use crate::facts::{FnFacts, Scalar};
use crate::report::Diagnostic;
use crate::rules::{Measure, diagnostic};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const RULE: &str = "value_object";

/// A parameter as a group reads it: by name and the plain value it holds.
type Parameter<'f> = (&'f str, Scalar);
type Group<'f> = BTreeSet<Parameter<'f>>;

/// Whether a name says what its value holds, which a letter or a placeholder doesn't.
fn says_something(name: &str, placeholders: &[Word]) -> bool {
    let lower = name.to_lowercase();
    lower.chars().count() > 1 && !placeholders.iter().any(|word| word.as_str() == lower)
}

fn any_says_something(group: &Group<'_>, placeholders: &[Word]) -> bool {
    group
        .iter()
        .any(|(name, _)| says_something(name, placeholders))
}

/// Each distinct set of plain parameters, and the functions with exactly that set.
fn functions_by_parameters(
    facts: &[FnFacts],
    min_parameters: usize,
) -> BTreeMap<Group<'_>, Vec<usize>> {
    let mut found: BTreeMap<Group<'_>, Vec<usize>> = BTreeMap::new();
    for (index, function) in facts.iter().enumerate() {
        let parameters: Group<'_> = function
            .param_list
            .iter()
            .filter(|parameter| !parameter.name.starts_with('_'))
            .filter_map(|parameter| Some((parameter.name.as_str(), parameter.scalar?)))
            .collect();
        if parameters.len() >= min_parameters {
            found.entry(parameters).or_default().push(index);
        }
    }
    found
}

/// What any two of those sets have in common, which is all a group can be.
fn shared<'f>(sets: &[&Group<'f>], rule: &ValueObjectConfig) -> BTreeSet<Group<'f>> {
    let mut found = BTreeSet::new();
    for (position, first) in sets.iter().enumerate() {
        for second in &sets[position + 1..] {
            let both: Group<'f> = first.intersection(second).copied().collect();
            if both.len() >= rule.min_parameters && any_says_something(&both, &rule.placeholders) {
                found.insert(both);
            }
        }
    }
    for set in sets {
        if any_says_something(set, &rule.placeholders) {
            found.insert((*set).clone());
        }
    }
    found
}

/// Each group in enough functions that no larger such group holds.
fn largest_groups<'f>(
    by_parameters: &BTreeMap<Group<'f>, Vec<usize>>,
    rule: &ValueObjectConfig,
) -> Vec<(Group<'f>, Vec<usize>)> {
    let sets: Vec<&Group<'f>> = by_parameters.keys().collect();
    let frequent: Vec<(Group<'f>, Vec<usize>)> = shared(&sets, rule)
        .into_iter()
        .filter_map(|group| {
            let holders = by_parameters
                .iter()
                .filter(|(set, _)| group.is_subset(set))
                .flat_map(|(_, functions)| functions.iter().copied());
            let mut holders: Vec<usize> = holders.collect();
            holders.sort_unstable();
            (holders.len() >= rule.min_functions).then_some((group, holders))
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

pub(super) fn add(facts: &[FnFacts], rule: &ValueObjectConfig, found: &mut [Vec<Diagnostic>]) {
    let by_parameters = functions_by_parameters(facts, rule.min_parameters);
    for (group, functions) in largest_groups(&by_parameters, rule) {
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
                limit: rule.min_functions.saturating_sub(1),
            };
            found[index].push(diagnostic(&facts[index], RULE, message, measure));
        }
    }
}

#[cfg(test)]
mod tests;
