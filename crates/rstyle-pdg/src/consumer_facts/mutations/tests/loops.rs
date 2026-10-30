use super::*;
use syn::spanned::Spanned;

const FOR: &str = "pub fn f(target:&mut u32){for _ in 0u32..100u32 {*target=1;}}";
const TRANSFER: &str = "pub fn f(target:&mut u32){loop {*target=1;break;*target=2;}}";

fn original_span(source: &str, value: &impl Spanned) -> ByteSpan {
    crate::source_evidence::original_span(source, value.span())
}

pub(in crate::consumer_facts::mutations) fn loop_fixture(source: &str) -> HirFnFacts {
    let mut facts = fixture(source, &[]);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    facts.signature.sig_span = crate::consumer_facts::header_span(source, &function);
    let registry = crate::data_library::registry();
    facts.build.rustc_commit = crate::known_str(&registry.rustc_commit);
    facts.build.target_triple = crate::known_str(&registry.target);
    facts.build.pointer_width = crate::known_count(registry.pointer_width);
    let mut nodes = Nodes { source, next: 100 };
    facts.body.children =
        loop_statements(&mut nodes, &function.block, &facts.bindings[0].hir, None);
    capture_control_types(&mut facts);
    facts
}

fn loop_statements(
    nodes: &mut Nodes<'_>,
    block: &syn::Block,
    root: &HirRef,
    target: Option<&HirRef>,
) -> Vec<HirNode> {
    block
        .stmts
        .iter()
        .filter_map(|statement| {
            let syn::Stmt::Expr(expression, _) = statement else {
                return None;
            };
            Some(loop_expression(nodes, expression, root, target))
        })
        .collect()
}

fn loop_expression(
    nodes: &mut Nodes<'_>,
    expression: &syn::Expr,
    root: &HirRef,
    target: Option<&HirRef>,
) -> HirNode {
    let span = original_span(nodes.source, expression);
    match expression {
        syn::Expr::Loop(_) => loop_node(nodes, "expr_loop", expression, root),
        syn::Expr::While(_) => loop_node(nodes, "expr_while", expression, root),
        syn::Expr::ForLoop(_) => loop_node(nodes, "expr_for", expression, root),
        syn::Expr::Assign(_) => {
            let text = crate::source_evidence::byte_range(nodes.source, span).unwrap();
            nodes.assignment(text, root)
        }
        syn::Expr::Break(_) | syn::Expr::Continue(_) => {
            let kind = if matches!(expression, syn::Expr::Break(_)) {
                "expr_break"
            } else {
                "expr_continue"
            };
            let mut node = nodes.node(NodeType::new(kind, Some("never")), span, vec![]);
            node.res = Some(PathRes {
                kind: "innermost_loop".into(),
                target: target.cloned(),
                description: String::new(),
            });
            node
        }
        _ => nodes.node(NodeType::new("expr_lit", Some("bool")), span, vec![]),
    }
}

fn loop_node(
    nodes: &mut Nodes<'_>,
    kind: &'static str,
    expression: &syn::Expr,
    root: &HirRef,
) -> HirNode {
    let body = match expression {
        syn::Expr::Loop(value) => &value.body,
        syn::Expr::While(value) => &value.body,
        syn::Expr::ForLoop(value) => &value.body,
        _ => unreachable!(),
    };
    let mut node = nodes.node(
        NodeType::new(kind, Some("unit")),
        original_span(nodes.source, expression),
        vec![],
    );
    let mut target = node.hir.clone();
    if let syn::Expr::ForLoop(value) = expression {
        target.local += 10000;
        node.res = Some(PathRes {
            kind: "innermost_loop".into(),
            target: Some(target.clone()),
            description: String::new(),
        });
        node.children.push(range_node(nodes, &value.expr));
        node.children.push(nodes.node(
            NodeType::new("pat_wild", Some("u32")),
            original_span(nodes.source, &value.pat),
            vec![],
        ));
    }
    if let syn::Expr::While(value) = expression {
        node.children
            .push(loop_expression(nodes, &value.cond, root, None));
    }
    let children = loop_statements(nodes, body, root, Some(&target));
    node.children.push(nodes.node(
        NodeType::new("expr_block", Some("unit")),
        original_span(nodes.source, body),
        children,
    ));
    node
}

fn range_node(nodes: &mut Nodes<'_>, expression: &syn::Expr) -> HirNode {
    let syn::Expr::Range(range) = expression else {
        panic!("fixture requires range");
    };
    let children = [&range.start, &range.end]
        .iter()
        .map(|endpoint| {
            nodes.node(
                NodeType::new("expr_lit", Some("u32")),
                original_span(nodes.source, endpoint.as_ref().unwrap()),
                vec![],
            )
        })
        .collect();
    let mut node = nodes.node(
        NodeType::new("expr_struct", None),
        original_span(nodes.source, range),
        children,
    );
    let registry = crate::data_library::registry();
    let callee = crate::data_library::Callee {
        path: "core::ops::Range".into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions["std::ops::Range"].clone(),
        implementation: None,
    };
    node.res = Some(PathRes {
        kind: "resolved_u32_range_type".into(),
        target: None,
        description: serde_json::to_string(&callee).unwrap(),
    });
    node
}

#[test]
fn loops_count_original_sites_once_even_after_transfers_or_zero_iterations() {
    for source in [
        FOR,
        TRANSFER,
        "pub fn f(target:&mut u32){while false {*target=1;continue;*target=2;}}",
        "pub fn f(target:&mut u32){for _ in 0u32..0u32 {*target=1;}}",
        "pub fn f(target:&mut u32){loop {while false {*target=1;break;} *target=2;break;}}",
    ] {
        let facts = loop_fixture(source);
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Complete, "{source}");
        let count = source.matches("*target=").count();
        assert_eq!(result.sites.len(), count, "{source}");
        assert!(
            result
                .sites
                .iter()
                .all(|site| site.root == result.sites[0].root && site.caller_visible)
        );
    }
}

#[test]
fn loop_child_order_cardinality_and_owner_contradictions_error() {
    for attack in 0..5 {
        let mut facts = loop_fixture(FOR);
        let node = &mut facts.body.children[0];
        match attack {
            0 => {
                node.children.swap(0, 1);
            }
            1 => {
                node.children.pop();
            }
            2 => {
                node.children[2].kind = "expr_call".into();
            }
            3 => {
                node.hir.owner += 1;
            }
            _ => {
                node.children[0].children.pop();
            }
        }
        assert!(assess(&facts).is_err(), "attack {attack}");
    }
}

#[test]
fn unavailable_loop_control_head_and_binder_types_prevent_complete_inventory() {
    for missing in 0..7 {
        let mut facts = loop_fixture(FOR);
        let node = &mut facts.body.children[0];
        match missing {
            0 => node.ty = None,
            1 => node.children[0].ty = None,
            2 => node.children[1].ty = None,
            3 => node.children[2].ty = None,
            4 => node.res = None,
            5 => node.children[0].res = None,
            _ => node.children[0].children[0].ty = None,
        }
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete, "missing {missing}");
        assert!(result.sites.is_empty(), "missing {missing}");
    }
}

#[test]
fn range_scalar_normalization_and_nested_implementation_cannot_mint_authority() {
    for normalized in ["bool", "u32", "never"] {
        let mut facts = loop_fixture(FOR);
        facts.body.children[0].children[0]
            .ty
            .as_mut()
            .unwrap()
            .normalized = Some(normalized.into());
        assert!(assess(&facts).is_err(), "{normalized}");
    }
    for attack in 0..3 {
        let mut facts = loop_fixture(FOR);
        let resolution = facts.body.children[0].children[0].res.as_mut().unwrap();
        let mut callee: crate::data_library::Callee =
            serde_json::from_str(&resolution.description).unwrap();
        match attack {
            0 => callee.implementation = Some(Box::new(callee.clone())),
            1 => callee.crate_hash.push('x'),
            _ => callee.definition_hash.push('x'),
        }
        resolution.description = serde_json::to_string(&callee).unwrap();
        assert!(assess(&facts).is_err(), "attack {attack}");
    }
}

#[test]
fn range_marker_and_selected_registry_remain_independent_requirements() {
    for missing in 0..2 {
        let mut facts = loop_fixture(FOR);
        if missing == 0 {
            facts.body.children[0].children[0]
                .res
                .as_mut()
                .unwrap()
                .kind = "type".into();
        } else {
            facts.build.rustc_commit = crate::known_str("unreviewed");
        }
        assert_eq!(assess(&facts).unwrap().coverage, Coverage::Incomplete);
    }
}

#[test]
fn transfer_targets_and_native_children_are_checked_against_current_loop() {
    for attack in 0..3 {
        let mut facts = loop_fixture(TRANSFER);
        let transfer = &mut facts.body.children[0].children[0].children[1];
        match attack {
            0 => {
                transfer
                    .res
                    .as_mut()
                    .unwrap()
                    .target
                    .as_mut()
                    .unwrap()
                    .local += 1
            }
            1 => transfer.res.as_mut().unwrap().kind = "label".into(),
            _ => {
                let mut extra = transfer.clone();
                extra.hir.local += 1000;
                transfer.children.push(extra);
            }
        }
        assert!(assess(&facts).is_err(), "attack {attack}");
    }
    let mut facts = loop_fixture(TRANSFER);
    facts.body.children[0].children[0].children[1]
        .res
        .as_mut()
        .unwrap()
        .target = None;
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert_eq!(result.sites.len(), 2);
}

#[test]
fn while_missing_or_contradictory_condition_type_stays_distinct() {
    let source = "pub fn f(target:&mut u32){while true {*target=1;}}";
    let mut facts = loop_fixture(source);
    facts.body.children[0].children[0].ty = None;
    assert_eq!(assess(&facts).unwrap().coverage, Coverage::Incomplete);
    let mut facts = loop_fixture(source);
    facts.body.children[0].children[0].ty = Some(ty(Some("u32")));
    assert!(assess(&facts).is_err());
}

#[test]
fn direct_loop_inventory_does_not_complete_call_or_use_summaries() {
    let facts = loop_fixture(FOR);
    for observations in [
        inspect_calls(&facts).unwrap(),
        inspect_calls_with_uses(&facts).unwrap(),
    ] {
        assert_eq!(observations.direct.coverage, Coverage::Incomplete);
        assert!(observations.direct.sites.is_empty());
    }
}

#[test]
fn expanded_loop_head_binder_body_and_transfers_do_not_certify_original_sites() {
    for expanded in 0..5 {
        let mut facts = loop_fixture(TRANSFER);
        if expanded < 4 {
            facts = loop_fixture(FOR);
        }
        let node = &mut facts.body.children[0];
        let selected = match expanded {
            0 => node,
            1 => &mut node.children[0],
            2 => &mut node.children[1],
            3 => &mut node.children[2],
            _ => &mut node.children[0].children[1],
        };
        selected.expansion = Some(crate::ExpansionSite {
            callsite: selected.span,
            snippet: selected.snippet.clone(),
        });
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete, "expanded {expanded}");
        let expected = if expanded == 4 { 2 } else { 0 };
        assert_eq!(result.sites.len(), expected, "expanded {expanded}");
    }
}

#[test]
fn labels_and_value_transfers_remain_outside_direct_loop_support() {
    for source in [
        "pub fn f(target:&mut u32){'outer: loop {*target=1;break;}}",
        "pub fn f(target:&mut u32){loop {*target=1;break 1u32;*target=2;}}",
    ] {
        let result = assess(&loop_fixture(source)).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete, "{source}");
    }
}

fn native_count(node: &HirNode) -> usize {
    1 + node.children.iter().map(native_count).sum::<usize>()
}

#[test]
fn loop_native_inventory_accepts_exact_limit_and_exhausts_one_node_later() {
    let mut facts = loop_fixture(FOR);
    let count = native_count(&facts.body)
        + facts
            .signature
            .params
            .iter()
            .map(|parameter| native_count(&parameter.pat))
            .sum::<usize>();
    let mut padding = facts.body.children[0].children[0].children[0].clone();
    padding.kind = "inventory_padding".into();
    for local in 0..16384 - count {
        padding.hir.local = 100000 + local as u32;
        facts.body.children.push(padding.clone());
    }
    assert_eq!(assess(&facts).unwrap().coverage, Coverage::Complete);
    padding.hir.local = 200000;
    facts.body.children.push(padding);
    let result = assess(&facts).unwrap();
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert!(result.sites.is_empty());
}

#[test]
fn loop_binding_inventory_accepts_exact_limit_and_exhausts_one_binding_later() {
    let mut facts = loop_fixture(FOR);
    let mut unused = facts.bindings[0].clone();
    for local in 1..16384 {
        unused.hir.local = 100000 + local;
        facts.bindings.push(unused.clone());
    }
    assert_eq!(assess(&facts).unwrap().coverage, Coverage::Complete);
    unused.hir.local = 200000;
    facts.bindings.push(unused);
    assert_eq!(
        assess(&facts).unwrap().execution,
        Execution::BudgetExhausted
    );
}

fn capture_control_types(facts: &mut HirFnFacts) {
    use crate::consumer_facts::control_types::{Control, IDENTITY, KEY, Observations};
    let mut never = Vec::new();
    let mut pending = vec![&mut facts.body];
    while let Some(node) = pending.pop() {
        if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) == Some("never") {
            node.ty.as_mut().unwrap().normalized = None;
            never.push(Control {
                hir: node.hir.clone(),
                span: node.span,
                kind: node.kind.clone(),
            });
        }
        pending.extend(&mut node.children);
    }
    if !never.is_empty() {
        facts.build.frontend_options.insert(
            KEY.into(),
            serde_json::to_value(Observations {
                identity: IDENTITY.into(),
                never,
            })
            .unwrap(),
        );
    }
}

#[test]
fn never_control_observations_require_exact_original_nodes_and_types() {
    use crate::consumer_facts::control_types::KEY;
    let original = loop_fixture(TRANSFER);
    for attack in 0..8 {
        let mut facts = original.clone();
        let capture = facts.build.frontend_options.get_mut(KEY).unwrap();
        match attack {
            0 => capture["identity"] = "another-schema".into(),
            1 => capture["never"][0]["hir"]["owner"] = 999.into(),
            2 => capture["never"][0]["span"]["start"] = 0.into(),
            3 => capture["never"][0]["kind"] = "expr_call".into(),
            4 => {
                let duplicate = capture["never"][0].clone();
                capture["never"].as_array_mut().unwrap().push(duplicate);
            }
            5 => capture["never"][0]["trusted"] = true.into(),
            6 => {
                facts.body.children[0].children[0].children[1]
                    .ty
                    .as_mut()
                    .unwrap()
                    .normalized = Some("u32".into())
            }
            _ => {
                capture["never"][0]["hir"]["owner"] = 999.into();
                facts.body.children[0].children[0].children[1].hir.owner = 999;
            }
        }
        assert!(assess(&facts).is_err(), "attack {attack}");
    }
}

#[test]
fn missing_never_control_observation_stays_unknown_without_display_inference() {
    use crate::consumer_facts::control_types::KEY;
    let mut facts = loop_fixture(TRANSFER);
    facts.build.frontend_options.remove(KEY);
    let transfer = &mut facts.body.children[0].children[0].children[1];
    transfer.ty.as_mut().unwrap().observed = "!".into();
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert_eq!(result.sites.len(), 2);
}
