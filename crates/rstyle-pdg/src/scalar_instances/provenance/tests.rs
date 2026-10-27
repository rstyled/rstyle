//! Actual compiler provenance slices; whole-body grammar remains a separate proof.
use super::*;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Fixture {
    facts: crate::HirFnFacts,
    record: Value,
    scalar: String,
}

fn fixtures() -> Vec<Fixture> {
    serde_json::from_str(include_str!("fixtures.json")).unwrap()
}

fn check(fixture: &Fixture, record: Value) -> Result<(), String> {
    let record: Observation = serde_json::from_value(record).map_err(|e| e.to_string())?;
    let registry = crate::data_library::registry_for(&fixture.facts).unwrap();
    if record.identity != "rstyle.rust.scalar-instance-provenance/1" || !record.complete {
        return Err("Incomplete provenance fixture".into());
    }

    bound(&record, &fixture.facts, &fixture.scalar, registry)?;
    assertions(&record.assertions, &fixture.facts, registry)
}

#[test]
fn actual_three_target_std_and_core_captures_preserve_exact_bound_and_macro_identities() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 24);
    for fixture in &fixtures {
        check(fixture, fixture.record.clone()).unwrap();
    }
}

#[test]
fn complete_original_parameter_bound_and_compiler_predicate_rosters_are_required() {
    for fixture in &fixtures() {
        for field in ["parameters", "bounds", "compiler_bounds", "assertions"] {
            if fixture.record[field].as_array().unwrap().is_empty() {
                continue;
            }
            let mut omitted = fixture.record.clone();
            omitted[field].as_array_mut().unwrap().pop();
            assert!(check(fixture, omitted).is_err(), "omitted {field}");
            let mut duplicate = fixture.record.clone();
            let first = duplicate[field][0].clone();
            duplicate[field].as_array_mut().unwrap().push(first);
            assert!(check(fixture, duplicate).is_err(), "duplicate {field}");
        }
    }
}

#[test]
fn original_bound_and_assertion_source_spans_cannot_be_relocated() {
    for fixture in &fixtures() {
        for field in [
            "parameter_span",
            "predicate_span",
            "bound_span",
            "path_span",
        ] {
            let mut changed = fixture.record.clone();
            changed["bounds"][0][field]["start"] = json!(0);
            assert!(check(fixture, changed).is_err(), "bound {field}");
        }
        for field in ["span", "declaration_span"] {
            let mut changed = fixture.record.clone();
            changed["parameters"][0][field]["start"] = json!(0);
            assert!(check(fixture, changed).is_err(), "parameter {field}");
        }
        for field in ["span", "callsite"] {
            if fixture.record["assertions"].as_array().unwrap().is_empty() {
                continue;
            }
            let mut changed = fixture.record.clone();
            changed["assertions"][0][field]["start"] = json!(0);
            assert!(check(fixture, changed).is_err(), "assertion {field}");
        }
    }
}

#[test]
fn builtin_name_and_boolean_markers_cannot_replace_resolved_definition_identity() {
    for fixture in &fixtures() {
        for field in ["bounds", "compiler_bounds", "assertions"] {
            if fixture.record[field].as_array().unwrap().is_empty() {
                continue;
            }
            let definition = if field == "assertions" {
                "definition"
            } else {
                "trait_definition"
            };
            let mut changed = fixture.record.clone();
            changed[field][0][definition]["definition_identity"] = json!("unreviewed definition");
            assert!(check(fixture, changed).is_err(), "definition {field}");
            let mut unknown = fixture.record.clone();
            unknown[field][0]["status"] = json!("unknown");
            assert!(check(fixture, unknown).is_err(), "unknown {field}");
        }
        let mut incomplete = fixture.record.clone();
        incomplete["complete"] = json!(false);
        assert!(check(fixture, incomplete).is_err());
    }
}
