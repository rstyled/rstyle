//! Frontend-fact round-trip tests: the IR survives JSON unchanged.
//!
//! These tests use hand-built facts only. They pin the fact wire format that
//! the driver walk must emit and the classifier must accept, without running
//! rustc or any producer analysis.

use rstyle_pdg::frontend::{
    BindingFact, BuildFacts, ByteSpan, HirFnFacts, HirNode, HirRef, ParamFact, ResolvedTy,
    SignatureFacts, SourceFacts, known_bool, known_count, known_str,
};

fn span(start: u32, end: u32) -> ByteSpan {
    ByteSpan { start, end }
}

fn scalar(normalized: &str, spelling: &str) -> ResolvedTy {
    ResolvedTy {
        normalized: Some(normalized.to_string()),
        observed: normalized.to_string(),
        spelling: spelling.to_string(),
    }
}

fn leaf(kind: &str, start: u32, end: u32, snippet: &str) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local: start,
        },
        kind: kind.to_string(),
        span: span(start, end),
        snippet: snippet.to_string(),
        ty: None,
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

fn read_node() -> HirNode {
    HirNode {
        res: Some(rstyle_pdg::frontend::PathRes {
            kind: "local".to_string(),
            target: Some(HirRef {
                krate: 0,
                owner: 1,
                local: 7,
            }),
            description: "Res::Local".to_string(),
        }),
        ty: Some(scalar("i32", "")),
        ..leaf("path", 26, 27, "a")
    }
}

fn lit_node() -> HirNode {
    HirNode {
        lit: Some("1".to_string()),
        ty: Some(scalar("i32", "")),
        ..leaf("lit", 30, 31, "1")
    }
}

fn add_node() -> HirNode {
    HirNode {
        dispatch: Some(rstyle_pdg::frontend::Dispatch {
            kind: "builtin".to_string(),
            op: "+".to_string(),
            operand_tys: vec!["i32".to_string(), "i32".to_string()],
            result_ty: "i32".to_string(),
        }),
        ty: Some(scalar("i32", "")),
        children: vec![read_node(), lit_node()],
        ..leaf("binary", 26, 31, "a + 1")
    }
}

fn pat_binding() -> HirNode {
    HirNode {
        binding: Some("s0".to_string()),
        binding_mut: Some(false),
        ..leaf("pat_binding", 5, 6, "a")
    }
}

fn signature() -> SignatureFacts {
    SignatureFacts {
        sig_span: span(0, 20),
        params: vec![ParamFact {
            symbol: "s0".to_string(),
            position: 0,
            name_span: span(5, 6),
            ty_span: span(8, 11),
            ty: scalar("i32", "i32"),
            pat: pat_binding(),
        }],
        receiver: None,
        return_ty: scalar("i32", "i32"),
        return_ty_span: span(15, 18),
        has_self: false,
        abi: known_str("Rust"),
        generics: Vec::new(),
        is_variadic: false,
        is_async: false,
        is_unsafe: false,
        is_const: false,
    }
}

fn bindings() -> Vec<BindingFact> {
    vec![BindingFact {
        symbol: "s0".to_string(),
        name: "a".to_string(),
        decl_span: span(5, 6),
        scope: "scope:test:fn:0:36".to_string(),
        hir: HirRef {
            krate: 0,
            owner: 1,
            local: 7,
        },
        is_param: true,
        is_mut: false,
        ty: scalar("i32", "i32"),
    }]
}

fn build() -> BuildFacts {
    BuildFacts {
        producer_revision: known_str("synthetic-test-only"),
        frontend_options: Default::default(),
        rustc_version: known_str("1.101.0-nightly"),
        rustc_commit: known_str("5c543b0b8"),
        edition: known_str("2021"),
        target_triple: known_str("aarch64-unknown-linux-gnu"),
        pointer_width: known_count(64),
        data_layout: known_str("e-m:e-p:64:64-i64:64-i128:128-n32:64-S128"),
        cfgs: Vec::new(),
        features: Vec::new(),
        overflow_checks: known_bool(true),
        debug_assertions: known_bool(true),
        panic_strategy: known_str("unwind"),
        opt_level: known_str("0"),
    }
}

fn straightline_facts() -> HirFnFacts {
    let body = HirNode {
        children: vec![add_node()],
        ..leaf("block", 21, 35, "{ let x = a + 1; x }")
    };

    HirFnFacts {
        source_diagnostics: Vec::new(),
        source_scan_complete: true,
        name: "f".to_string(),
        fn_span: span(0, 36),
        source: SourceFacts {
            bytes: "fn f(a: i32) -> i32 { a + 1 }".into(),
            digest: "sha256:placeholder".to_string(),
        },
        signature: signature(),
        bindings: bindings(),
        body,
        build: build(),
    }
}

#[test]
fn facts_survive_json_round_trip() {
    let facts = straightline_facts();
    let json = serde_json::to_string(&facts).expect("facts serialize");
    let back: HirFnFacts = serde_json::from_str(&json).expect("facts parse");

    assert_eq!(facts, back);
}

#[test]
fn unknown_fields_are_rejected() {
    let json = serde_json::json!({"krate": 0, "owner": 1, "local": 2, "extra": true});
    let parsed: Result<HirRef, _> = serde_json::from_value(json);

    assert!(parsed.is_err());
}

#[test]
fn unknown_evidence_keeps_a_reason() {
    let facts = HirFnFacts {
        build: BuildFacts {
            rustc_commit: rstyle_pdg::frontend::unknown_str("rustc -vV unavailable"),
            ..straightline_facts().build
        },
        ..straightline_facts()
    };
    let json = serde_json::to_string(&facts).expect("facts serialize");
    let back: HirFnFacts = serde_json::from_str(&json).expect("facts parse");

    assert_eq!(facts, back);
    assert_eq!(back.build.rustc_commit.value, None);
    assert!(back.build.rustc_commit.reason.is_some());
}
