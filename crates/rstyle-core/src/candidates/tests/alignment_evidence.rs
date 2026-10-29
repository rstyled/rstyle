use super::*;
use crate::facts::{EffectEvent, EffectKind};

const WARNING: &str = "Some graph nodes are unmatched; recorded substitutions describe aligned regions, not complete functions.";
const PRESERVE: &str = "Preserve behavior outside the aligned graph regions.";
const KINDS: [CandidateKind; 2] = [CandidateKind::Parameterize, CandidateKind::GenericFn];

fn facts(kind: CandidateKind, members: usize) -> Vec<FnFacts> {
    if kind == CandidateKind::GenericFn {
        let mut facts = type_only_corpus(["Dog", "Cat"]);
        if members == 3 {
            facts.insert(2, type_only("Cow"));
            facts.push(getter("Cow", "own"));
        }
        return facts;
    }
    ["A", "B", "C"]
        .into_iter()
        .take(members)
        .map(|name| external_only(&format!("ext::{name}"), &format!("ext::{name}::go")))
        .collect()
}

fn cluster(kind: CandidateKind, coverage: &[u32], empty_columns: bool) -> Cluster {
    let (hole, var, values) = if kind == CandidateKind::GenericFn {
        (HoleKind::Type, "T0", vec!["k::Dog", "k::Cat", "k::Cow"])
    } else {
        (
            HoleKind::FreeFn,
            "F0",
            vec!["ext::A::go", "ext::B::go", "ext::C::go"],
        )
    };
    let columns = if empty_columns {
        vec![]
    } else {
        vec![Column {
            kind: hole,
            var: var.into(),
            values: values
                .into_iter()
                .take(coverage.len())
                .map(String::from)
                .collect(),
        }]
    };
    Cluster {
        forwarding_only: false,
        members: (0..coverage.len()).collect(),
        coverage_milli: coverage.to_vec(),
        columns,
    }
}

fn raw_proposal(facts: &[FnFacts], kind: CandidateKind, cluster: &Cluster) -> Candidate {
    let groups = mine_groups(facts);
    let ix = Index::new(facts, &groups, &Params::default());
    let sites: Vec<_> = facts.iter().take(cluster.members.len()).collect();
    let mut candidate = match kind {
        CandidateKind::GenericFn => generic_fn(&ix, &sites, cluster),
        _ => parameterize(&ix, &sites, cluster),
    }
    .expect("existing helper proposal policy");
    candidate.id = ledger::candidate_id(candidate.kind, &candidate.sites, &candidate.definitions);
    candidate
}

fn retain(candidate: Candidate) -> Candidate {
    Mined::default()
        .with(Outcome::Ready(Box::new(candidate)))
        .candidates
        .pop()
        .expect("completed proposal")
}

fn assert_caveat_preserves_proposal(before: Candidate) {
    let after = retain(before.clone());
    assert_eq!(after.id, before.id);
    assert_eq!(after.score_milli, before.score_milli);
    assert_eq!(after.breakdown, before.breakdown);
    assert_eq!(after.sites, before.sites);
    assert_eq!(after.definitions, before.definitions);
    assert!(
        after
            .counter_evidence
            .iter()
            .any(|warning| warning == WARNING)
    );
    assert!(after.possible_refactor.ends_with(PRESERVE));
}

#[test]
fn complete_alignment_describes_regions_without_unmatched_warning() {
    for kind in KINDS {
        let facts = facts(kind, 2);
        let cluster = cluster(kind, &[1000, 1000], false);
        let candidate = retain(raw_proposal(&facts, kind, &cluster));
        assert_eq!(candidate.breakdown.coverage_milli, 1000);
        assert!(candidate.observation.starts_with(
            "2 functions have aligned dependence-graph regions; recorded substitutions: "
        ));
        assert!(
            !candidate
                .counter_evidence
                .iter()
                .any(|warning| warning == WARNING)
        );
        assert!(!candidate.possible_refactor.contains(PRESERVE));
    }
}

#[test]
fn partial_alignment_uses_independent_mean_and_preserves_identity_scores_and_roles() {
    let cases: &[(&[u32], u32)] = &[
        (&[1000, 800], 900),
        (&[1000, 1000, 800], 933),
        (&[1000, 1000, 999], 999),
    ];
    for kind in KINDS {
        for &(coverage, expected_mean) in cases {
            let facts = facts(kind, coverage.len());
            let cluster = cluster(kind, coverage, false);
            let candidate = raw_proposal(&facts, kind, &cluster);
            assert_eq!(candidate.breakdown.coverage_milli, expected_mean);
            assert_caveat_preserves_proposal(candidate);
        }
    }
}

#[test]
fn zero_column_alignment_never_claims_an_exact_clone() {
    let kind = CandidateKind::Parameterize;
    let facts = facts(kind, 2);
    for coverage in [[1000, 1000], [1000, 800]] {
        let cluster = cluster(kind, &coverage, true);
        let candidate = retain(raw_proposal(&facts, kind, &cluster));
        assert!(
            candidate
                .observation
                .ends_with("recorded substitutions: no recorded substitutions")
        );
        assert!(
            candidate
                .inference
                .contains("aligned graph regions may support a shared computation")
        );
        assert!(!candidate.possible_refactor.contains("exact clone"));
        assert_eq!(
            candidate.possible_refactor.ends_with(PRESERVE),
            coverage[1] < 1000
        );
    }
}

#[test]
fn unmatched_alignment_caveat_keeps_existing_recorded_effect_warning() {
    for kind in KINDS {
        let mut facts = facts(kind, 2);
        facts[0].effects.push(EffectEvent {
            kind: EffectKind::Unknown,
            detail: "unresolved effect".into(),
            line: 1,
        });
        let cluster = cluster(kind, &[1000, 800], false);
        let candidate = retain(raw_proposal(&facts, kind, &cluster));
        assert!(
            candidate
                .counter_evidence
                .iter()
                .any(|warning| warning == WARNING)
        );
        assert!(candidate.counter_evidence.iter().any(|warning| {
            warning.starts_with("recorded effect mismatch") && warning.contains("unknown")
        }));
    }
}

#[test]
fn trait_proposals_receive_the_same_partial_alignment_caveat() {
    let facts = corpus();
    let groups = mine_groups(&facts);
    let ix = Index::new(&facts, &groups, &Params::default());
    let fns: Vec<_> = facts.iter().collect();
    let cluster = Cluster {
        forwarding_only: false,
        members: vec![2, 3],
        coverage_milli: vec![1000, 800],
        columns: vec![Column {
            kind: HoleKind::Method,
            var: "M0".into(),
            values: facts[..2].iter().map(|fact| fact.id.clone()).collect(),
        }],
    };
    let mined = Mined::default().with(from_cluster(&ix, &fns, &cluster));
    let candidate = &mined.candidates[0];
    assert_eq!(candidate.kind, CandidateKind::TraitMethod);
    assert_eq!(candidate.breakdown.coverage_milli, 900);
    assert!(
        candidate
            .counter_evidence
            .iter()
            .any(|warning| warning == WARNING)
    );
    assert!(candidate.possible_refactor.ends_with(PRESERVE));
}
