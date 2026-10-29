use super::*;
use crate::facts::{EffectEvent, EffectKind};

const KINDS: [CandidateKind; 2] = [CandidateKind::Parameterize, CandidateKind::GenericFn];

fn proposal(kind: CandidateKind, raw: Option<EffectKind>) -> Candidate {
    let mut facts = if kind == CandidateKind::GenericFn {
        type_only_corpus(["Dog", "Cat"])
    } else {
        vec![
            external_only("ext::A", "ext::A::go"),
            external_only("ext::B", "ext::B::go"),
        ]
    };
    for (owner, name) in facts[..2].iter_mut().zip(["first", "second"]) {
        owner.id = format!("krate::{name}");
    }
    if let Some(kind) = raw {
        facts[0].effects.push(EffectEvent {
            kind,
            detail: "raw intrinsic".into(),
            line: 1,
        });
    }
    run(&facts)
        .candidates
        .into_iter()
        .find(|candidate| candidate.kind == kind)
        .expect("existing helper proposal")
}

fn effects(kinds: &[EffectKind]) -> Vec<crate::rules::Effect> {
    kinds
        .iter()
        .map(|&kind| crate::rules::Effect {
            kind,
            label: "completed policy evidence".into(),
            line: 7,
            via: Some("completed callee".into()),
        })
        .collect()
}

fn completed(
    candidate: &Candidate,
    first: &[EffectKind],
    second: &[EffectKind],
) -> BTreeMap<String, Vec<crate::rules::Effect>> {
    BTreeMap::from([
        (candidate.sites[0].id.clone(), effects(first)),
        (candidate.sites[1].id.clone(), effects(second)),
    ])
}

fn assert_only_notes_change(before: &Candidate, after: &Candidate) {
    let mut expected = before.clone();
    expected.counter_evidence = after.counter_evidence.clone();
    assert_eq!(&expected, after);
}

#[test]
fn completed_policy_mismatch_changes_only_helper_counter_evidence() {
    for kind in KINDS {
        let mut candidate = proposal(kind, None);
        let before = candidate.clone();
        let classes = completed(&candidate, &[EffectKind::Io], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert_only_notes_change(&before, &candidate);
        assert!(candidate.counter_evidence.iter().any(|note| {
            note.starts_with("core-policy effect mismatch")
                && note.contains("io")
                && note.contains("no recorded effects under this policy")
        }));
    }
}

#[test]
fn completed_equal_classes_ignore_event_multiplicity_and_keep_empty_evidence_distinct() {
    for kind in KINDS {
        for effect_kinds in [vec![], vec![EffectKind::Io]] {
            let mut candidate = proposal(kind, None);
            let before = candidate.clone();
            let mut repeated = effect_kinds.clone();
            repeated.extend(effect_kinds.iter().copied());
            let classes = completed(&candidate, &effect_kinds, &repeated);
            annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
            assert_eq!(candidate, before);
        }
    }
}

#[test]
fn explicit_completed_unknown_is_not_equivalent_to_no_recorded_effects() {
    for kind in KINDS {
        let mut candidate = proposal(kind, None);
        let classes = completed(&candidate, &[EffectKind::Unknown], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert!(candidate.counter_evidence.iter().any(|note| {
            note.starts_with("core-policy effect mismatch")
                && note.contains("unknown")
                && note.contains("no recorded effects under this policy")
        }));
    }
}

#[test]
fn completed_effect_lookup_uses_exact_owner_ids_even_when_source_names_match() {
    for kind in KINDS {
        let mut candidate = proposal(kind, None);
        for site in &mut candidate.sites {
            site.name = "same source name".into();
        }
        let classes = completed(&candidate, &[EffectKind::Io], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert!(
            candidate
                .counter_evidence
                .iter()
                .any(|note| note.starts_with("core-policy effect mismatch"))
        );
        let wrong_keys = BTreeMap::from([("same source name".into(), Vec::new())]);
        assert!(
            annotate_completed_effects(std::slice::from_mut(&mut candidate), &wrong_keys).is_err()
        );
    }
}

#[test]
fn every_selected_helper_owner_requires_a_completed_entry_including_empty_classes() {
    for kind in KINDS {
        let mut candidate = proposal(kind, None);
        let incomplete = BTreeMap::from([(candidate.sites[0].id.clone(), Vec::new())]);
        assert!(
            annotate_completed_effects(std::slice::from_mut(&mut candidate), &incomplete).is_err()
        );
        let complete = completed(&candidate, &[], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &complete).unwrap();
    }
}

#[test]
fn equivalent_raw_mismatch_keeps_its_existing_note_without_redundant_policy_note() {
    for kind in KINDS {
        let mut candidate = proposal(kind, Some(EffectKind::Io));
        let before = candidate.clone();
        let classes = completed(&candidate, &[EffectKind::Io], &[]);
        assert!(
            before
                .counter_evidence
                .iter()
                .any(|note| note.starts_with("recorded effect mismatch"))
        );
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert_eq!(candidate, before);
    }
}

#[test]
fn supplied_completed_classes_can_disagree_with_raw_intrinsic_classes() {
    for kind in KINDS {
        let mut candidate = proposal(kind, Some(EffectKind::Io));
        let before = candidate.clone();
        let classes = completed(&candidate, &[EffectKind::Time], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert_only_notes_change(&before, &candidate);
        assert!(candidate.counter_evidence.iter().any(|note| {
            note.starts_with("core-policy effect mismatch") && note.contains("time")
        }));
        assert!(
            candidate.counter_evidence.iter().any(|note| {
                note.starts_with("recorded effect mismatch") && note.contains("io")
            })
        );
    }
}

#[test]
fn repeating_annotation_does_not_repeat_a_completed_policy_warning() {
    for kind in KINDS {
        let mut candidate = proposal(kind, None);
        let classes = completed(&candidate, &[EffectKind::Time], &[]);
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        let first = candidate.clone();
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &classes).unwrap();
        assert_eq!(candidate, first);
    }
}

#[test]
fn non_helper_proposal_families_do_not_require_or_consume_completed_owner_classes() {
    let original = proposal(CandidateKind::GenericFn, None);
    for kind in [
        CandidateKind::TraitMethod,
        CandidateKind::CapabilitySet,
        CandidateKind::EnumDispatch,
        CandidateKind::FnParam,
    ] {
        let mut candidate = original.clone();
        candidate.kind = kind;
        let before = candidate.clone();
        annotate_completed_effects(std::slice::from_mut(&mut candidate), &BTreeMap::new()).unwrap();
        assert_eq!(candidate, before);
    }
}
