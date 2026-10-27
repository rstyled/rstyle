use super::*;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Fixture {
    target: String,
    node: HirNode,
    record: Value,
}

fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

fn check(fixture: &Fixture, record: Value) -> Result<Checked, String> {
    let record: Observation = serde_json::from_value(record).map_err(|error| error.to_string())?;
    let checked = values(&fixture.node, &record)?;
    let registry = crate::data_library::registries()
        .iter()
        .find(|registry| registry.target == fixture.target)
        .unwrap();
    pins::verify(&record, checked, registry)?;
    Ok(checked)
}

#[test]
fn actual_three_target_core_and_std_captures_preserve_all_six_signed_and_unsigned_operations() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 72);
    for fixture in &fixtures {
        let checked = check(fixture, fixture.record.clone()).unwrap();
        assert_eq!(
            checked.operator,
            Operator::parse(fixture.record["operator"].as_str().unwrap()).unwrap()
        );
        assert_eq!(fixture.node.dispatch.as_ref().unwrap().kind, "overloaded");
    }
}

#[test]
fn every_definition_component_requires_its_exact_pinned_identity() {
    let fixtures = fixtures();
    for fixture in &fixtures {
        for definition in [
            "requested",
            "requested_trait",
            "selected",
            "selected_impl",
            "selected_trait",
        ] {
            for field in [
                "path",
                "definition_identity",
                "crate",
                "stable_crate_id",
                "crate_hash",
            ] {
                let mut damaged = fixture.record.clone();
                damaged[definition][field] = json!("untrusted");
                assert!(check(fixture, damaged).is_err(), "{definition}.{field}");
            }
            let mut damaged = fixture.record.clone();
            damaged[definition]["local"] = json!(true);
            assert!(check(fixture, damaged).is_err());
        }
    }
}

#[test]
fn displays_cannot_override_structural_signedness_or_signature() {
    let fixtures = fixtures();
    let fixture = &fixtures[0];
    let mut display = fixture.record.clone();
    display["requested_arguments"][0]["value"] = json!("untrusted display");
    assert!(check(fixture, display).is_ok());
    for field in ["requested_arguments", "operand_types", "result_type"] {
        let mut damaged = fixture.record.clone();
        damaged[field][0]["identity"]["name"] = json!("f32");
        assert!(check(fixture, damaged).is_err(), "{field}");
    }
    for field in ["requested_arguments", "operand_types"] {
        let mut damaged = fixture.record.clone();
        damaged[field][1]["identity"]["name"] = json!("f32");
        assert!(check(fixture, damaged).is_err(), "second {field}");
    }
    for signature in ["requested_signature", "signature"] {
        for operand in [0, 1] {
            let mut damaged = fixture.record.clone();
            damaged[signature]["inputs"][operand]["identity"]["mutable"] = json!(true);
            assert!(check(fixture, damaged).is_err());
        }
        let mut damaged = fixture.record.clone();
        damaged[signature]["output"][0]["identity"]["name"] = json!("u32");
        assert!(check(fixture, damaged).is_err());
    }
}

#[test]
fn real_definition_swaps_across_signedness_operator_and_target_are_rejected() {
    let fixtures = fixtures();
    let fixture = &fixtures[0];
    for other in &fixtures[1..] {
        if fixture.record["selected"]["definition_identity"]
            == other.record["selected"]["definition_identity"]
        {
            continue;
        }
        let mut damaged = fixture.record.clone();
        for field in ["selected", "selected_impl", "selected_trait"] {
            damaged[field] = other.record[field].clone();
        }
        assert!(check(fixture, damaged).is_err());
    }
}

#[test]
fn ordered_autorefs_cannot_be_missing_duplicated_mutable_or_reassigned_to_another_operand() {
    let fixtures = fixtures();
    let fixture = &fixtures[0];
    for operand in [0, 1] {
        for steps in [json!([]), json!([{"kind":"builtin_deref","target":[]}])] {
            let mut damaged = fixture.record.clone();
            damaged["operand_adjustments"][operand]["steps"] = steps;
            assert!(check(fixture, damaged).is_err());
        }
        for field in ["source_type", "adjusted_type"] {
            let mut damaged = fixture.record.clone();
            damaged["operand_adjustments"][operand][field] = json!([]);
            assert!(check(fixture, damaged).is_err());
        }
        let mut damaged = fixture.record.clone();
        damaged["operand_adjustments"][operand]["steps"][0]["target"][0]["identity"]["mutable"] =
            json!(true);
        assert!(check(fixture, damaged).is_err());
        let mut damaged = fixture.record.clone();
        damaged["operand_adjustments"][operand]["hir"] = json!({"krate":0,"owner":0,"local":0});
        assert!(check(fixture, damaged).is_err());
    }
}

#[test]
fn unknown_incomplete_instances_and_changed_dispatch_remain_rejected() {
    let mut fixtures = fixtures();
    let fixture = &mut fixtures[0];
    for status in ["unresolved", "unsupported", "error", ""] {
        let mut damaged = fixture.record.clone();
        damaged["status"] = json!(status);
        assert!(check(fixture, damaged).is_err());
    }
    for field in [
        "requested_signature",
        "operand_adjustments",
        "selected_impl",
    ] {
        let mut damaged = fixture.record.clone();
        damaged.as_object_mut().unwrap().remove(field);
        assert!(check(fixture, damaged).is_err());
    }
    fixture.node.dispatch.as_mut().unwrap().kind = "builtin".into();
    assert!(check(fixture, fixture.record.clone()).is_err());
}

#[test]
fn top_level_comparison_adjustments_require_the_exact_supplied_operand_record() {
    let fixtures = fixtures();
    let fixture = &fixtures[0];
    let comparison: Observation = serde_json::from_value(fixture.record.clone()).unwrap();
    let scalar = crate::frontend::normalized(&fixture.node.children[0]).unwrap();
    assert_eq!(site(&comparison), &fixture.node.hir);
    for (index, operand) in fixture.node.children.iter().enumerate() {
        let raw = fixture.record["operand_adjustments"][index].clone();
        let top: Adjustments = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(adjustment_site(&top), operand_sites(&comparison)[index]);
        assert!(verify_adjustments(operand, &top, Some(&comparison), scalar).is_ok());
        assert!(verify_adjustments(operand, &top, None, scalar).is_err());
        for field in ["source_type", "adjusted_type", "steps"] {
            let mut damaged = raw.clone();
            damaged[field] = json!([]);
            let damaged: Adjustments = serde_json::from_value(damaged).unwrap();
            assert!(verify_adjustments(operand, &damaged, Some(&comparison), scalar).is_err());
        }
        let mut display = raw;
        display["source_type"][0]["value"] = json!("display cannot identify an adjustment");
        let display: Adjustments = serde_json::from_value(display).unwrap();
        assert!(verify_adjustments(operand, &display, Some(&comparison), scalar).is_ok());
    }
}

fn scalar_argument(name: &str) -> Value {
    json!({"kind":"type","value":name,"display_only":true,"identity_status":"known","identity":{"kind":"builtin","name":name}})
}

fn return_continuation() -> (HirNode, Value, String) {
    let fixtures = fixtures();
    let child = fixtures[0].node.children[0].clone();
    let scalar = crate::frontend::normalized(&child).unwrap().to_owned();
    let mut node = child.clone();
    node.kind = "expr_ret".into();
    node.children = vec![child];
    let record = json!({
        "hir":node.hir,"span":node.span,"source_type":[scalar_argument("!")],
        "steps":[{"kind":"never_to_any","target":[scalar_argument(&scalar)]}],
        "adjusted_type":[scalar_argument(&scalar)]
    });
    (node, record, scalar)
}

#[test]
fn never_to_any_is_bounded_to_exact_scalar_return_continuations() {
    let (mut node, raw, scalar) = return_continuation();
    let record: Adjustments = serde_json::from_value(raw.clone()).unwrap();
    assert!(verify_adjustments(&node, &record, None, &scalar).is_ok());
    for kind in ["expr_path", "expr_block", "expr_break", "expr_loop"] {
        node.kind = kind.into();
        assert!(verify_adjustments(&node, &record, None, &scalar).is_err());
    }
    node.kind = "expr_ret".into();
    for field in ["source_type", "adjusted_type"] {
        let mut damaged = raw.clone();
        damaged[field] = json!([scalar_argument("bool")]);
        let damaged: Adjustments = serde_json::from_value(damaged).unwrap();
        assert!(verify_adjustments(&node, &damaged, None, &scalar).is_err());
    }
    let mut damaged = raw;
    damaged["steps"][0]["target"] = json!([scalar_argument("bool")]);
    let damaged: Adjustments = serde_json::from_value(damaged).unwrap();
    assert!(verify_adjustments(&node, &damaged, None, &scalar).is_err());
    node.children.clear();
    assert!(verify_adjustments(&node, &record, None, &scalar).is_err());
}

#[derive(Deserialize)]
struct SourcePart {
    name: String,
    bytes: String,
}

#[test]
fn reviewed_core_macro_definitions_and_integer_invocations_match_exact_installed_source_bytes() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let source = std::fs::read_to_string(
        std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src/cmp.rs"),
    )
    .unwrap();
    let parts: Vec<SourcePart> = serde_json::from_str(include_str!("source-parts.json")).unwrap();
    assert_eq!(parts.len(), 5);
    for part in parts {
        assert_eq!(
            source.match_indices(&part.bytes).count(),
            1,
            "{}",
            part.name
        );
    }
}

#[test]
fn independently_specified_integer_truth_tables_cover_equality_signedness_and_boundaries() {
    for (left, right, expected) in [
        (i32::MIN, i32::MAX, [false, true, true, true, false, false]),
        (-1, 0, [false, true, true, true, false, false]),
        (7, 7, [true, false, false, true, false, true]),
        (i32::MAX, i32::MIN, [false, true, false, false, true, true]),
    ] {
        assert_eq!(
            [
                left == right,
                left != right,
                left < right,
                left <= right,
                left > right,
                left >= right
            ],
            expected
        );
    }
    for (left, right, expected) in [
        (0, u32::MAX, [false, true, true, true, false, false]),
        (u32::MAX, 0, [false, true, false, false, true, true]),
        (u32::MAX, u32::MAX, [true, false, false, true, false, true]),
    ] {
        assert_eq!(
            [
                left == right,
                left != right,
                left < right,
                left <= right,
                left > right,
                left >= right
            ],
            expected
        );
    }
}
