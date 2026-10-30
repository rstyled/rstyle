use super::*;
use crate::{HirFnFacts, HirNode, PathRes};

fn owned(mut facts: HirFnFacts) -> HirFnFacts {
    let controls = facts
        .build
        .frontend_options
        .get(super::super::control_types::KEY)
        .cloned();
    let source = facts.source.bytes.to_string();
    super::call_tests::ownership(&mut facts, &source);
    if let Some(controls) = controls {
        facts
            .build
            .frontend_options
            .insert(super::super::control_types::KEY.into(), controls);
    }
    facts.signature.return_ty.normalized = Some("unit".into());
    facts.build.frontend_options.insert(
        super::super::formal_types::KEY.into(),
        serde_json::to_value(super::super::formal_types::Observations {
            identity: super::super::formal_types::IDENTITY.into(),
            mutable_u32: facts
                .signature
                .params
                .iter()
                .filter(|parameter| {
                    parameter
                        .pat
                        .ty
                        .as_ref()
                        .is_some_and(|ty| ty.normalized.is_none())
                })
                .map(|parameter| super::super::formal_types::Formal {
                    parameter: parameter.position,
                    binding: parameter.pat.hir.clone(),
                })
                .collect(),
        })
        .unwrap(),
    );
    facts
}

fn branch(source: &str) -> HirFnFacts {
    owned(super::tests::conditional::branch_fixture(source))
}

fn primitive_loop(source: &str) -> HirFnFacts {
    owned(super::tests::loops::loop_fixture(source))
}

fn uses_unrequested(observed: &Observations) {
    assert!(
        matches!(&observed.uses.availability, Availability::Unknown { reason }
        if reason == "Formal use inventory was not requested")
    );
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    assert!(observed.uses.formals.is_empty());
    assert!(observed.uses.occurrences.is_empty());
}

#[test]
fn effects_observe_checked_branch_and_primitive_loop_static_sites() {
    let cases = [
        branch("pub fn f(target:&mut u32){if true {*target=1;} else {*target=2;}}"),
        primitive_loop("pub fn f(target:&mut u32){for _ in 0u32..100u32 {*target=1;}}"),
        primitive_loop("pub fn f(target:&mut u32){loop {*target=1;break;*target=2;}}"),
        primitive_loop("pub fn f(target:&mut u32){while false {*target=1;}}"),
    ];
    for facts in cases {
        let observed = inspect_effect_calls(&facts).unwrap();
        assert_eq!(observed.direct.coverage, Coverage::Complete);
        assert_eq!(observed.direct.execution, Execution::Completed);
        let expected = facts.source.bytes.matches("*target=").count();
        assert_eq!(observed.direct.sites.len(), expected);
        assert!(
            observed
                .direct
                .sites
                .iter()
                .all(|site| site.caller_visible && site.root.native == facts.bindings[0].hir)
        );
        assert!(observed.calls.is_empty());
        uses_unrequested(&observed);
        assert_eq!(
            serde_json::to_value(&observed.direct).unwrap(),
            serde_json::to_value(inspect(&facts).unwrap()).unwrap()
        );
        for conservative in [
            inspect_calls(&facts).unwrap(),
            inspect_calls_with_uses(&facts).unwrap(),
        ] {
            assert_eq!(conservative.direct.coverage, Coverage::Incomplete);
            assert!(conservative.direct.sites.is_empty());
        }
    }
}

#[test]
fn effects_require_original_native_control_correspondence() {
    let source = "pub fn f(target:&mut u32){if true {*target=1;} else {*target=2;}}";
    for attack in 0..3 {
        let mut facts = branch(source);
        match attack {
            0 => facts.body.children[0].children.swap(1, 2),
            1 => {
                facts.body.children[0].children.pop();
            }
            _ => {
                facts.body.children[0].children[0]
                    .ty
                    .as_mut()
                    .unwrap()
                    .normalized = Some("u32".into())
            }
        }
        assert!(
            inspect_effect_calls(&facts).is_err(),
            "branch attack {attack}"
        );
    }
    let source = "pub fn f(target:&mut u32){for _ in 0u32..2u32 {*target=1;}}";
    let mut facts = primitive_loop(source);
    facts.body.children[0].children.swap(0, 1);
    assert!(inspect_effect_calls(&facts).is_err());
    let mut facts = primitive_loop(source);
    facts.body.children[0].children[0].res = None;
    let observed = inspect_effect_calls(&facts).unwrap();
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
    assert!(observed.direct.sites.is_empty());
    let mut facts = branch("pub fn f(target:&mut u32){if true {*target=1;}}");
    facts.body.children[0].children[0].ty = None;
    let observed = inspect_effect_calls(&facts).unwrap();
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
    assert!(observed.direct.sites.is_empty());
}

#[test]
fn effects_do_not_restore_roots_poisoned_by_reference_redirection() {
    for prefix in ["target=other;", "opaque!();"] {
        let source = format!(
            "pub fn f(mut target:&mut u32){{if true {{{prefix} *target=1;}} else {{*target=2;}} *target=3;}}"
        );
        let observed = inspect_effect_calls(&branch(&source)).unwrap();
        assert_eq!(observed.direct.coverage, Coverage::Incomplete, "{prefix}");
        assert!(observed.direct.sites.is_empty(), "{prefix}");
        uses_unrequested(&observed);
    }
}

fn original_call(facts: &HirFnFacts, text: &str, local: u32) -> HirNode {
    let mut call = HirNode {
        hir: crate::HirRef {
            krate: 0,
            owner: 1,
            local,
        },
        kind: "expr_call".into(),
        span: super::tests::span(&facts.source.bytes, text),
        snippet: text.into(),
        ty: Some(crate::ResolvedTy {
            normalized: Some("unit".into()),
            observed: "()".into(),
            spelling: String::new(),
        }),
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: vec![],
    };
    call.dispatch = Some(crate::Dispatch {
        kind: "resolved_direct".into(),
        op: "sample::leaf".into(),
        operand_tys: vec![],
        result_ty: "unit".into(),
    });
    call.res = Some(PathRes {
        kind: "resolved_direct_callee".into(), target: None,
        description: serde_json::json!({"local":true,"path":"sample::leaf","crate":"sample","definition_identity":"leaf-id"}).to_string(),
    });
    call
}

#[test]
fn effects_keep_nested_call_boundaries_and_checked_local_write_scopes() {
    let source = "pub fn f(mut target:u32){if true {target=1;leaf();} else {target=2;} target=3;}";
    let mut facts = branch(source);
    let call = original_call(&facts, "leaf()", 2000);
    facts.body.children[0].children[1].children[1] = call;
    let observed = inspect_effect_calls(&facts).unwrap();
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
    assert_eq!(observed.direct.sites.len(), 3);
    assert!(
        observed
            .direct
            .sites
            .iter()
            .all(|site| !site.caller_visible && site.root.provenance == Provenance::FormalOwned)
    );
    assert_eq!(observed.calls.len(), 1);
    let call = &observed.calls[0];
    assert_eq!(call.target, Availability::Known("leaf-id".into()));
    assert_eq!(call.target_path.as_deref(), Some("sample::leaf"));
    assert_eq!(
        observed.direct.boundaries[call.boundary_index].span,
        call.span
    );
    uses_unrequested(&observed);
    let mut changed = facts;
    changed.body.children[0].children[1].children[1]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys
        .push("u32".into());
    assert!(inspect_effect_calls(&changed).is_err());
}
