//! Calls: library calls the change adds, by what they do, and those it drops.

use super::super::{Links, code, distinct_names, name, place};
use super::{Check, Status, counted};
use crate::merge_risk::{ChangeKind, ChangeReport, FunctionChange};
use std::collections::{BTreeMap, BTreeSet};

/// How many times the change's functions call a callee, with one of them.
type Counted<'c> = BTreeMap<&'c String, (usize, &'c FunctionChange)>;
/// A callee and a function that calls it.
type Calls<'c> = Vec<(&'c String, &'c FunctionChange)>;

/// What a new call does: an effect's kind, `unknown` when nothing classifies it, or nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Does {
    Effect(&'static str),
    Unknown,
    Nothing,
}

impl Does {
    /// What a new call's line says it does.
    fn label(self) -> &'static str {
        match self {
            Self::Effect(kind) => kind,
            Self::Unknown => "effect unknown",
            Self::Nothing => "no effect",
        }
    }
}

pub(in super::super) fn calls(report: &ChangeReport, links: &Links) -> Check {
    let (gained, lost) = library_calls(report);
    if gained.is_empty() && lost.is_empty() {
        let item = "no changed function added or dropped a call outside this repository";
        return Check::fine("Calls", "no library calls added or removed", item.into());
    }

    let names = shown_names(&gained, &lost);
    let shown = |callee: &String| code(&names[callee.as_str()]);
    let does = what_calls_do(&report.changes);
    let of = |wanted: fn(Does) -> bool| -> Calls<'_> {
        gained
            .iter()
            .filter(|(callee, _)| wanted(does(callee)))
            .copied()
            .collect()
    };
    let with_effect = of(|found| matches!(found, Does::Effect(_)));
    let unknown = of(|found| found == Does::Unknown);
    let quiet = of(|found| found == Does::Nothing);

    let said = |calls: &Calls<'_>| -> Vec<String> {
        let items = calls.iter().map(|(callee, owner)| {
            format!(
                "new, {}: {} in {}",
                does(callee).label(),
                shown(callee),
                place(owner, links)
            )
        });
        items.collect()
    };
    let items = [
        said(&with_effect),
        said(&unknown),
        said(&quiet),
        removed_items(&lost, &shown, links),
    ];
    let summary = counted(&[
        (with_effect.len(), "with effects"),
        (unknown.len(), "effect unknown"),
        (quiet.len(), "no effects"),
        (lost.len(), "removed"),
    ]);
    let status = Status::of(!with_effect.is_empty(), !unknown.is_empty());
    Check::new("Calls", status, summary.join(", "), items.concat())
}

/// What each callee the check names reads as, told apart from any other with the same short name.
fn shown_names<'c>(gained: &Calls<'c>, lost: &Calls<'c>) -> BTreeMap<&'c str, String> {
    let callees = gained.iter().chain(lost).map(|(callee, _)| callee.as_str());
    distinct_names(callees)
}

/// The library callees the change calls more of, and less of: a callee that isn't a function of
/// either run is a library's.
fn library_calls(report: &ChangeReport) -> (Calls<'_>, Calls<'_>) {
    let added = counted_calls(&report.changes, |change| &change.calls_added);
    let removed = counted_calls(&report.changes, |change| &change.calls_removed);
    let workspace = &report.workspace;
    (
        library(more(&added, &removed), workspace),
        library(more(&removed, &added), workspace),
    )
}

fn library<'c>(calls: Calls<'c>, workspace: &BTreeSet<String>) -> Calls<'c> {
    let outside = calls
        .into_iter()
        .filter(|(callee, _)| !workspace.contains(*callee));
    outside.collect()
}

fn counted_calls(
    changes: &[FunctionChange],
    pick: fn(&FunctionChange) -> &Vec<String>,
) -> Counted<'_> {
    let mut counts = Counted::new();
    for change in changes {
        for callee in pick(change) {
            counts.entry(callee).or_insert((0, change)).0 += 1;
        }
    }
    counts
}

/// Each callee called more times in `of` than in `than`, with a function that calls it: a call
/// that moved from one function to another is neither new nor removed.
fn more<'c>(of: &Counted<'c>, than: &Counted<'c>) -> Calls<'c> {
    let fewer = |callee: &String| than.get(callee).map_or(0, |(count, _)| *count);
    of.iter()
        .filter(|(callee, (count, _))| *count > fewer(callee))
        .map(|(callee, (_, owner))| (*callee, *owner))
        .collect()
}

/// What each new call does: unknown when nothing classifies it, else the kind of effect named for
/// it, else nothing.
fn what_calls_do(changes: &[FunctionChange]) -> impl Fn(&String) -> Does + '_ {
    let unclassified: BTreeSet<&String> = changes
        .iter()
        .flat_map(|change| &change.calls_unclassified)
        .collect();
    let effects: BTreeMap<String, &'static str> = changes
        .iter()
        .flat_map(|change| change.effects_gained.iter())
        .map(|effect| (name(&effect.label), effect.kind))
        .collect();
    move |callee| match (
        unclassified.contains(callee),
        effects.get(name(callee).as_str()),
    ) {
        (true, _) | (false, Some(&"unknown")) => Does::Unknown,
        (false, Some(kind)) => Does::Effect(kind),
        (false, None) => Does::Nothing,
    }
}

/// Calls dropped with a function the change removed read as one line per function.
fn removed_items(
    lost: &Calls<'_>,
    shown: &dyn Fn(&String) -> String,
    links: &Links,
) -> Vec<String> {
    let mut with_function: BTreeMap<&str, (&FunctionChange, Vec<String>)> = BTreeMap::new();
    let mut items = Vec::new();
    for (callee, owner) in lost {
        let callee = shown(callee);
        match owner.kind {
            ChangeKind::Removed => {
                let (_, callees) = with_function
                    .entry(owner.function.as_str())
                    .or_insert((owner, Vec::new()));
                callees.push(callee);
            }
            _ => items.push(format!("removed: {callee} in {}", place(owner, links))),
        }
    }
    let gone = with_function.into_values().map(|(owner, callees)| {
        format!(
            "removed with {}: {}",
            place(owner, links),
            callees.join(", ")
        )
    });
    items.extend(gone);
    items
}
