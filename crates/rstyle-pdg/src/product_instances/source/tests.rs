//! Source-selected original constructor and independent adversarial role edits.
use super::*;
use serde_json::json;

pub(super) fn fixture() -> (HirFnFacts, Value) {
    let (canonical, conversion, _) = crate::product_instances::test_support::conversion();
    let mut instance: Value = serde_json::from_str(include_str!("observation.json")).unwrap();
    instance["conversions"] = conversion["conversions"].clone();
    (canonical, instance)
}

// The reduced capture retains exact original body/observations, not a complete
// nominal census. Full source/owner closure is exercised by the natural probe.
fn body_proof<'a>(facts: &'a HirFnFacts, instance: &Value) -> Result<CheckedSource<'a>> {
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    let checked = body::verify(facts, &syntax)?;
    roster::verify(&checked, instance)?;
    Ok(checked)
}

#[test]
fn complete_original_source_retains_saved_conversion_assertion_and_result() {
    let (facts, instance) = fixture();

    let source = body_proof(&facts, &instance).unwrap();
    assert_eq!(source.pattern().snippet, "pattern");
    assert_eq!(
        source.receiver().res.as_ref().unwrap().target.as_ref(),
        Some(&source.input().hir)
    );
    assert_eq!(source.local().snippet, "span");
    assert_eq!(source.condition().snippet, "span.start <= span.end");
    assert_eq!(source.result_operands()[0].snippet, "pattern");
    assert_eq!(source.result_operands()[1].snippet, "span");
}

const HIR_ATTACKS: &[fn(&mut HirFnFacts)] = &[
    |f| {
        f.body.children.pop();
    },
    |f| {
        f.body.children.swap(0, 1);
    },
    |f| {
        f.body.children.push(f.body.children[2].clone());
    },
    |f| {
        f.body.children[0].children[2]
            .children
            .push(f.signature.params[1].pat.clone());
    },
    |f| {
        f.body.children[0].children[2].children[0]
            .res
            .as_mut()
            .unwrap()
            .target = Some(f.signature.params[0].pat.hir.clone());
    },
    |f| {
        f.body.children[1].children[0].expansion = None;
    },
    |f| {
        f.body.children[1].children[0]
            .res
            .as_mut()
            .unwrap()
            .description = "core::debug_assert".into();
    },
    |f| {
        f.body.children[1].children[0].lit = Some("false".into());
    },
    |f| {
        f.body.children[1].children[0].children[0]
            .dispatch
            .as_mut()
            .unwrap()
            .op = ">=".into();
    },
    |f| {
        f.body.children[1].children[0].children[0]
            .children
            .swap(0, 1);
    },
    |f| {
        f.body.children[2].children.swap(0, 1);
    },
    |f| {
        f.body.children[0].hir = f.body.children[1].hir.clone();
    },
    |f| {
        f.bindings.pop();
    },
    |f| {
        f.bindings[2].is_mut = true;
    },
];

#[test]
fn altered_complete_original_hir_roles_cannot_pass_source_proof() {
    let (facts, instance) = fixture();

    for (index, attack) in HIR_ATTACKS.iter().enumerate() {
        let mut changed = facts.clone();
        attack(&mut changed);
        assert!(
            body_proof(&changed, &instance).is_err(),
            "HIR attack {index}"
        );
    }
}

#[test]
fn incomplete_or_rebound_owner_and_node_observations_cannot_pass() {
    let (facts, instance) = fixture();

    for pointer in [
        "/original_signature/inputs/1/index",
        "/nodes/0/hir/local",
        "/nodes/0/span/end",
        "/nodes/0/original_kind",
        "/nodes/0/kind",
        "/nodes/6/original/index",
        "/conversions/0/receiver/local",
        "/owners/1/clauses/1/original_arguments/1/definition/definition_identity",
    ] {
        let mut changed = instance.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("forged");
        assert!(body_proof(&facts, &changed).is_err(), "{pointer}");
    }
    for pointer in ["/nodes", "/projected_inputs"] {
        let mut changed = instance.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(body_proof(&facts, &changed).is_err(), "{pointer}");
    }
    let mut changed = instance.clone();
    changed["nodes"][1] = changed["nodes"][0].clone();
    assert!(body_proof(&facts, &changed).is_err());
}

#[test]
fn changed_source_assertion_or_result_cannot_reuse_original_body_evidence() {
    let (facts, instance) = fixture();

    for (before, after) in [
        ("span.start <= span.end", "span.start >= span.end"),
        ("invalid match span", "another panic text"),
        ("Match { pattern, span }", "Match { span, pattern }"),
    ] {
        let mut changed = facts.clone();
        changed.source.bytes = facts.source.bytes.replace(before, after).into();
        assert!(body_proof(&changed, &instance).is_err(), "{before}");
    }
}

#[test]
fn typed_sites_reject_unknown_identity_and_span_members() {
    let (facts, instance) = fixture();

    for pointer in [
        "/conversions/0/hir",
        "/conversions/0/span",
        "/conversions/0/receiver",
        "/nodes/0/hir",
        "/nodes/0/span",
    ] {
        let mut changed = instance.clone();
        changed
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unproved".into(), json!(0));
        assert!(body_proof(&facts, &changed).is_err(), "{pointer}");
    }
}
