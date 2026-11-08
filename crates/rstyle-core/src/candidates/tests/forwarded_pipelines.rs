use super::*;
use crate::shape::fixtures::adt;

fn pipeline(name: &str, callees: &[&str]) -> FnFacts {
    let mut facts = sample(name);
    let mut input = node(NodeKind::Param);
    input.ty = Some(TyShape::Prim("u32".into()));
    let mut nodes = vec![input];
    for callee in callees {
        nodes.push(call(
            callee,
            method(
                "ext::Step",
                Receiver::None,
                vec![TyShape::Prim("u32".into())],
                TyShape::Prim("u32".into()),
            ),
        ));
    }
    nodes.push(node(NodeKind::Return));
    let edges = (0..nodes.len() - 1)
        .map(|index| data(index, index + 1, 0))
        .collect();
    facts.pdg = Some(Pdg { nodes, edges });
    facts
}

fn pair() -> Vec<FnFacts> {
    vec![
        pipeline("first", &["ext::A::first", "ext::A::second"]),
        pipeline("second", &["ext::B::first", "ext::B::second"]),
    ]
}

fn proposes_helper(facts: &[FnFacts]) -> bool {
    run(facts)
        .candidates
        .iter()
        .any(|candidate| candidate.kind == CandidateKind::Parameterize)
}

fn add_computation(facts: &mut FnFacts, kind: NodeKind) {
    let pdg = facts.pdg.as_mut().expect("source-like pipeline graph");
    let extra = pdg.nodes.len();
    let mut computation = node(kind);
    computation.detail = "extra".into();
    if kind == NodeKind::Lit {
        computation.lit_kind = Some("int".into());
        computation.detail = "1".into();
    }
    pdg.nodes.push(computation);
    pdg.edges.push(data(extra, 2, 1));
}

#[test]
fn differing_two_and_three_call_forwarding_pipelines_are_not_helper_proposals() {
    assert!(!proposes_helper(&pair()));
    let facts = [
        pipeline(
            "first",
            &["ext::A::first", "ext::A::second", "ext::A::third"],
        ),
        pipeline(
            "second",
            &["ext::B::first", "ext::B::second", "ext::B::third"],
        ),
    ];
    assert!(!proposes_helper(&facts));
}

#[test]
fn one_retained_common_call_keeps_real_shared_pipeline_work_visible() {
    let facts = [
        pipeline("first", &["ext::shared", "ext::A::second"]),
        pipeline("second", &["ext::shared", "ext::B::second"]),
    ];
    assert!(proposes_helper(&facts));
}

#[test]
fn repeated_differing_callee_is_not_a_single_use_forwarding_substitution() {
    let facts = [
        pipeline("first", &["ext::A::step", "ext::A::step"]),
        pipeline("second", &["ext::B::step", "ext::B::step"]),
    ];
    assert!(proposes_helper(&facts));
}

#[test]
fn recorded_computation_or_control_nodes_keep_the_proposal_outside_forwarding_only() {
    for kind in [
        NodeKind::Lit,
        NodeKind::Field,
        NodeKind::Op,
        NodeKind::Iterate,
        NodeKind::Loop,
        NodeKind::Closure,
        NodeKind::Let,
    ] {
        let mut facts = pair();
        for owner in &mut facts {
            add_computation(owner, kind);
        }
        assert!(proposes_helper(&facts), "missing proposal with {kind:?}");
    }
}

#[test]
fn unknown_callee_is_not_treated_as_a_known_forwarded_callable() {
    let mut facts = pair();
    for owner in &mut facts {
        owner.pdg.as_mut().unwrap().nodes[1].callee_id = None;
    }
    assert!(proposes_helper(&facts));
}

#[test]
fn function_item_substitutions_do_not_count_as_forwarded_call_nodes() {
    let mut facts = pair();
    for (owner, callback) in facts
        .iter_mut()
        .zip(["ext::A::callback", "ext::B::callback"])
    {
        add_computation(owner, NodeKind::Lit);
        owner
            .pdg
            .as_mut()
            .unwrap()
            .nodes
            .last_mut()
            .unwrap()
            .callee_id = Some(callback.into());
    }
    assert!(proposes_helper(&facts));
}

#[test]
fn type_holes_keep_fallback_and_generic_proposals_outside_the_forwarding_guard() {
    let mut facts = pair();
    for (owner, ty) in facts.iter_mut().zip(["ext::Left", "ext::Right"]) {
        owner.pdg.as_mut().unwrap().nodes[0].ty = Some(adt(ty));
    }
    assert!(proposes_helper(&facts));
    assert!(
        run(&type_only_corpus(["Dog", "Cat"]))
            .candidates
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::GenericFn)
    );
}

#[test]
fn exact_pipeline_without_substitutions_still_offers_a_shared_helper() {
    let facts = [
        pipeline("first", &["ext::shared_first", "ext::shared_second"]),
        pipeline("second", &["ext::shared_first", "ext::shared_second"]),
    ];
    assert!(proposes_helper(&facts));
}

#[test]
fn node_numbering_does_not_change_forwarding_or_shared_computation_decisions() {
    let mut forwarded = pair();
    let first = forwarded[0].pdg.as_ref().unwrap();
    forwarded[0].pdg = Some(permuted(first, 1));
    assert!(!proposes_helper(&forwarded));
    let mut computed = pair();
    for owner in &mut computed {
        add_computation(owner, NodeKind::Lit);
    }
    let second = computed[1].pdg.as_ref().unwrap();
    computed[1].pdg = Some(permuted(second, 2));
    assert!(proposes_helper(&computed));
}

#[test]
fn one_nonforwarding_member_relation_preserves_the_whole_cluster_proposal() {
    let facts = [
        pipeline("first", &["ext::A::first", "ext::A::second"]),
        pipeline("second", &["ext::B::first", "ext::B::second"]),
        pipeline("third", &["ext::A::first", "ext::C::second"]),
    ];
    let mined = run(&facts);
    assert!(mined.candidates.iter().any(|candidate| {
        candidate.kind == CandidateKind::Parameterize && candidate.sites.len() == 3
    }));
}

#[test]
fn existing_trait_proposals_do_not_use_the_fallback_forwarding_rejection() {
    assert!(
        run(&corpus())
            .candidates
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::TraitMethod)
    );
}
