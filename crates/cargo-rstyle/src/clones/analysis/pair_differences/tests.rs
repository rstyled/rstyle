// Fixture-only entry point; production constructs each checked endpoint once.
fn inspect<'a>(left: &'a HirFnFacts, right: &'a HirFnFacts) -> Assessment<'a> {
    let graph = admitted(Some("fixture-context"));
    let left = Endpoint::from_native(&graph, left);
    let right = Endpoint::from_native(&graph, right);
    explain(&left, &right)
}

use super::*;
use rstyle_pdg::{ByteSpan, Dispatch, HirRef, PathRes, ResolvedTy, SourceText};

fn ty() -> ResolvedTy {
    ResolvedTy {
        normalized: Some("u32".into()),
        observed: "u32".into(),
        spelling: "u32".into(),
    }
}

fn node(kind: &str, local: u32) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local,
        },
        kind: kind.into(),
        span: ByteSpan { start: 0, end: 0 },
        snippet: String::new(),
        ty: Some(ty()),
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: vec![],
    }
}

fn read(local: u32, parameter: u32) -> HirNode {
    let mut value = node("expr_path", local);
    value.res = Some(PathRes {
        kind: "local".into(),
        target: Some(HirRef {
            krate: 0,
            owner: 1,
            local: parameter,
        }),
        description: "local".into(),
    });
    value
}

fn literal(local: u32, text: &str) -> HirNode {
    let mut value = node("expr_lit", local);
    value.lit = Some(text.into());
    value
}

fn binary(op: &str, left: HirNode, right: HirNode) -> HirNode {
    let mut value = node("expr_binary", 100);
    value.dispatch = Some(Dispatch {
        kind: "builtin".into(),
        op: op.into(),
        operand_tys: vec!["u32".into(); 2],
        result_ty: "u32".into(),
    });
    value.children = vec![left, right];
    value
}

fn facts(body: HirNode) -> HirFnFacts {
    // Reuse only transport/build boilerplate; native cases below are authored
    // independently and do not assert that synthetic inputs passed admission.
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../../../../rstyle-pdg/src/nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    )).unwrap();
    facts.source.bytes = SourceText::from("fn f(left: u32, right: u32) -> u32 { left - right }");
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.signature.return_ty = ty();
    facts.signature.params = (0..2)
        .map(|position| {
            let mut pat = node("pat_binding", position + 1);
            let symbol = format!("symbol:{position}");
            pat.binding = Some(symbol.clone());
            ParamFact {
                symbol,
                position,
                name_span: pat.span,
                ty_span: pat.span,
                ty: ty(),
                pat,
            }
        })
        .collect();
    facts.body = body;
    facts
}

#[test]
fn literal_hole_and_binder_rename_preserve_formal_role() {
    let left = facts(binary("+", read(10, 1), literal(11, "1")));
    let mut right = facts(binary("+", read(10, 1), literal(11, "2")));
    right.signature.params[0].symbol = "renamed".into();
    right.signature.params[0].pat.binding = Some("renamed".into());
    let result = inspect(&left, &right);
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.correspondence.len(), 3);
    assert_eq!(result.holes.len(), 1);
    assert_eq!(result.holes[0].kind, HoleKind::Literal);
    assert_eq!(result.holes[0].left.lit.as_deref(), Some("1"));
    assert_eq!(result.holes[0].right.lit.as_deref(), Some("2"));
}

#[test]
fn same_expression_is_known_empty() {
    let facts = facts(binary("+", read(10, 1), literal(11, "1")));
    let result = inspect(&facts, &facts);
    assert_eq!(result.coverage, Coverage::Complete);
    assert!(result.holes.is_empty());
    assert!(result.reason.is_none());
}

#[test]
fn swapped_subtraction_retains_both_ordered_role_holes() {
    let left = facts(binary("-", read(10, 1), read(11, 2)));
    let right = facts(binary("-", read(10, 2), read(11, 1)));
    let result = inspect(&left, &right);
    assert_eq!(result.holes.len(), 2);
    assert!(
        result
            .holes
            .iter()
            .all(|hole| hole.kind == HoleKind::FormalRole)
    );
    assert_eq!(result.correspondence[1].left.local, 10);
    assert_eq!(result.correspondence[1].right.local, 10);
    assert_eq!(result.holes[0].role, Role::BinaryLeft);
    assert_eq!(result.holes[1].role, Role::BinaryRight);
}

#[test]
fn repeated_operands_are_occurrences_not_ambiguous_label_classes() {
    let left = facts(binary("-", read(10, 1), read(11, 1)));
    let result = inspect(&left, &left);
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.correspondence.len(), 3);
    assert!(result.holes.is_empty());
}

#[test]
fn unsupported_roster_cannot_return_partial_clean_correspondence() {
    let left = facts(binary("+", read(10, 1), literal(11, "1")));
    for kind in ["expr_call", "expr_closure", "stmt_let"] {
        let mut right = left.clone();
        right.body.children.push(node(kind, 12));
        let result = inspect(&left, &right);
        assert_eq!(result.coverage, Coverage::Unsupported);
        assert!(result.correspondence.is_empty());
        assert!(result.reason.is_some());
    }
}

#[test]
fn wrong_dispatch_type_or_missing_binding_is_unsupported() {
    let left = facts(binary("+", read(10, 1), literal(11, "1")));
    let mut wrong_dispatch = left.clone();
    wrong_dispatch.body.dispatch.as_mut().unwrap().kind = "overloaded".into();
    let mut wrong_type = left.clone();
    wrong_type.body.children[0].ty.as_mut().unwrap().normalized = Some("u64".into());
    let mut missing = left.clone();
    missing.body.children[0].res.as_mut().unwrap().target = None;
    for right in [wrong_dispatch, wrong_type, missing] {
        assert_eq!(inspect(&left, &right).coverage, Coverage::Unsupported);
    }
}

#[test]
fn duplicate_nodes_and_formal_roles_are_ambiguous() {
    let left = facts(binary("-", read(10, 1), read(11, 2)));
    let mut duplicate_node = left.clone();
    duplicate_node.body.children[1].hir = duplicate_node.body.children[0].hir.clone();
    let mut duplicate_formal = left.clone();
    duplicate_formal.signature.params[1].position = 0;
    for right in [duplicate_node, duplicate_formal] {
        let result = inspect(&left, &right);
        assert_eq!(result.coverage, Coverage::Ambiguous);
        assert!(result.holes.is_empty());
    }
}

fn admitted(context: Option<&str>) -> ImportedFunction {
    ImportedFunction {
        matching_profile: rstyle_ccgraph::MatchingProfile::DataV1,
        extraction_context: context.map(str::to_owned),
        key: "fixture".into(),
        nodes: vec![],
        edges: vec![],
        inputs: vec![],
        outputs: vec![],
        name: Some("fixture".into()),
        reference_count: Some(0),
        target_pointer_width: Some(64),
        native_width_involved: false,
    }
}

#[test]
fn missing_or_different_context_is_incomplete() {
    let facts = facts(binary("+", read(10, 1), literal(11, "1")));
    let left_graph = admitted(Some("context-a"));
    let left = Endpoint::from_native(&left_graph, &facts);
    for context in [None, Some(""), Some("context-b")] {
        let graph = admitted(context);
        let right = Endpoint::from_native(&graph, &facts);
        let result = explain(&left, &right);
        assert_eq!(result.coverage, Coverage::Unsupported);
        assert!(result.correspondence.is_empty());
    }
}

#[test]
fn matching_context_still_reports_differences_without_equivalence_claim() {
    let left_facts = facts(binary("+", read(10, 1), literal(11, "1")));
    let right_facts = facts(binary("+", read(10, 1), literal(11, "2")));
    let graph = admitted(Some("context-a"));
    let left = Endpoint::from_native(&graph, &left_facts);
    let right = Endpoint::from_native(&graph, &right_facts);
    let result = explain(&left, &right);
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.holes[0].kind, HoleKind::Literal);
}

#[test]
fn native_width_flag_requires_known_equal_widths_on_both_endpoints() {
    let facts = facts(binary("+", read(10, 1), literal(11, "1")));
    for (left_width, right_width) in [
        (Some(64), Some(32)),
        (None, Some(64)),
        (Some(64), None),
        (None, None),
    ] {
        let mut left_graph = admitted(Some("context-a"));
        left_graph.native_width_involved = true;
        left_graph.target_pointer_width = left_width;
        let mut right_graph = admitted(Some("context-a"));
        right_graph.target_pointer_width = right_width;
        let result = explain(
            &Endpoint::from_native(&left_graph, &facts),
            &Endpoint::from_native(&right_graph, &facts),
        );
        assert_eq!(result.coverage, Coverage::Unsupported);
        assert!(result.correspondence.is_empty());
        assert!(result.reason.unwrap().contains("target widths"));
    }
}

fn native_facts() -> HirFnFacts {
    let mut facts = facts(binary("+", read(10, 1), literal(11, "1")));
    facts.signature.return_ty.normalized = Some("usize".into());
    for parameter in &mut facts.signature.params {
        parameter.ty.normalized = Some("usize".into());
        parameter.pat.ty.as_mut().unwrap().normalized = Some("usize".into());
    }
    fn convert(node: &mut HirNode) {
        node.ty.as_mut().unwrap().normalized = Some("usize".into());
        if let Some(dispatch) = &mut node.dispatch {
            dispatch.operand_tys = vec!["usize".into(); 2];
            dispatch.result_ty = "usize".into();
        }
        for child in &mut node.children {
            convert(child);
        }
    }
    convert(&mut facts.body);
    facts
}

#[test]
fn normalized_native_tokens_require_width_even_without_graph_flag() {
    let facts = native_facts();
    let left_graph = admitted(Some("context-a"));
    let mut right_graph = admitted(Some("context-a"));
    right_graph.target_pointer_width = Some(32);
    let result = explain(
        &Endpoint::from_native(&left_graph, &facts),
        &Endpoint::from_native(&right_graph, &facts),
    );
    assert_eq!(result.coverage, Coverage::Unsupported);
}

#[test]
fn matching_known_native_widths_allow_closed_native_comparison() {
    let facts = native_facts();
    let mut graph = admitted(Some("context-a"));
    graph.native_width_involved = true;
    let endpoint = Endpoint::from_native(&graph, &facts);
    let result = explain(&endpoint, &endpoint);
    assert_eq!(result.coverage, Coverage::Complete);
    assert!(result.holes.is_empty());
}

#[test]
fn depth_and_parameter_budgets_have_explicit_negative_outcomes() {
    let mut body = literal(10, "1");
    for local in 20..86 {
        let mut block = node("expr_block", local);
        block.children.push(body);
        body = block;
    }
    let deep = facts(body);
    let result = inspect(&deep, &deep);
    assert_eq!(result.coverage, Coverage::BudgetExhausted);
    assert!(result.correspondence.is_empty());
    let mut many = facts(literal(10, "1"));
    many.signature.params = vec![many.signature.params[0].clone(); 65];
    assert_eq!(inspect(&many, &many).coverage, Coverage::BudgetExhausted);
}
