//! Retained compiler calls and actual post-scan findings with adversarial edits.
use super::*;
use serde_json::Value;

fn canonical() -> HirFnFacts {
    let operations: Value =
        serde_json::from_str(include_str!("../operations/fixtures.json")).unwrap();
    serde_json::from_value(operations[0]["canonical"].clone()).unwrap()
}

fn call(facts: &HirFnFacts) -> &HirNode {
    &facts.body.children[0].children[1]
}

#[test]
fn raw_generic_facts_and_observations_never_activate_an_instance() {
    let facts = canonical();
    assert!(!verify(&facts).unwrap());
    assert!(!source_findings(&facts).unwrap());
    assert!(operation(call(&facts), &facts).unwrap().is_none());
}

#[test]
fn retained_conversion_has_one_exact_operand_and_shared_formal_origin() {
    let facts = canonical();
    let node = call(&facts);
    let checked = checked_operation(node, node, Haystack::Bytes)
        .unwrap()
        .unwrap();
    assert_eq!(checked.receiver, &node.children[0]);
    assert_eq!(checked.input_origin.parameter, 0);
    assert_eq!(checked.input_origin.path, [Projection::SharedReferent]);
    assert!(
        checked_operation(&facts.body, node, Haystack::Bytes)
            .unwrap()
            .is_none()
    );
}

#[test]
fn same_site_edits_cannot_reuse_the_checked_operation() {
    let facts = canonical();
    let expected = call(&facts);
    for mode in [
        "snippet",
        "span",
        "resolution",
        "dispatch",
        "operand",
        "missing",
        "duplicate",
    ] {
        let mut changed = expected.clone();
        match mode {
            "snippet" => changed.snippet.push(' '),
            "span" => changed.span.start += 1,
            "resolution" => changed.res = None,
            "dispatch" => changed.dispatch = None,
            "operand" => changed.children[0].res = None,
            "missing" => changed.children.clear(),
            "duplicate" => changed.children.push(expected.children[0].clone()),
            _ => unreachable!(),
        }
        assert!(
            checked_operation(&changed, expected, Haystack::Bytes).is_err(),
            "{mode}"
        );
    }
}

#[test]
fn exact_retained_deferred_finding_is_covered_without_rewriting_it() {
    let scans: Value = serde_json::from_str(include_str!("scan-fixtures.json")).unwrap();
    for scan in scans.as_array().unwrap() {
        let mut facts = canonical();
        facts.source_scan_complete = scan["source_scan_complete"].as_bool().unwrap();
        facts.source_diagnostics =
            serde_json::from_value(scan["source_diagnostics"].clone()).unwrap();
        let before = facts.clone();
        findings(&facts, call(&facts)).unwrap();
        assert_eq!(facts, before);
        for mode in [
            "absent",
            "extra",
            "complete",
            "capability",
            "construct",
            "span",
            "snippet",
            "severity",
            "reason",
        ] {
            let mut changed = facts.clone();
            match mode {
                "absent" => changed.source_diagnostics.clear(),
                "extra" => changed
                    .source_diagnostics
                    .push(facts.source_diagnostics[0].clone()),
                "complete" => changed.source_scan_complete = true,
                "capability" => changed.source_diagnostics[0].capability = "rust.macros".into(),
                "construct" => changed.source_diagnostics[0].construct = "expr_call".into(),
                "span" => changed.source_diagnostics[0].span.end += 1,
                "snippet" => changed.source_diagnostics[0].snippet = "other.as_ref()".into(),
                "severity" => changed.source_diagnostics[0].severity = "warning".into(),
                "reason" => changed.source_diagnostics[0].reason.clear(),
                _ => unreachable!(),
            }
            assert!(findings(&changed, call(&facts)).is_err(), "{mode}");
        }
    }
}

#[test]
fn string_view_preserves_one_root_and_adds_exactly_one_byte_projection() {
    let facts = canonical();
    let node = call(&facts);
    let checked = checked_operation(node, node, Haystack::Str)
        .unwrap()
        .unwrap();
    assert_eq!(
        checked.input_origin,
        Origin {
            parameter: 0,
            path: vec![Projection::SharedReferent]
        }
    );
    let expected = Origin {
        parameter: 0,
        path: vec![Projection::SharedReferent, Projection::Utf8Bytes],
    };
    assert_eq!(
        checked.project(vec![checked.input_origin.clone()]).unwrap(),
        std::slice::from_ref(&expected)
    );
    for roots in [
        vec![],
        vec![expected],
        vec![Origin {
            parameter: 1,
            path: vec![Projection::SharedReferent],
        }],
        vec![checked.input_origin.clone(), checked.input_origin.clone()],
    ] {
        assert!(checked.project(roots).is_err());
    }
}

#[test]
fn byte_view_types_and_summary_do_not_conflate_strings_with_slices() {
    let facts = canonical();
    let node = call(&facts);
    let string = checked_operation(node, node, Haystack::Str)
        .unwrap()
        .unwrap();
    let bytes = checked_operation(node, node, Haystack::Bytes)
        .unwrap()
        .unwrap();
    let str_ty = r#"{"kind":"shared","value":{"kind":"str"}}"#;
    let byte_ty =
        r#"{"kind":"shared","value":{"kind":"slice","value":{"kind":"scalar","value":"u8"}}}"#;
    assert_eq!(
        string.summary_id(),
        "rstyle.reference-instance.str-as-bytes"
    );
    assert_eq!(bytes.summary_id(), "rstyle.reference-instance.byte-as-ref");
    string.verify_types(str_ty, byte_ty).unwrap();
    bytes.verify_types(byte_ty, byte_ty).unwrap();
    for (input, output) in [(byte_ty, byte_ty), (str_ty, str_ty), (str_ty, "usize")] {
        assert!(string.verify_types(input, output).is_err());
    }
    assert!(bytes.verify_types(str_ty, byte_ty).is_err());
}

#[test]
fn utf8_origin_typing_rejects_wrong_roots_and_duplicate_views() {
    use crate::data_types::TypeToken;
    let mut facts = canonical();
    facts.signature.params[0].ty.normalized =
        Some(r#"{"kind":"shared","value":{"kind":"str"}}"#.into());
    let path = [Projection::SharedReferent, Projection::Utf8Bytes];
    assert_eq!(
        crate::helper_reads::type_at(&facts, 0, &path).unwrap(),
        TypeToken::Slice(Box::new(TypeToken::Scalar("u8".into())))
    );
    assert!(crate::helper_reads::type_at(&facts, 0, &[Projection::Utf8Bytes]).is_err());
    assert!(
        crate::helper_reads::type_at(
            &facts,
            0,
            &[
                Projection::SharedReferent,
                Projection::Utf8Bytes,
                Projection::Utf8Bytes
            ]
        )
        .is_err()
    );
    assert!(crate::helper_reads::type_at(&facts, 1, &path).is_err());
    let origin = Origin {
        parameter: 0,
        path: path.to_vec(),
    };
    assert!(verify_returned_origin(&origin, &facts, &shared_bytes()).is_err());
    facts.signature.params[0].ty.normalized = Some(shared_bytes().text().unwrap());
    assert!(crate::helper_reads::type_at(&facts, 0, &path).is_err());
}
