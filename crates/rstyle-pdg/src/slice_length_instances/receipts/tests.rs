//! Independent inventory/unavailable-boundary expectations; no missing call authority.
use super::*;
use serde_json::json;

const SOURCE: &str = "pub fn caller() { for _ in 0..1 { check(&[], 4, \"why\"); } }";
const LOOP: &str = "for _ in 0..1 { check(&[], 4, \"why\"); }";
const CALL: &str = "check(&[], 4, \"why\")";

fn definition() -> Definition {
    Definition {
        path: "check".into(),
        definition_identity: "callee-definition".into(),
        crate_name: "fixture".into(),
        stable_crate_id: "fixture-id".into(),
        local: true,
    }
}

fn span(text: &str) -> crate::ByteSpan {
    let start = SOURCE.find(text).unwrap() as u32;
    crate::ByteSpan {
        start,
        end: start + text.len() as u32,
    }
}

fn sample() -> (HirFnFacts, Value) {
    let mut caller: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    caller.source.bytes = SOURCE.into();
    caller.source.digest = crate::source_identity::checksum(SOURCE.as_bytes());
    caller.source_scan_complete = false;
    caller
        .build
        .frontend_options
        .insert("definition_identity".into(), json!("caller-definition"));
    let mut opaque = caller.body.clone();
    opaque.hir.local = 1001;
    opaque.kind = "expr_match".into();
    opaque.span = span(LOOP);
    opaque.snippet = LOOP.into();
    opaque.children.clear();
    opaque.res = None;
    opaque.dispatch = None;
    opaque.is_unsafe_block = false;
    opaque.expansion = Some(crate::ExpansionSite {
        callsite: opaque.span,
        snippet: LOOP.into(),
    });
    caller.body.children = vec![opaque];
    caller.source_diagnostics = vec![crate::HirDiagnostic {
        capability: "rust.additional-types".into(), construct: "source_deferred".into(),
        span: span(LOOP), snippet: LOOP.into(), severity: "error".into(),
        reason: "Original owned source contains deferred rust.additional-types; cfg removal cannot certify a supported remainder.".into(),
    }];

    let observed = observed(&caller);
    caller.build.frontend_options.insert(
        "rstyle.rust.call-instances".into(),
        json!({
            "identity":"rstyle.rust.call-instance-observations/1", "complete":true,
            "calls":[observed],
        }),
    );
    (caller, observed)
}

fn observed(caller: &HirFnFacts) -> Value {
    let hir = HirRef {
        local: 1002,
        ..caller.body.hir.clone()
    };
    let arguments = json!([{
        "kind":"type", "value":"u8", "display_only":true, "identity_status":"known",
        "identity":{"kind":"builtin", "name":"u8"},
    }]);
    json!({
        "hir":hir, "span":span(CALL), "expanded":false, "status":"resolved",
        "requested":definition(), "selected":definition(),
        "requested_arguments":arguments, "selected_arguments":arguments,
        "signature":{"display_only":true},
    })
}

fn projection(caller: &HirFnFacts) -> Projection {
    Projection {
        definition: definition(),
        type_parameter: definition(),
        owner: caller.body.hir.clone(),
        source_digest: caller.source.digest.clone(),
        fn_span: caller.fn_span,
        original_source_scan_complete: false,
        arguments: vec![],
        parameters: vec![],
        result: Endpoint::Structural { arguments: vec![] },
        bindings: vec![],
        nodes: vec![],
        methods: vec![],
    }
}

fn receipt(caller: &HirFnFacts, call: Value) -> Receipt {
    Receipt {
        caller: "caller-definition".into(),
        caller_source_digest: caller.source.digest.clone(),
        caller_fn_span: caller.fn_span,
        call,
    }
}

#[test]
fn opaque_for_receipt_is_retained_but_never_requests_an_absent_call() {
    let (caller, observed) = sample();
    let projection = projection(&caller);
    let receipts = [receipt(&caller, observed.clone())];
    verify(
        &caller,
        &projection,
        &receipts,
        std::slice::from_ref(&caller),
    )
    .unwrap();
    assert_eq!(
        expected(&caller, &definition(), std::slice::from_ref(&caller))
            .unwrap()
            .len(),
        1
    );
    let mut invented = caller.body.children[0].clone();
    invented.hir = serde_json::from_value(observed["hir"].clone()).unwrap();
    invented.span = span(CALL);
    invented.kind = "expr_call".into();
    invented.expansion = None;
    invented.children = vec![invented.clone(); 3];
    let requests = Requests {
        definition: &projection.definition,
        receipts: &receipts,
    };
    assert!(requests.requested(&caller, &caller, &invented).is_err());
}

#[test]
fn unavailable_inventory_requires_explicit_original_boundary_and_diagnostic() {
    for change in [
        "delete",
        "diagnostic",
        "duplicate",
        "reason",
        "snippet",
        "expansion",
        "ordinary",
        "owner",
    ] {
        let (mut caller, observed) = sample();
        match change {
            "delete" => caller.body.children.clear(),
            "diagnostic" => caller.source_diagnostics.clear(),
            "duplicate" => caller
                .source_diagnostics
                .push(caller.source_diagnostics[0].clone()),
            "reason" => caller.source_diagnostics[0].reason.push('x'),
            "snippet" => caller.source_diagnostics[0].snippet.push('x'),
            "expansion" => caller.body.children[0].expansion = None,
            "ordinary" => caller.body.children[0].kind = "expr_call".into(),
            "owner" => caller.body.hir.owner += 1,
            _ => unreachable!(),
        }
        assert!(
            inventory_member(&caller, &observed, &definition()).is_err(),
            "{change}"
        );
    }
}

#[test]
fn unavailable_inventory_keeps_exact_receipts_and_substitutions() {
    for change in [
        "callee",
        "argument",
        "receipt",
        "duplicate",
        "removed",
        "inventory",
    ] {
        let (mut caller, observed) = sample();
        let projection = projection(&caller);
        let mut receipts = vec![receipt(&caller, observed)];
        match change {
            "callee" => receipts[0].call["selected"]["definition_identity"] = json!("different"),
            "argument" => {
                receipts[0].call["selected_arguments"][0]["identity"]["name"] = json!("u16")
            }
            "receipt" => receipts[0].caller_source_digest.push('x'),
            "duplicate" => receipts.push(receipt(&caller, receipts[0].call.clone())),
            "removed" => receipts.clear(),
            "inventory" => caller
                .build
                .frontend_options
                .get_mut("rstyle.rust.call-instances")
                .unwrap()["calls"]
                .as_array_mut()
                .unwrap()
                .clear(),
            _ => unreachable!(),
        }
        if matches!(change, "callee" | "argument") {
            caller
                .build
                .frontend_options
                .get_mut("rstyle.rust.call-instances")
                .unwrap()["calls"][0] = receipts[0].call.clone();
        }
        assert!(
            verify(
                &caller,
                &projection,
                &receipts,
                std::slice::from_ref(&caller)
            )
            .is_err(),
            "{change}"
        );
    }
}

#[test]
fn nonempty_receipt_prefix_cannot_omit_an_unavailable_caller() {
    let (caller, observed) = sample();
    let projection = projection(&caller);
    let receipts = [receipt(&caller, observed)];
    let mut other = caller.clone();
    other
        .build
        .frontend_options
        .insert("definition_identity".into(), json!("other-caller"));
    let callers = [caller.clone(), other];
    assert!(verify(&caller, &projection, &receipts, &callers).is_err());
}

#[test]
fn unavailable_boundary_is_independent_of_source_scan_completion() {
    let (mut caller, observed) = sample();
    for complete in [false, true] {
        caller.source_scan_complete = complete;
        inventory_member(&caller, &observed, &definition()).unwrap();
        caller.source_diagnostics.clear();
        assert!(inventory_member(&caller, &observed, &definition()).is_err());
        caller = sample().0;
    }
}

#[test]
fn incomplete_unrelated_inventory_cannot_prove_an_empty_call_roster() {
    let (caller, observed) = sample();
    let projection = projection(&caller);
    let receipts = [receipt(&caller, observed)];
    for bad in [
        Value::Null,
        json!({"identity":"rstyle.rust.call-instance-observations/1", "complete":true}),
    ] {
        let mut other = caller.clone();
        other
            .build
            .frontend_options
            .insert("definition_identity".into(), json!("unrelated"));
        other
            .build
            .frontend_options
            .insert("rstyle.rust.call-instances".into(), bad);
        assert!(verify(&caller, &projection, &receipts, &[caller.clone(), other]).is_err());
    }
    let mut missing = caller.clone();
    missing
        .build
        .frontend_options
        .insert("definition_identity".into(), json!("unrelated"));
    missing
        .build
        .frontend_options
        .remove("rstyle.rust.call-instances");
    assert!(verify(&caller, &projection, &receipts, &[caller.clone(), missing]).is_err());
}
