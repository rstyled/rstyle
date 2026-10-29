use super::*;
use crate::facts::{EffectEvent, EffectKind, MutationOrigin};

const KINDS: [CandidateKind; 2] = [CandidateKind::Parameterize, CandidateKind::GenericFn];

fn facts(kind: CandidateKind) -> Vec<FnFacts> {
    match kind {
        CandidateKind::GenericFn => type_only_corpus(["Dog", "Cat"]),
        CandidateKind::Parameterize => vec![
            external_only("ext::A", "ext::A::go"),
            external_only("ext::B", "ext::B::go"),
        ],
        _ => unreachable!("only the two helper proposal families are under test"),
    }
}

fn candidate(facts: &[FnFacts], kind: CandidateKind) -> Candidate {
    run(facts)
        .candidates
        .into_iter()
        .find(|candidate| candidate.kind == kind)
        .expect("existing helper proposal")
}

fn mutation(escapes: bool) -> MutationEvent {
    MutationEvent {
        origin: MutationOrigin::Source,
        root: "state".into(),
        field_path: "value".into(),
        line: 1,
        escapes,
        delegated: false,
    }
}

fn assert_same_proposal(before: &Candidate, after: &Candidate) {
    assert_eq!(before.id, after.id);
    assert_eq!(before.kind, after.kind);
    assert_eq!(before.score_milli, after.score_milli);
    assert_eq!(before.breakdown, after.breakdown);
    assert_eq!(before.sites, after.sites);
    assert_eq!(before.definitions, after.definitions);
}

#[test]
fn helper_effect_mismatch_preserves_identity_score_and_member_roles() {
    for kind in KINDS {
        let mut facts = facts(kind);
        let before = candidate(&facts, kind);
        facts[0].effects.push(EffectEvent {
            kind: EffectKind::Io,
            detail: "recorded IO".into(),
            line: 1,
        });
        let after = candidate(&facts, kind);
        assert_same_proposal(&before, &after);
        assert!(after.counter_evidence.iter().any(|warning| {
            warning.starts_with("recorded effect mismatch")
                && warning.contains("io")
                && warning.contains("no recorded intrinsic or escaping-mutation effects")
        }));
    }
}

#[test]
fn local_mutation_does_not_add_recorded_effect_mismatch() {
    for kind in KINDS {
        let mut facts = facts(kind);
        let before = candidate(&facts, kind);
        facts[0].mutations.push(mutation(false));
        let after = candidate(&facts, kind);
        assert_same_proposal(&before, &after);
        assert_eq!(before.counter_evidence, after.counter_evidence);
    }
}

#[test]
fn equal_escaping_mutation_classes_do_not_add_effect_mismatch() {
    for kind in KINDS {
        let mut facts = facts(kind);
        let before = candidate(&facts, kind);
        for owner in &mut facts[..2] {
            owner.mutations.push(mutation(true));
        }
        let after = candidate(&facts, kind);
        assert_same_proposal(&before, &after);
        assert_eq!(before.counter_evidence, after.counter_evidence);
    }
}

#[test]
fn unknown_effect_differs_from_no_recorded_effect_evidence() {
    for kind in KINDS {
        let mut facts = facts(kind);
        let before = candidate(&facts, kind);
        facts[0].effects.push(EffectEvent {
            kind: EffectKind::Unknown,
            detail: "unresolved effect".into(),
            line: 1,
        });
        let after = candidate(&facts, kind);
        assert_same_proposal(&before, &after);
        assert!(after.counter_evidence.iter().any(|warning| {
            warning.starts_with("recorded effect mismatch")
                && warning.contains("unknown")
                && warning.contains("no recorded intrinsic or escaping-mutation effects")
        }));
    }
}

#[test]
fn one_escaping_mutation_adds_evidence_without_changing_the_proposal() {
    for kind in KINDS {
        let mut facts = facts(kind);
        let before = candidate(&facts, kind);
        facts[0].mutations.push(mutation(true));
        let after = candidate(&facts, kind);
        assert_same_proposal(&before, &after);
        assert!(after.counter_evidence.iter().any(|warning| {
            warning.starts_with("recorded effect mismatch")
                && warning.contains("is mutation")
                && warning.contains("no recorded intrinsic or escaping-mutation effects")
        }));
    }
}
