//! Syntax-only expectations; positive member authority requires original Cargo callsites.
use super::*;

fn syntax(source: &str, method: &str) -> Result<(), String> {
    let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Call(call) = expression else {
        return Err("Not original static-call syntax".into());
    };
    static_path(&call, method).map(|_| ())
}

#[test]
fn original_explicit_static_syntax_does_not_claim_selected_member_authority() {
    assert!(syntax("TryFromParsed::from(value)", "from").is_ok());
    assert!(syntax("error::ComponentRange::try_from(tagged)", "try_from").is_ok());
}

#[test]
fn generic_ufcs_computed_and_wrong_member_syntax_stays_outside_the_call_contract() {
    for source in [
        "from(value)",
        "TryFromParsed::from::<u8>(value)",
        "Wrapper::<u8>::from(value)",
        "<Wrapper as From<Input>>::from(value)",
        "::Wrapper::from(value)",
        "Wrapper::try_from(value)",
        "(if choose { first } else { second })(value)",
        "#[cfg(any())] Wrapper::from(value)",
    ] {
        assert!(syntax(source, "from").is_err(), "{source}");
    }
}

// Exact rows/nodes from the ordinary no_std Cargo development crate captured at
// /tmp/rstyle-421-tagged-calls. These test the borrowed adapter obligations only;
// they do not mint checked source receipts or certify a complete helper graph.
fn originals() -> Vec<Value> {
    serde_json::from_str(include_str!("original-calls.json")).unwrap()
}

fn node(value: &Value) -> HirNode {
    serde_json::from_value(value["node"].clone()).unwrap()
}

fn observation(value: &Value) -> Observation {
    serde_json::from_value(value["observation"].clone()).unwrap()
}

fn registry() -> crate::data_library::Registry {
    serde_json::from_str(include_str!("../../../contracts/read-only-lengths-v2.json")).unwrap()
}

fn owner(value: &Value) -> super::super::verify::Owner<'_> {
    super::super::verify::Owner {
        name: value["name"].as_str().unwrap(),
        identity: value["identity"].as_str().unwrap(),
        crate_name: value["compiled"]["crate_name"].as_str().unwrap(),
        stable_crate_id: value["compiled"]["stable_crate_id"].as_str().unwrap(),
    }
}

fn interface(value: &Value) -> (TypeTree, TypeTree) {
    let input = value["interface"]["parameters"][0]["ty"].clone();
    let output = value["interface"]["result"]["ty"].clone();
    (
        serde_json::from_value(input).unwrap(),
        serde_json::from_value(output).unwrap(),
    )
}

fn typed_call(value: &Value, changed: &HirNode, signature: &Value) -> Result<(), String> {
    let (input, output) = interface(value);
    Interface {
        input: &input,
        output: &output,
        direction: if value["name"] == "wrap" {
            Direction::Wrap
        } else {
            Direction::Unwrap
        },
    }
    .dispatch(changed, signature)
}

#[test]
fn actual_wrap_inverse_and_bool_projection_use_the_call_output() {
    let registry = registry();
    for value in originals() {
        let node = node(&value);
        let call = inventory_call(&node, &value["inventory"]).unwrap();
        let observed = observation(&value);
        selection(&call, &observed).unwrap();
        selected_context(
            &call.selected,
            &owner(&value["caller_owner"]),
            &owner(&value["callee_owner"]),
        )
        .unwrap();
        requested_resolution(&node, &call, &registry.core_hash).unwrap();
        typed_call(&value, &node, &call.signature).unwrap();
        let direction = if value["name"] == "wrap" {
            Direction::Wrap
        } else {
            Direction::Unwrap
        };
        authority::Pins {
            definitions: &registry.definitions,
        }
        .conversion(direction, &observed)
        .unwrap();
        if value["name"] == "projected" {
            assert_eq!(value["caller_result"]["normalized"], "bool");
            assert!(matches!(interface(&value).1, TypeTree::Result { .. }));
            let mut wrong = node.clone();
            wrong.ty.as_mut().unwrap().normalized = Some("bool".into());
            assert!(typed_call(&value, &wrong, &call.signature).is_err());
        }
    }
}

#[test]
fn inventory_requires_complete_unique_exact_original_rows() {
    for value in originals() {
        let original = node(&value);
        for pointer in [
            "/complete",
            "/identity",
            "/calls/0/status",
            "/calls/0/expanded",
        ] {
            let mut changed = value["inventory"].clone();
            let slot = changed.pointer_mut(pointer).unwrap();
            *slot = match pointer {
                "/complete" => false.into(),
                "/calls/0/expanded" => true.into(),
                _ => "changed".into(),
            };
            assert!(inventory_call(&original, &changed).is_err(), "{pointer}");
        }
        for pointer in ["/hir/local", "/span/end"] {
            let mut changed = value["inventory"].clone();
            let row = &mut changed["calls"][0];
            let slot = row.pointer_mut(pointer).unwrap();
            *slot = (slot.as_u64().unwrap() + 1).into();
            assert!(inventory_call(&original, &changed).is_err(), "{pointer}");
            // A second row sharing just HIR or span is also ambiguous.
            let row = changed["calls"][0].clone();
            let mut duplicate = value["inventory"].clone();
            duplicate["calls"].as_array_mut().unwrap().push(row);
            assert!(inventory_call(&original, &duplicate).is_err());
        }
        let mut changed = value["inventory"].clone();
        changed["calls"] = serde_json::json!([]);
        assert!(inventory_call(&original, &changed).is_err());
        let mut changed = value["inventory"].clone();
        changed["calls"][0]["compiled_dependency"] = serde_json::json!({});
        assert!(inventory_call(&original, &changed).is_err());
        let mut wrong_node = original.clone();
        wrong_node.kind = "expr_method_call".into();
        assert!(inventory_call(&wrong_node, &value["inventory"]).is_err());
    }
}

#[test]
fn selected_members_and_structural_substitutions_cannot_be_replaced() {
    for value in originals() {
        let node = node(&value);
        let original = inventory_call(&node, &value["inventory"]).unwrap();
        let observed = observation(&value);
        let mut variants = Vec::new();
        let mut changed = original.clone();
        changed.selected.definition_identity.push_str("changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.requested.definition_identity.push_str("changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.selected.local = false;
        variants.push(changed);
        let mut changed = original.clone();
        changed.requested_arguments.swap(0, 1);
        variants.push(changed);
        let mut changed = original.clone();
        changed.requested_arguments[0].identity_status = "unknown".into();
        variants.push(changed);
        let mut changed = original.clone();
        changed.selected_arguments = changed.requested_arguments.clone();
        variants.push(changed);
        let mut changed = original.clone();
        changed.requested_arguments.pop();
        variants.push(changed);
        for changed in variants {
            assert!(selection(&changed, &observed).is_err());
        }
        // Display text is not structural type authority.
        let mut changed = original;
        changed.requested_arguments[0].value = "display alias".into();
        selection(&changed, &observed).unwrap();
    }
}

#[test]
fn original_selected_context_rejects_foreign_crates_and_other_bodies() {
    for value in originals() {
        let node = node(&value);
        let call = inventory_call(&node, &value["inventory"]).unwrap();
        for context in ["caller_owner", "callee_owner"] {
            for pointer in ["/compiled/crate_name", "/compiled/stable_crate_id"] {
                let mut changed = value.clone();
                *changed[context].pointer_mut(pointer).unwrap() = "foreign".into();
                assert!(
                    selected_context(
                        &call.selected,
                        &owner(&changed["caller_owner"]),
                        &owner(&changed["callee_owner"]),
                    )
                    .is_err()
                );
            }
        }
        for pointer in ["/name", "/identity"] {
            let mut changed = value["callee_owner"].clone();
            *changed.pointer_mut(pointer).unwrap() = "other body".into();
            assert!(
                selected_context(
                    &call.selected,
                    &owner(&value["caller_owner"]),
                    &owner(&changed),
                )
                .is_err()
            );
        }
    }
}

#[test]
fn call_resolution_requires_independent_consistency() {
    let registry = registry();
    for value in originals() {
        let original = node(&value);
        let call = inventory_call(&original, &value["inventory"]).unwrap();
        let resolution = original.res.as_ref().unwrap();
        let observed: Value = serde_json::from_str(&resolution.description).unwrap();
        for field in [
            "path",
            "crate",
            "crate_hash",
            "definition_hash",
            "implementation",
        ] {
            let mut changed = original.clone();
            let mut description = observed.clone();
            description[field] = if field == "implementation" {
                serde_json::json!({"path":"forged"})
            } else {
                "forged".into()
            };
            changed.res.as_mut().unwrap().description = description.to_string();
            assert!(
                requested_resolution(&changed, &call, &registry.core_hash).is_err(),
                "{field}"
            );
        }
        if value["name"] == "wrap" {
            for field in ["path", "crate", "crate_hash", "definition_hash"] {
                let mut changed = original.clone();
                let mut description = observed.clone();
                description["implementation"][field] = "different selected body".into();
                changed.res.as_mut().unwrap().description = description.to_string();
                assert!(requested_resolution(&changed, &call, &registry.core_hash).is_err());
            }
        }
    }
}

#[test]
fn typed_dispatch_requires_independent_consistency() {
    for value in originals() {
        let original = node(&value);
        let call = inventory_call(&original, &value["inventory"]).unwrap();

        for mutation in 0..7 {
            let mut changed = original.clone();
            match mutation {
                0 => changed.children.clear(),
                1 => changed.dispatch.as_mut().unwrap().kind = "overloaded".into(),
                2 => changed.dispatch.as_mut().unwrap().op = "another".into(),
                3 => changed.dispatch.as_mut().unwrap().operand_tys = vec!["bool".into()],
                4 => changed.dispatch.as_mut().unwrap().result_ty = "bool".into(),
                5 => changed.children[0].ty.as_mut().unwrap().normalized = None,
                _ => changed.ty.as_mut().unwrap().normalized = None,
            }
            assert!(typed_call(&value, &changed, &call.signature).is_err());
        }
        for field in ["inputs", "output", "display_only"] {
            let mut changed = call.signature.clone();
            changed[field] = Value::Null;
            assert!(typed_call(&value, &original, &changed).is_err());
        }
    }
}
