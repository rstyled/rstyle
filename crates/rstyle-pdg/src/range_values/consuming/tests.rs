//! Independent source and affine-use attacks; these tests grant no activation.
use super::*;
use crate::{HirFnFacts, HirNode};

fn original() -> HirFnFacts {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../product_instances/conversion/fixture.json"
    ))
    .unwrap();
    crate::unit_store::read_diagnostic_snapshot(&serde_json::to_vec(&fixture["snapshot"]).unwrap())
        .unwrap()
        .pop()
        .unwrap()
}

fn owned_read(node: &mut HirNode) {
    node.ty.as_mut().unwrap().normalized = Some(super::super::TOKEN.into());
}

fn source_field(expression: &str) -> syn::FieldValue {
    let literal: syn::ExprStruct = syn::parse_str(&format!("Span {{ {expression} }}")).unwrap();
    literal.fields.into_iter().next().unwrap()
}

#[test]
fn exact_source_selectors_are_ordered_and_never_convert_or_borrow() {
    for member in ["start", "end"] {
        fields::source_field(
            &source_field(&format!("{member}: range.{member}")),
            member,
            "range",
        )
        .unwrap();
    }
    for changed in [
        "start: range.end",
        "end: range.start",
        "start: other.start",
        "start: &range.start",
        "start: range.start + 1",
        "start: f(range.start)",
        "start",
        "start: (range.start)",
    ] {
        assert!(
            fields::source_field(&source_field(changed), "start", "range").is_err(),
            "{changed}"
        );
    }
}

#[test]
fn owned_range_source_type_excludes_references_and_other_scalar_instantiations() {
    for ty in [
        "Range<usize>",
        "core::ops::Range<usize>",
        "std::ops::Range<usize>",
    ] {
        assert!(source::range_type(
            &syn::parse_str::<syn::Type>(ty).unwrap()
        ));
    }
    for ty in [
        "&Range<usize>",
        "&mut Range<usize>",
        "Range<u64>",
        "RangeFrom<usize>",
        "Range<(usize,)>",
        "(Range<usize>,)",
        "Range<usize, usize>",
    ] {
        assert!(
            !source::range_type(&syn::parse_str::<syn::Type>(ty).unwrap()),
            "{ty}"
        );
    }
}

#[test]
fn original_transfer_has_exact_first_initializer_and_shadowing() {
    let facts = original();
    let formal = &facts.signature.params[1];
    let call = facts.body.children[0].children.last().unwrap();
    transfer::source(&facts, formal, call).unwrap();
    for replacement in [
        "(&span).into()",
        "span.clone().into()",
        "into(span)",
        "span.into::<Span>()",
    ] {
        let mut changed = facts.clone();
        changed.source.bytes = facts
            .source
            .bytes
            .replace("span.into()", replacement)
            .into();
        assert!(
            transfer::source(&changed, formal, call).is_err(),
            "{replacement}"
        );
    }
}

#[test]
fn one_owned_whole_read_excludes_duplicate_transfer_and_use_after_move() {
    let mut facts = original();
    let receiver = &mut facts.body.children[0].children.last_mut().unwrap().children[0];
    owned_read(receiver);
    let formal = &facts.signature.params[1];
    let mut paths = Vec::new();
    transfer::uses(&facts.body, formal, &mut paths).unwrap();
    assert_eq!(paths.len(), 1);
    let receiver = paths[0].clone();
    let mut duplicated = facts.body.clone();
    duplicated.children.push(receiver);
    let mut paths = Vec::new();
    transfer::uses(&duplicated, formal, &mut paths).unwrap();
    assert_eq!(paths.len(), 2);
}

#[test]
fn branches_loops_mutations_and_wrong_formal_reads_have_no_affine_permission() {
    let mut facts = original();
    let formal = &facts.signature.params[1];
    let receiver = &mut facts.body.children[0].children.last_mut().unwrap().children[0];
    owned_read(receiver);
    for kind in [
        "expr_loop",
        "expr_if",
        "expr_match",
        "expr_closure",
        "expr_assign",
        "expr_assign_op",
        "expr_await",
        "expr_yield",
    ] {
        let mut changed = facts.body.clone();
        changed.kind = kind.into();
        assert!(
            transfer::uses(&changed, formal, &mut Vec::new()).is_err(),
            "{kind}"
        );
    }
    let mut changed = facts.body.clone();
    changed.children[0].children.last_mut().unwrap().children[0].kind = "expr_addr_of".into();
    assert!(transfer::uses(&changed, formal, &mut Vec::new()).is_err());
}

#[test]
fn raw_generic_capture_cannot_mint_owned_range_evidence() {
    let facts = original();
    let call = facts.body.children[0].children.last().unwrap();
    assert!(verify_transfer(&facts, call, &facts).is_err());
    assert!(verify_fields(&facts).is_err());
}
