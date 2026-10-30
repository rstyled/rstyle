//! Compare already recorded effect kinds; this module never classifies or propagates calls.
use super::{Candidate, CandidateKind};
use crate::{facts::FnFacts, rules::Effect};
use std::collections::{BTreeMap, BTreeSet};

type Class = BTreeSet<&'static str>;
const RECORDED_EMPTY: &str = "no recorded intrinsic or escaping-mutation effects";

/// Notes differences in recorded intrinsic effects and escaping mutations.
/// Calls, configured classifications and propagation are outside this comparison.
pub(super) fn effect_mismatch(defs: &[&FnFacts]) -> Option<String> {
    let classes = defs
        .iter()
        .map(|f| {
            let kinds = f.effects.iter().map(|e| e.kind.name());
            kinds
                .chain(f.mutations.iter().any(|m| m.escapes).then_some("mutation"))
                .collect()
        })
        .collect::<Vec<Class>>();
    mismatch(
        defs.iter().map(|f| f.name.as_str()),
        &classes,
        "recorded effect mismatch",
        RECORDED_EMPTY,
    )
}

fn mismatch<'a>(
    names: impl Iterator<Item = &'a str>,
    classes: &[Class],
    prefix: &str,
    empty: &str,
) -> Option<String> {
    if classes.windows(2).all(|pair| pair[0] == pair[1]) {
        return None;
    }
    let parts = names
        .zip(classes)
        .map(|(name, class)| {
            let show = if class.is_empty() {
                empty.to_string()
            } else {
                class.iter().copied().collect::<Vec<_>>().join("+")
            };
            format!("{name} is {show}")
        })
        .collect::<Vec<_>>();
    Some(format!("{prefix}: {}", parts.join(", ")))
}

/// Add display-only helper warnings from the same analysis's completed core effects.
/// Original owner IDs must refer to the same compilation unit and policy. Missing results
/// are errors; empty recorded kinds do not prove purity. Scores and membership stay fixed.
pub fn annotate_completed_effects(
    candidates: &mut [Candidate],
    completed: &BTreeMap<String, Vec<Effect>>,
) -> Result<(), String> {
    for candidate in candidates.iter_mut().filter(|candidate| {
        matches!(
            candidate.kind,
            CandidateKind::Parameterize | CandidateKind::GenericFn
        )
    }) {
        let classes = candidate
            .sites
            .iter()
            .map(|site| {
                completed
                    .get(&site.id)
                    .map(|effects| effects.iter().map(|effect| effect.kind.name()).collect())
                    .ok_or_else(|| format!("Missing completed compiler effects for {}", site.id))
            })
            .collect::<Result<Vec<Class>, String>>()?;
        annotate(candidate, &classes);
    }
    Ok(())
}

fn annotate(candidate: &mut Candidate, classes: &[Class]) {
    let names = || candidate.sites.iter().map(|site| site.name.as_str());
    let Some(recorded) = mismatch(names(), classes, "recorded effect mismatch", RECORDED_EMPTY)
    else {
        return;
    };
    if candidate.counter_evidence.contains(&recorded) {
        return;
    }
    if let Some(note) = mismatch(
        names(),
        classes,
        "core-policy effect mismatch",
        "no recorded effects under this policy",
    ) && !candidate.counter_evidence.contains(&note)
    {
        candidate.counter_evidence.push(note);
    }
}
