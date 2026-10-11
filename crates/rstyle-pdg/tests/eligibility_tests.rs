//! Eligibility classifier tests over hand-built facts.
//!
//! No rustc runs here: these tests pin the closed R0 subset interpretation
//! (accepted shapes, every pinned rejection capability, default-deny,
//! evidence gaps) that the driver walk must satisfy. Real-HIR coverage
//! arrives with the driver integration tests.

use rstyle_pdg::EligibilityVerdict;
use rstyle_pdg::eligibility::{classify, span};
use rstyle_pdg::frontend::{
    BindingFact, BuildFacts, ByteSpan, Dispatch, HirRef, ParamFact, PathRes, R0FnFacts, R0Node,
    ResolvedTy, SignatureFacts, SourceFacts, known_bool, known_count, known_str,
};

fn hir(local: u32) -> HirRef {
    HirRef {
        krate: 0,
        owner: 1,
        local,
    }
}

fn scalar(normalized: &str) -> ResolvedTy {
    ResolvedTy {
        normalized: Some(normalized.to_string()),
        observed: normalized.to_string(),
        spelling: String::new(),
    }
}

fn untyped() -> Option<ResolvedTy> {
    None
}

fn leaf(kind: &str, snippet: &str) -> R0Node {
    R0Node {
        hir: hir(0),
        kind: kind.to_string(),
        span: span(0, 0),
        snippet: snippet.to_string(),
        ty: untyped(),
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

fn typed(mut node: R0Node, normalized: &str) -> R0Node {
    node.ty = Some(scalar(normalized));
    node
}

fn kids(mut node: R0Node, children: Vec<R0Node>) -> R0Node {
    node.children = children;
    node
}

fn local_read(target_local: u32, ty: &str) -> R0Node {
    let mut node = leaf("expr_path", "a");
    node.res = Some(PathRes {
        kind: "local".to_string(),
        target: Some(hir(target_local)),
        description: "Res::Local".to_string(),
    });
    typed(node, ty)
}

fn builtin_binary(op: &str, left_ty: &str, right_ty: &str, result: &str) -> Dispatch {
    Dispatch {
        kind: "builtin".to_string(),
        op: op.to_string(),
        operand_tys: vec![left_ty.to_string(), right_ty.to_string()],
        result_ty: result.to_string(),
    }
}

fn local_binding(symbol: &str, name: &str, local: u32, ty: &str) -> BindingFact {
    BindingFact {
        symbol: symbol.to_string(),
        name: name.to_string(),
        decl_span: span(0, 0),
        scope: "scope:test:fn:0:9".to_string(),
        hir: hir(local),
        is_param: false,
        is_mut: false,
        ty: scalar(ty),
    }
}

fn param_binding(symbol: &str, name: &str, local: u32, ty: &str) -> BindingFact {
    BindingFact {
        is_param: true,
        ..local_binding(symbol, name, local, ty)
    }
}

fn pat_binding(symbol: &str) -> R0Node {
    let mut node = leaf("pat_binding", "a");
    node.binding = Some(symbol.to_string());
    node.binding_mut = Some(false);
    node
}

fn param(symbol: &str, ty: &str, pat: R0Node) -> ParamFact {
    ParamFact {
        symbol: symbol.to_string(),
        position: 0,
        name_span: span(0, 0),
        ty_span: span(0, 0),
        ty: ResolvedTy {
            normalized: Some(ty.to_string()),
            observed: ty.to_string(),
            spelling: ty.to_string(),
        },
        pat,
    }
}

fn signature(params: Vec<ParamFact>, ret: &str) -> SignatureFacts {
    SignatureFacts {
        sig_span: span(0, 0),
        params,
        receiver: None,
        return_ty: scalar(ret),
        return_ty_span: span(0, 0),
        has_self: false,
        abi: known_str("Rust"),
        generics: Vec::new(),
        is_variadic: false,
        is_async: false,
        is_unsafe: false,
        is_const: false,
    }
}

fn build() -> BuildFacts {
    BuildFacts {
        rustc_version: known_str("test"),
        rustc_commit: known_str("test"),
        edition: known_str("2021"),
        target_triple: known_str("test"),
        pointer_width: known_count(64),
        data_layout: known_str("test"),
        cfgs: Vec::new(),
        features: Vec::new(),
        overflow_checks: known_bool(true),
        debug_assertions: known_bool(true),
        panic_strategy: known_str("unwind"),
        opt_level: known_str("0"),
    }
}

fn make_facts(
    body: R0Node,
    bindings: Vec<BindingFact>,
    params: Vec<ParamFact>,
    ret: &str,
) -> R0FnFacts {
    R0FnFacts {
        name: "f".to_string(),
        fn_span: span(0, 9),
        source: SourceFacts {
            bytes: "fn f() {}".to_string(),
            digest: "sha256:test".to_string(),
        },
        signature: signature(params, ret),
        bindings,
        body,
        build: build(),
    }
}

fn capabilities(found: &rstyle_pdg::frontend::R0Eligibility) -> Vec<&str> {
    found
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.capability.as_str())
        .collect()
}

#[test]
fn empty_unit_body_is_eligible() {
    let body = typed(leaf("expr_block", "{}"), "unit");
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "unit"));

    assert_eq!(found.status, EligibilityVerdict::Eligible);
    assert!(found.diagnostics.is_empty());
}

#[test]
fn straightline_binding_graph_is_eligible() {
    let read = local_read(7, "i32");
    let lit = typed(leaf("expr_lit", "1"), "i32");
    let mut add = typed(leaf("expr_binary", "a + 1"), "i32");
    add.dispatch = Some(builtin_binary("+", "i32", "i32", "i32"));
    add.children = vec![read, lit];
    let decl = kids(
        leaf("stmt_let", "let x = a + 1;"),
        vec![pat_binding("s1"), add],
    );
    let body = kids(typed(leaf("expr_block", "{ .. }"), "unit"), vec![decl]);
    let bindings = vec![
        param_binding("s0", "a", 7, "i32"),
        local_binding("s1", "x", 9, "i32"),
    ];
    let found = classify(&make_facts(
        body,
        bindings,
        vec![param("s0", "i32", pat_binding("s0"))],
        "i32",
    ));

    assert_eq!(found.status, EligibilityVerdict::Eligible);
}

#[test]
fn direct_call_is_rejected() {
    let body = kids(
        typed(leaf("expr_block", "{ g(a) }"), "i32"),
        vec![leaf("expr_call", "g(a)")],
    );
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.call-summaries"]);
}

#[test]
fn unit_comparison_is_rejected_as_scalar_computation() {
    let mut cmp = typed(leaf("expr_binary", "() == ()"), "bool");
    cmp.dispatch = Some(builtin_binary("==", "unit", "unit", "bool"));
    let body = kids(typed(leaf("expr_block", "{ .. }"), "bool"), vec![cmp]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "bool"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.scalar-computation"]);
}

#[test]
fn overloaded_operator_is_rejected() {
    let mut add = typed(leaf("expr_binary", "a + b"), "i32");
    add.dispatch = Some(Dispatch {
        kind: "overloaded".to_string(),
        op: "+".to_string(),
        operand_tys: vec!["i32".to_string(), "i32".to_string()],
        result_ty: "i32".to_string(),
    });
    let body = kids(typed(leaf("expr_block", "{ .. }"), "i32"), vec![add]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.scalar-computation"]);
}

#[test]
fn macro_expansion_reports_once_at_the_callsite() {
    let expanded_call = R0Node {
        expansion: Some(rstyle_pdg::frontend::ExpansionSite {
            callsite: span(11, 19),
            snippet: "panic!()".to_string(),
        }),
        children: vec![leaf("expr_call", "panic_fmt()")],
        ..leaf("expr_call", "panic_fmt(..)")
    };
    let mut and = typed(leaf("expr_binary", "false && panic!()"), "bool");
    and.dispatch = Some(builtin_binary("&&", "bool", "bool", "bool"));
    and.children = vec![typed(leaf("expr_lit", "false"), "bool"), expanded_call];
    let body = kids(typed(leaf("expr_block", "{ .. }"), "bool"), vec![and]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "bool"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.macros"]);
    assert_eq!(found.diagnostics[0].span, span(11, 19));
    assert_eq!(found.diagnostics[0].snippet, "panic!()");
}

#[test]
fn nested_item_is_rejected_without_body_scan() {
    let nested = kids(
        leaf("stmt_item", "fn nested() { g(); }"),
        vec![leaf("expr_call", "g()")],
    );
    let body = kids(typed(leaf("expr_block", "{ .. }"), "i32"), vec![nested]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.blocks"]);
}

#[test]
fn closure_rejects_at_its_site_and_skips_its_body() {
    let closure = kids(
        leaf("expr_closure", "|| g()"),
        vec![leaf("expr_call", "g()")],
    );
    let body = kids(typed(leaf("expr_block", "{ .. }"), "i32"), vec![closure]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.closures"]);
}

#[test]
fn wildcard_parameter_is_rejected() {
    let wild = leaf("pat_wild", "_");
    let body = typed(leaf("expr_block", "{}"), "unit");
    let found = classify(&make_facts(
        body,
        Vec::new(),
        vec![param("s0", "i32", wild)],
        "unit",
    ));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.scalar-interface"]);
}

#[test]
fn reference_interface_and_deref_accumulate() {
    let mut deref = typed(leaf("expr_unary", "*a"), "i32");
    deref.dispatch = Some(Dispatch {
        kind: "builtin".to_string(),
        op: "*".to_string(),
        operand_tys: vec!["&i32".to_string()],
        result_ty: "i32".to_string(),
    });
    let body = kids(typed(leaf("expr_block", "{ .. }"), "i32"), vec![deref]);
    let param_binding = BindingFact {
        ty: ResolvedTy {
            normalized: None,
            observed: "&i32".to_string(),
            spelling: "&i32".to_string(),
        },
        ..param_binding("s0", "a", 7, "&i32")
    };
    let found = classify(&make_facts(
        body,
        vec![param_binding],
        vec![param("s0", "&i32", pat_binding("s0"))],
        "i32",
    ));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(
        capabilities(&found),
        vec!["rust.additional-types", "rust.abstract-memory"]
    );
}

#[test]
fn default_deny_rejects_unknown_constructs() {
    let body = kids(
        typed(leaf("expr_block", "{ .. }"), "i32"),
        vec![leaf("expr_other", "gen { }")],
    );
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.additional-types"]);
}

#[test]
fn missing_path_resolution_is_unknown() {
    let body = kids(
        typed(leaf("expr_block", "{ .. }"), "i32"),
        vec![leaf("expr_path", "a")],
    );
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Unknown);
}

#[test]
fn deferred_constructs_map_to_their_capabilities() {
    let cases = [
        ("expr_loop", "rust.loops"),
        ("expr_match", "rust.match"),
        ("expr_cast", "rust.additional-types"),
        ("expr_addr_of", "rust.abstract-memory"),
        ("expr_yield", "rust.async"),
        ("expr_const_block", "rust.additional-types"),
    ];

    for (kind, capability) in cases {
        let body = kids(
            typed(leaf("expr_block", "{ .. }"), "i32"),
            vec![leaf(kind, kind)],
        );
        let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

        assert_eq!(found.status, EligibilityVerdict::Ineligible, "kind {kind}");
        assert_eq!(capabilities(&found), vec![capability], "kind {kind}");
    }
}

#[test]
fn uninitialized_and_let_else_declarations_are_rejected() {
    let bare = kids(leaf("stmt_let", "let x;"), vec![pat_binding("s0")]);
    let body = kids(typed(leaf("expr_block", "{ .. }"), "unit"), vec![bare]);
    let found = classify(&make_facts(
        body,
        vec![local_binding("s0", "x", 9, "i32")],
        Vec::new(),
        "unit",
    ));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.local-bindings"]);
}

fn with_signature(body: R0Node, signature: SignatureFacts) -> R0FnFacts {
    R0FnFacts {
        signature,
        ..make_facts(body, Vec::new(), Vec::new(), "unit")
    }
}

fn async_sig() -> SignatureFacts {
    SignatureFacts {
        is_async: true,
        ..signature(Vec::new(), "unit")
    }
}

fn unsafe_sig() -> SignatureFacts {
    SignatureFacts {
        is_unsafe: true,
        ..signature(Vec::new(), "unit")
    }
}

fn const_sig() -> SignatureFacts {
    SignatureFacts {
        is_const: true,
        ..signature(Vec::new(), "unit")
    }
}

fn variadic_sig() -> SignatureFacts {
    SignatureFacts {
        is_variadic: true,
        ..signature(Vec::new(), "unit")
    }
}

fn abi_sig() -> SignatureFacts {
    SignatureFacts {
        abi: known_str("C"),
        ..signature(Vec::new(), "unit")
    }
}

fn generics_sig() -> SignatureFacts {
    SignatureFacts {
        generics: vec![rstyle_pdg::frontend::GenericFact {
            name: "T".to_string(),
            span: span(0, 0),
        }],
        ..signature(Vec::new(), "unit")
    }
}

fn receiver_sig() -> SignatureFacts {
    SignatureFacts {
        receiver: Some(rstyle_pdg::frontend::ReceiverFact {
            span: span(1, 2),
            snippet: "self".to_string(),
        }),
        has_self: true,
        ..signature(Vec::new(), "unit")
    }
}

#[test]
fn signature_shapes_are_rejected() {
    let body = typed(leaf("expr_block", "{}"), "unit");
    let shapes = [
        ("receiver", receiver_sig(), "rust.scalar-interface"),
        ("async", async_sig(), "rust.async"),
        ("unsafe", unsafe_sig(), "rust.unsafe"),
        ("const", const_sig(), "rust.additional-types"),
        ("variadic", variadic_sig(), "rust.additional-types"),
        ("abi", abi_sig(), "rust.additional-types"),
        ("generics", generics_sig(), "rust.additional-types"),
    ];

    for (shape, sig, capability) in shapes {
        let found = classify(&with_signature(body.clone(), sig));

        assert_eq!(
            found.status,
            EligibilityVerdict::Ineligible,
            "shape {shape}"
        );
        assert_eq!(capabilities(&found), vec![capability], "shape {shape}");
    }
}

#[test]
fn if_let_conditions_are_rejected() {
    let cond = leaf("expr_let", "let Some(x) = o");
    let branch = kids(
        typed(leaf("expr_if", "if .."), "i32"),
        vec![cond, typed(leaf("expr_lit", "1"), "i32")],
    );
    let body = kids(typed(leaf("expr_block", "{ .. }"), "i32"), vec![branch]);
    let found = classify(&make_facts(body, Vec::new(), Vec::new(), "i32"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.branching"]);
}

#[test]
fn unsafe_blocks_are_rejected() {
    let mut block = typed(leaf("expr_block", "unsafe { }"), "unit");
    block.is_unsafe_block = true;
    let found = classify(&make_facts(block, Vec::new(), Vec::new(), "unit"));

    assert_eq!(found.status, EligibilityVerdict::Ineligible);
    assert_eq!(capabilities(&found), vec!["rust.unsafe"]);
}

#[test]
fn span_helper_builds_half_open_spans() {
    let made: ByteSpan = span(3, 9);

    assert_eq!((made.start, made.end), (3, 9));
}
