//! Actual retained compiler captures plus independently specified adversarial mutations.
use super::*;
use serde_json::json;

#[derive(Deserialize)]
struct Fixture {
    canonical: HirFnFacts,
    conversions: Vec<Value>,
    haystack: String,
}

fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

#[test]
fn original_slice_and_str_selections_pass_without_origin_or_admission_authority() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 2);
    for fixture in fixtures {
        verify(&fixture.canonical, &fixture.conversions, &fixture.haystack).unwrap();
    }
}

#[test]
fn call_observation_proof_preserves_unresolved_whole_body_findings() {
    for mut fixture in fixtures() {
        fixture.canonical.source_scan_complete = false;
        fixture
            .canonical
            .source_diagnostics
            .push(crate::HirDiagnostic {
                capability: "rust.call-summaries".into(),
                construct: "expr_method_call".into(),
                span: crate::ByteSpan {
                    start: 396,
                    end: 413,
                },
                snippet: "haystack.as_ref()".into(),
                severity: "error".into(),
                reason: "Original generic call still requires projected body proof".into(),
            });
        let before = fixture.canonical.clone();
        verify(&fixture.canonical, &fixture.conversions, &fixture.haystack).unwrap();
        assert_eq!(fixture.canonical, before);
    }
}

#[test]
fn every_native_definition_component_requires_reviewed_authority() {
    for fixture in fixtures() {
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
                "crate_hash",
                "stable_crate_id",
                "local",
            ] {
                let mut changed = fixture.conversions.clone();
                changed[0][definition][field] = if field == "local" {
                    json!(true)
                } else {
                    json!("custom")
                };
                assert!(
                    verify(&fixture.canonical, &changed, &fixture.haystack).is_err(),
                    "{definition}.{field}"
                );
            }
        }
    }
}

#[test]
fn substitution_signatures_and_binder_equality_cannot_invent_library_semantics() {
    for fixture in fixtures() {
        let mut substituted = fixture.conversions.clone();
        substituted[0]["requested_arguments"][0]["identity"] = if fixture.haystack == "str" {
            json!({"kind":"slice","element":{"kind":"builtin","name":"u8"}})
        } else {
            json!({"kind":"str"})
        };
        assert!(verify(&fixture.canonical, &substituted, &fixture.haystack).is_err());
        for pointer in [
            "/requested_arguments/0/identity/kind",
            "/requested_arguments/1/identity/element/name",
            "/signature/inputs/0/identity/region",
            "/signature/output/0/identity/mutable",
            "/result_type/0/identity/referent/kind",
        ] {
            let mut changed = fixture.conversions.clone();
            *changed[0].pointer_mut(pointer).unwrap() = json!("wrong");
            assert!(
                verify(&fixture.canonical, &changed, &fixture.haystack).is_err(),
                "{pointer}"
            );
        }
        let mut changed = fixture.conversions.clone();
        changed[0]["signature"]["receiver_region"]["variable"] = json!(1);
        changed[0]["signature"]["result_region"]["variable"] = json!(1);
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
        changed[0] = fixture.conversions[0].clone();
        changed[0]["signature"]["result_region"]["binder_depth"] = json!(1);
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
    }
}

#[test]
fn as_ref_receiver_must_resolve_to_the_original_formal() {
    for mut fixture in fixtures() {
        let mut body = serde_json::to_value(&fixture.canonical.body).unwrap();
        wrong_formal(&mut body);
        fixture.canonical.body = serde_json::from_value(body).unwrap();
        assert!(verify(&fixture.canonical, &fixture.conversions, &fixture.haystack).is_err());
    }
}

fn wrong_formal(node: &mut Value) {
    if node["dispatch"]["op"] == "core::convert::AsRef::as_ref" {
        node["children"][0]["res"]["target"]["local"] = json!(999999);
        return;
    }
    for child in node["children"].as_array_mut().unwrap() {
        wrong_formal(child);
    }
}

#[test]
fn receiver_adjustment_order_endpoints_and_saved_formal_cannot_be_transplanted() {
    for fixture in fixtures() {
        let mut changed = fixture.conversions.clone();
        changed[0]["receiver_adjustments"]["steps"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
        for pointer in [
            "/hir/local",
            "/span/start",
            "/receiver_adjustments/hir/local",
            "/receiver_adjustments/span/end",
        ] {
            changed = fixture.conversions.clone();
            *changed[0].pointer_mut(pointer).unwrap() = json!(999999);
            assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
        }
        changed = fixture.conversions.clone();
        changed[0]["receiver_adjustments"]["steps"][1]["target"][0]["identity"]["mutable"] =
            json!(true);
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
    }
}

#[test]
fn missing_duplicate_repeated_and_foreign_context_observations_fail_closed() {
    for fixture in fixtures() {
        assert!(verify(&fixture.canonical, &[], &fixture.haystack).is_err());
        let mut changed = fixture.conversions.clone();
        changed.push(changed[0].clone());
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
        assert!(verify(&fixture.canonical, &fixture.conversions, "Custom").is_err());
        let mut canonical = fixture.canonical.clone();
        canonical.source.bytes = canonical
            .source
            .bytes
            .replace(
                "let haystack = haystack.as_ref();",
                "let haystack = haystack.as_ref(); haystack.as_ref();",
            )
            .into();
        assert!(verify(&canonical, &fixture.conversions, &fixture.haystack).is_err());
        canonical = fixture.canonical.clone();
        canonical.build.target_triple.value = Some("x86_64-unknown-linux-gnu".into());
        assert!(verify(&canonical, &fixture.conversions, &fixture.haystack).is_err());
    }
}

#[test]
fn diagnostic_displays_do_not_identify_projected_types_and_effect_flags_are_rejected() {
    for fixture in fixtures() {
        let mut changed = fixture.conversions.clone();
        changed[0]["requested_arguments"][0]["value"] = json!("custom display text");
        verify(&fixture.canonical, &changed, &fixture.haystack).unwrap();
        changed[0]["pure"] = json!(true);
        assert!(verify(&fixture.canonical, &changed, &fixture.haystack).is_err());
    }
}
