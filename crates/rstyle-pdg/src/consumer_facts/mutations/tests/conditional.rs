use super::*;
use syn::spanned::Spanned;

fn syntax_span(source: &str, value: &impl Spanned) -> ByteSpan {
    crate::source_evidence::original_span(source, value.span())
}

pub(in crate::consumer_facts::mutations) fn branch_fixture(source: &str) -> HirFnFacts {
    let mut facts = fixture(source, &[]);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    facts.signature.sig_span = crate::consumer_facts::header_span(source, &function);
    let mut nodes = Nodes { source, next: 100 };
    let root = facts.bindings[0].hir.clone();
    if function.sig.inputs.len() == 2 {
        flag_formal(&mut nodes, &mut facts);
    }
    facts.body.children = block_children(&mut nodes, &function.block, &root);
    facts
}

fn flag_formal(nodes: &mut Nodes<'_>, facts: &mut HirFnFacts) {
    let declaration = span(nodes.source, "flag");
    let mut pattern = nodes.node(
        NodeType::new("pat_binding", Some("bool")),
        declaration,
        vec![],
    );
    pattern.param_index = Some(1);
    pattern.binding = Some("s:flag".into());
    pattern.binding_mut = Some(false);
    facts.bindings.push(BindingFact {
        symbol: "s:flag".into(),
        name: "flag".into(),
        decl_span: declaration,
        scope: format!("scope:source:fn:0:{}", nodes.source.len()),
        hir: pattern.hir.clone(),
        is_param: true,
        is_mut: false,
        ty: ty(Some("bool")),
    });
    facts.signature.params.push(ParamFact {
        symbol: "s:flag".into(),
        position: 1,
        name_span: declaration,
        ty_span: span(nodes.source, "bool"),
        ty: ty(Some("bool")),
        pat: pattern,
    });
}

fn block_children(nodes: &mut Nodes<'_>, block: &syn::Block, root: &HirRef) -> Vec<HirNode> {
    block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Expr(expression, _) => Some(expression_node(nodes, expression, root)),
            _ => None,
        })
        .collect()
}

fn expression_node(nodes: &mut Nodes<'_>, expression: &syn::Expr, root: &HirRef) -> HirNode {
    let span = syntax_span(nodes.source, expression);
    match expression {
        syn::Expr::If(branch) => {
            let mut children = vec![
                expression_node(nodes, &branch.cond, root),
                block_node(nodes, &branch.then_branch, root),
            ];
            if let Some((_, alternative)) = &branch.else_branch {
                children.push(expression_node(nodes, alternative, root));
            }
            nodes.node(NodeType::new("expr_if", Some("unit")), span, children)
        }
        syn::Expr::Block(block) => block_node(nodes, &block.block, root),
        syn::Expr::Paren(paren) => {
            let mut node = expression_node(nodes, &paren.expr, root);
            node.span = span;
            node.snippet = crate::source_evidence::byte_range(nodes.source, span)
                .unwrap()
                .into();
            node
        }
        syn::Expr::Assign(assignment) => assignment_node(nodes, assignment, span, root),
        _ => condition_node(nodes, expression, root),
    }
}

fn assignment_node(
    nodes: &mut Nodes<'_>,
    assignment: &syn::ExprAssign,
    span: ByteSpan,
    root: &HirRef,
) -> HirNode {
    let text = crate::source_evidence::byte_range(nodes.source, span).unwrap();
    let mut node = if text.starts_with('*') {
        nodes.assignment(text, root)
    } else {
        nodes.owned_assignment(text, root)
    };
    let right = &mut node.children[1];
    right.span = syntax_span(nodes.source, &*assignment.right);
    right.snippet = crate::source_evidence::byte_range(nodes.source, right.span)
        .unwrap()
        .into();
    if matches!(&*assignment.right, syn::Expr::Path(_)) {
        right.kind = "expr_path".into();
        right.ty = Some(ty(None));
    }
    node
}

fn block_node(nodes: &mut Nodes<'_>, block: &syn::Block, root: &HirRef) -> HirNode {
    let children = block_children(nodes, block, root);
    nodes.node(
        NodeType::new("expr_block", Some("unit")),
        syntax_span(nodes.source, block),
        children,
    )
}

fn condition_node(nodes: &mut Nodes<'_>, expression: &syn::Expr, root: &HirRef) -> HirNode {
    if let syn::Expr::Binary(binary) = expression
        && matches!(binary.op, syn::BinOp::AddAssign(_))
    {
        return compound_condition_node(nodes, expression, binary, root);
    }
    let span = syntax_span(nodes.source, expression);
    let (kind, children) = match expression {
        syn::Expr::Binary(binary) => (
            "expr_binary",
            vec![
                expression_node(nodes, &binary.left, root),
                expression_node(nodes, &binary.right, root),
            ],
        ),
        syn::Expr::Unary(unary) => (
            "expr_unary",
            vec![expression_node(nodes, &unary.expr, root)],
        ),
        syn::Expr::Path(_) => ("expr_path", vec![]),
        syn::Expr::Call(_) => ("expr_call", vec![]),
        syn::Expr::Lit(_) => ("expr_lit", vec![]),
        _ => ("unsupported", vec![]),
    };
    let mut node = nodes.node(NodeType::new(kind, Some("bool")), span, children);
    if matches!(expression, syn::Expr::Binary(_) | syn::Expr::Unary(_)) {
        node.dispatch = Some(dispatch("builtin-condition", "bool"));
    }
    node
}

fn compound_condition_node(
    nodes: &mut Nodes<'_>,
    expression: &syn::Expr,
    binary: &syn::ExprBinary,
    root: &HirRef,
) -> HirNode {
    let mut left = expression_node(nodes, &binary.left, root);
    let mut right = expression_node(nodes, &binary.right, root);
    left.ty = Some(ty(Some("u32")));
    left.res = Some(PathRes {
        kind: "local".into(),
        target: Some(root.clone()),
        description: String::new(),
    });
    right.ty = Some(ty(Some("u32")));
    let mut node = nodes.node(
        NodeType::new("expr_assign_op", Some("unit")),
        syntax_span(nodes.source, expression),
        vec![left, right],
    );
    node.dispatch = Some(dispatch("+=", "unit"));
    node
}

#[test]
fn ordinary_if_else_counts_both_original_sites_and_one_borrowed_root() {
    let source =
        "pub fn branch(target:&mut u32,flag:bool) { if flag { *target=1; } else { *target=2; } }";
    let facts = branch_fixture(source);
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.execution, Execution::Completed);
    assert_eq!(result.sites.len(), 2);
    assert_eq!(result.sites[0].root, result.sites[1].root);
    assert_eq!(result.sites[0].root.native, facts.bindings[0].hir);
    assert!(result.sites.iter().all(|site| site.caller_visible));
    assert_eq!(result.sites[0].span, span(source, "*target=1"));
    assert_eq!(result.sites[1].span, span(source, "*target=2"));
}

#[test]
fn nested_else_if_and_following_sites_are_static_occurrences() {
    let source = "pub fn f(target:&mut u32){if true {*target=1;} else if false {*target=2;} else {*target=3;} *target=4;}";
    let result = assess(&branch_fixture(source)).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.sites.len(), 4);
    assert!(
        result
            .sites
            .iter()
            .all(|site| site.root == result.sites[0].root)
    );
    let no_else = "pub fn f(target:&mut u32){if !false {*target=1;}}";
    assert_eq!(
        assess(&branch_fixture(no_else)).unwrap().coverage,
        Coverage::Complete
    );
}

#[test]
fn known_native_if_roster_and_condition_contradictions_error() {
    let source = "pub fn f(target:&mut u32){if true {*target=1;} else {*target=2;}}";
    for attack in 0..5 {
        let mut facts = branch_fixture(source);
        let branch = &mut facts.body.children[0];
        match attack {
            0 => {
                branch.children.swap(1, 2);
            }
            1 => {
                branch.children.pop();
            }
            2 => {
                branch.children[1].kind = "expr_if".into();
            }
            3 => {
                branch.children[0].ty.as_mut().unwrap().normalized = Some("u32".into());
            }
            _ => {
                branch.children[2].kind = "expr_call".into();
            }
        }
        assert!(assess(&facts).is_err(), "attack {attack}");
    }
}

#[test]
fn missing_conditional_identity_or_types_remain_unknown() {
    let source = "pub fn f(target:&mut u32){if true {*target=1;} else {*target=2;}}";
    for missing in 0..7 {
        let mut facts = branch_fixture(source);
        let branch = &mut facts.body.children[0];
        match missing {
            0 => {
                branch.kind = "unavailable_if".into();
            }
            1 => {
                branch.children[0].ty = None;
            }
            2 => {
                branch.ty = None;
            }
            3 => {
                branch.children[1].ty = None;
            }
            4 => {
                branch.ty.as_mut().unwrap().normalized = None;
            }
            5 => {
                branch.children[1].ty.as_mut().unwrap().normalized = None;
            }
            _ => {
                branch.children[0].ty.as_mut().unwrap().normalized = None;
            }
        }
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete);
        assert!(result.sites.is_empty());
        assert!(
            result
                .boundaries
                .iter()
                .any(|boundary| boundary.availability == BoundaryAvailability::Unknown)
        );
    }
}

#[test]
fn primitive_condition_child_roster_cannot_be_resealed() {
    let source = "pub fn f(target:&mut u32){if !false {*target=1;}}";
    let mut facts = branch_fixture(source);
    facts.body.children[0].children[0].children.clear();
    assert!(assess(&facts).is_err());
}

#[test]
fn call_conditions_keep_direct_sites_without_complete_coverage() {
    let source = "pub fn f(target:&mut u32){if query() {*target=1;} else {*target=2;}}";
    let result = assess(&branch_fixture(source)).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert_eq!(result.sites.len(), 2);
    assert!(
        result
            .boundaries
            .iter()
            .any(|boundary| boundary.reason == "Unknown call boundary")
    );
}

#[test]
fn conditional_reference_redirection_cannot_restore_borrowed_roots() {
    for prefix in [
        "target=other;",
        "swap(&mut target);",
        "unsafe { redirect(); }",
        "opaque!();",
    ] {
        let source = format!(
            "pub fn f(mut target:&mut u32){{if true {{{prefix} *target=1;}} else {{*target=2;}} *target=3;}}"
        );
        let facts = branch_fixture(&source);
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete, "{prefix}");
        assert!(result.sites.is_empty(), "{prefix}");
    }
}

#[test]
fn direct_branch_support_does_not_complete_call_or_formal_use_scans() {
    let source = "pub fn f(target:&mut u32,flag:bool){if flag {*target=1;} else {*target=2;}}";
    let facts = branch_fixture(source);
    for observed in [
        inspect_calls(&facts).unwrap(),
        inspect_calls_with_uses(&facts).unwrap(),
    ] {
        assert_eq!(observed.direct.coverage, Coverage::Incomplete);
        assert!(observed.direct.sites.is_empty());
    }
}

#[test]
fn owned_formal_branch_rebinding_stays_local_to_the_callee() {
    let source = "pub fn f(mut target:u32){if true {target=1;} else {target=2;}}";
    let result = assess(&branch_fixture(source)).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.sites.len(), 2);
    assert!(
        result
            .sites
            .iter()
            .all(|site| !site.caller_visible && site.root.provenance == Provenance::FormalOwned)
    );
}

#[test]
fn sibling_branch_declarations_preserve_distinct_lexical_roots() {
    let source = "pub fn f(target:&mut u32){if true {let mut value=0u32;value=1;} else {let mut value=2u32;value=3;} *target=4;}";
    let mut facts = branch_fixture(source);
    let mut nodes = Nodes { source, next: 1000 };
    for (arm, declaration, literal, assignment) in [
        (1, "mut value=0u32", "0u32", "value=1"),
        (2, "mut value=2u32", "2u32", "value=3"),
    ] {
        let scope = facts.body.children[0].children[arm].span;
        let (pattern, binding) = local_binding(&mut nodes, declaration, scope);
        let value = nodes.node(
            NodeType::new("expr_lit", Some("u32")),
            span(source, literal),
            vec![],
        );
        let write = nodes.owned_assignment(assignment, &pattern.hir);
        facts.body.children[0].children[arm].children = vec![pattern, value, write];
        facts.bindings.push(binding);
    }
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.sites.len(), 3);
    assert_ne!(result.sites[0].root.native, result.sites[1].root.native);
    assert_ne!(
        result.sites[0].root.scope_span,
        result.sites[1].root.scope_span
    );
    assert_eq!(result.sites[2].root.native, facts.bindings[0].hir);
    assert!(result.sites[2].caller_visible);
}

#[test]
fn unknown_arm_rhs_retains_its_checked_original_assignment_lower_bound() {
    let source = "pub fn f(target:&mut u32){if true {*target=query();} else {*target=2;}}";
    let result = assess(&branch_fixture(source)).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert_eq!(result.sites.len(), 2);
    assert_eq!(result.sites[0].root, result.sites[1].root);
    assert!(
        result
            .boundaries
            .iter()
            .any(|boundary| boundary.availability == BoundaryAvailability::Unknown)
    );
}

#[test]
fn condition_nodes_retain_original_enclosing_parentheses_without_new_lookup() {
    for source in [
        "pub fn f(target:&mut u32,flag:bool){if ((flag)) {*target=1;} else {*target=2;}}",
        "pub fn f(target:&mut u32){if ((true) && !false) {*target=1;} else {*target=2;}}",
        "pub fn f(target:&mut u32){if (/*before*/ true /*after*/) {*target=1;}}",
    ] {
        let facts = branch_fixture(source);
        let condition = &facts.body.children[0].children[0];
        assert!(condition.snippet.starts_with('('));
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Complete, "{source}");
        let expected = if source.contains("else") { 2 } else { 1 };
        assert_eq!(result.sites.len(), expected, "{source}");
    }
}

#[test]
fn native_condition_span_cannot_hide_an_operator_as_parentheses() {
    let source = "pub fn f(target:&mut u32){if !false {*target=1;}}";
    let mut facts = branch_fixture(source);
    let condition = &mut facts.body.children[0].children[0];
    let enclosing_span = condition.span;
    condition.children[0].span = enclosing_span;
    condition.children[0].snippet = "!false".into();
    assert!(assess(&facts).is_err());
}

#[test]
fn compound_condition_writes_cannot_be_omitted_from_a_complete_inventory() {
    let source = "pub fn f(mut target:u32){if (target+=1)==(target+=2) {target=3;} else {target=4;} target=5;}";
    let facts = branch_fixture(source);
    let condition = &facts.body.children[0].children[0];
    assert!(
        condition
            .children
            .iter()
            .all(|node| node.kind == "expr_assign_op"
                && node.ty.as_ref().unwrap().normalized.as_deref() == Some("unit"))
    );
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert_eq!(result.sites.len(), 3);
    assert!(
        result
            .sites
            .iter()
            .all(|site| site.root.provenance == Provenance::FormalOwned)
    );
    assert!(result.boundaries.iter().any(|boundary| {
        boundary.availability == BoundaryAvailability::Unsupported
            && boundary
                .reason
                .contains("Compound assignment in conditional")
    }));
}
