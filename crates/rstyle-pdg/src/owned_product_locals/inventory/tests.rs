//! Actual Cargo-captured statement controls; these mint no helper/ownership proof.
use super::*;

fn fixture() -> crate::HirFnFacts {
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("raw-wrapper-capture.json")).unwrap();
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.source.bytes = raw["source"].as_str().unwrap().into();
    facts.body = serde_json::from_value(raw["body"].clone()).unwrap();
    facts
}

fn expected(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    let plan = source::plan(facts).unwrap();
    let index = SourceIndex::new(&facts.body).unwrap();
    plan.guards
        .iter()
        .map(|span| index.unique(*span, "expr_if").unwrap())
        .chain([index.unique(plan.declaration, "stmt_let").unwrap()])
        .chain(
            plan.updates
                .iter()
                .map(|update| index.unique(update.span, "expr_assign").unwrap()),
        )
        .chain([index.unique(plan.terminal, "expr_call").unwrap()])
        .collect()
}

#[test]
fn actual_capture_preserves_both_statement_wrappers_and_original_order() {
    let facts = fixture();
    assert_eq!(facts.body.children[0].kind, "stmt_expr");
    assert_eq!(facts.body.children[2].kind, "stmt_semi");
    check_statements(&facts.body, &expected(&facts), &facts).unwrap();
    let branch = &facts.body.children[0].children[0].children[1];
    let returned = statement_child(&branch.children[0], &facts).unwrap();
    assert_eq!(returned.kind, "expr_ret");
}

#[test]
fn captured_wrappers_reject_extra_children_wrong_identity_and_wrong_order() {
    let facts = fixture();
    let roles = expected(&facts);
    let mut extra = facts.body.clone();
    let child = extra.children[0].children[0].clone();
    extra.children[0].children.push(child);
    assert!(
        check_statements(&extra, &roles, &facts)
            .unwrap_err()
            .contains("hidden operands")
    );
    let mut changed = facts.body.clone();
    changed.children[0].children[0].hir.local += 1000;
    assert!(
        check_statements(&changed, &roles, &facts)
            .unwrap_err()
            .contains("evaluation order")
    );
    let mut reordered = facts.body.clone();
    reordered.children.swap(0, 1);
    assert!(
        check_statements(&reordered, &roles, &facts)
            .unwrap_err()
            .contains("evaluation order")
    );
}

#[test]
fn captured_wrapper_source_and_nested_return_cannot_hide_operands() {
    let facts = fixture();
    let roles = expected(&facts);
    let mut changed = facts.body.clone();
    changed.children[0].span.start += 1;
    assert!(check_statements(&changed, &roles, &facts).is_err());
    changed = facts.body.clone();
    changed.children[0].kind = "stmt_semi".into();
    assert!(check_statements(&changed, &roles, &facts).is_err());
    let mut nested = facts.body.children[0].children[0].children[1].children[0].clone();
    nested.children.push(nested.children[0].clone());
    assert!(
        statement_child(&nested, &facts)
            .unwrap_err()
            .contains("hidden operands")
    );
}
