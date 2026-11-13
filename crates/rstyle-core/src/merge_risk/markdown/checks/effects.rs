//! Effects: what changed functions gained or lost, what new functions do, and what only moved.

use super::super::{Links, code, name, nonzero, place};
use super::{Check, Status, counted};
use crate::merge_risk::{ChangeKind, ChangeReport, EffectChange, FunctionChange};
use std::collections::{BTreeMap, BTreeSet};

/// The effects a new function has that a reviewer should hear of by name.
const NOTEWORTHY_FOR_NEW: [&str; 4] = ["io", "network", "global", "unsafe"];
const UNKNOWN: &str = "unknown";

/// An effect and the function that has it.
type Owned<'c> = (&'c FunctionChange, &'c EffectChange);

pub(in super::super) fn effects(report: &ChangeReport, links: &Links) -> Check {
    let found = Found::of(&report.changes);
    let Some(summary) = found.summary() else {
        let item = "no function gained or lost io, network, global state, mutation or unsafe code";
        return Check::fine("Effects", "none gained or lost", item.into());
    };
    Check::new("Effects", found.status(), summary, found.items(links))
}

/// The change's effects, by what a reviewer makes of each.
struct Found<'c> {
    /// What functions that were there before gained.
    gains: Vec<Owned<'c>>,
    /// What they lost, that is known.
    losses: Vec<Owned<'c>>,
    /// What new functions do that a reviewer should hear of by name.
    noteworthy: Vec<Owned<'c>>,
    /// Calls new functions make that nothing classifies.
    unclassified: Vec<Owned<'c>>,
    moved: Vec<Moved<'c>>,
    new_functions: usize,
}

impl<'c> Found<'c> {
    fn of(changes: &'c [FunctionChange]) -> Self {
        let Moves {
            gained,
            lost,
            moved,
        } = Moves::of(changes);
        let (new, gains): (Vec<Owned<'c>>, Vec<Owned<'c>>) = gained
            .into_iter()
            .partition(|(owner, _)| owner.kind == ChangeKind::Added);
        let of_kind = |kinds: &[&str]| -> Vec<Owned<'c>> {
            let wanted = new
                .iter()
                .filter(|(_, effect)| kinds.contains(&effect.kind));
            wanted.copied().collect()
        };
        Self {
            noteworthy: of_kind(&NOTEWORTHY_FOR_NEW),
            unclassified: of_kind(&[UNKNOWN]),
            gains,
            losses: lost
                .into_iter()
                .filter(|(_, effect)| effect.kind != UNKNOWN)
                .collect(),
            moved,
            new_functions: changes
                .iter()
                .filter(|change| change.kind == ChangeKind::Added)
                .count(),
        }
    }

    fn summary(&self) -> Option<String> {
        let mut parts = counted(&[
            (self.gains.len(), "gained"),
            (self.losses.len(), "lost"),
            (self.moved.len(), "moved between functions"),
        ]);
        if let Some(new) = nonzero(self.new_functions, "new function", "new functions") {
            parts.push(format!("{new}, {}", what_new_does(&self.noteworthy)));
        }
        if let Some(count) = nonzero(self.unclassified.len(), "call", "calls") {
            parts.push(format!("{count} nothing classifies"));
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// A known effect gained or lost, or a new function's io, is something to look at; an effect
    /// nothing classifies is unknown.
    fn status(&self) -> Status {
        let known_gain = self.gains.iter().any(|(_, effect)| effect.kind != UNKNOWN);
        let unknown_gain = self.gains.iter().any(|(_, effect)| effect.kind == UNKNOWN);
        let look = known_gain || !self.losses.is_empty() || !self.noteworthy.is_empty();
        Status::of(look, unknown_gain || !self.unclassified.is_empty())
    }

    fn items(&self, links: &Links) -> Vec<String> {
        let said = |found: &[Owned<'_>], verb: &str| -> Vec<String> {
            let items = found
                .iter()
                .map(|(owner, effect)| effect_item(owner, verb, effect, links));
            items.collect()
        };
        let unclassified = self.unclassified.iter().map(|(owner, effect)| {
            let callee = code(&name(&effect.label));
            format!(
                "{} is new and calls {callee}, which nothing classifies",
                place(owner, links)
            )
        });
        let moved = self.moved.iter().map(|moved| moved.item(links));
        [
            said(&self.gains, "gains"),
            said(&self.losses, "loses"),
            said(&self.noteworthy, "is new and does"),
        ]
        .concat()
        .into_iter()
        .chain(unclassified)
        .chain(moved)
        .collect()
    }
}

fn effect_item(owner: &FunctionChange, verb: &str, effect: &EffectChange, links: &Links) -> String {
    let label = code(&name(&effect.label));
    format!("{} {verb} {}: {label}", place(owner, links), effect.kind)
}

/// Which new functions do io, network, global state or unsafe code, and which.
fn what_new_does(noteworthy: &[Owned<'_>]) -> String {
    let mut doers: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (owner, effect) in noteworthy {
        doers
            .entry(owner.name.as_str())
            .or_default()
            .insert(effect.kind);
    }
    if doers.is_empty() {
        return "none does io or network".into();
    }
    let said = doers.into_iter().map(|(owner, kinds)| {
        let kinds: Vec<&str> = kinds.into_iter().collect();
        format!("{} does {}", code(&name(owner)), kinds.join(" and "))
    });
    said.collect::<Vec<_>>().join("; ")
}

/// An effect one function lost and another gained, under the same name or the same last part of
/// it, as `typed` becomes `f.typed`.
struct Moved<'c> {
    from: &'c FunctionChange,
    to: &'c FunctionChange,
    lost: &'c EffectChange,
    gained: &'c EffectChange,
}

impl Moved<'_> {
    fn item(&self, links: &Links) -> String {
        let renamed = (self.lost.label != self.gained.label)
            .then(|| format!(" as {}", code(&name(&self.gained.label))));
        format!(
            "{} {} moved from {} to {}{}",
            self.lost.kind,
            code(&name(&self.lost.label)),
            place(self.from, links),
            place(self.to, links),
            renamed.unwrap_or_default()
        )
    }
}

/// The effects the change gained and lost on balance, and those that only moved.
struct Moves<'c> {
    gained: Vec<Owned<'c>>,
    lost: Vec<Owned<'c>>,
    moved: Vec<Moved<'c>>,
}

impl<'c> Moves<'c> {
    /// Pairs each lost effect with a gained one by the same label first, then by the same last
    /// part of it. A pair in one function cancels out; across two, it moved.
    fn of(changes: &'c [FunctionChange]) -> Self {
        let owned = |pick: fn(&FunctionChange) -> &Vec<EffectChange>| -> Vec<Owned<'c>> {
            let each = changes
                .iter()
                .flat_map(move |change| pick(change).iter().map(move |effect| (change, effect)));
            each.collect()
        };
        let mut gained = owned(|change| &change.effects_gained);
        let mut lost = owned(|change| &change.effects_lost);
        let mut moved = Vec::new();
        for same_label in [true, false] {
            lost.retain(|&(from, effect)| {
                let Some(at) = gained
                    .iter()
                    .position(|(_, other)| same(effect, other, same_label))
                else {
                    return true;
                };
                let (to, other) = gained.remove(at);
                if !std::ptr::eq(from, to) {
                    moved.push(Moved {
                        from,
                        to,
                        lost: effect,
                        gained: other,
                    });
                }
                false
            });
        }
        Self {
            gained,
            lost,
            moved,
        }
    }
}

/// Whether a lost effect and a gained one are the same effect: the same kind, with the same label,
/// or, failing that, the same last part of one.
fn same(lost: &EffectChange, gained: &EffectChange, same_label: bool) -> bool {
    let last = |label: &str| label.rsplit('.').next().unwrap_or(label).to_owned();
    lost.kind == gained.kind
        && match same_label {
            true => lost.label == gained.label,
            false => last(&lost.label) == last(&gained.label),
        }
}
