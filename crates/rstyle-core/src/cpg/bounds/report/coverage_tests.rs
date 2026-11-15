use super::tests::{assessment, row};
use super::*;
use crate::cpg::{records::Demand, run_records};
use runtime_bounds::{Bound, OpaqueKind, compose::Composition};
use serde_json::{Value, json};

fn records() -> Vec<Record> {
    let document =
        serde_json::from_str(include_str!("../../testdata/type-call-py.cpg.json")).unwrap();
    let demand = Demand {
        bounds: true,
        ..Demand::default()
    };
    run_records(vec![document], demand)
        .unwrap()
        .into_iter()
        .filter(|record| matches!(record.measured, Measured::Known(_)))
        .collect()
}

fn composition(loops: Bound, library: Bound) -> Composition {
    Composition {
        cost_scope: "test".into(),
        opaque_factors: Vec::new(),
        loop_body_entries: loops,
        workspace_invocations: Bound::Const(0),
        library_steps: library,
        library_factors: Vec::new(),
        resolved_callees: Vec::new(),
        excluded_library_callees: Vec::new(),
        assumptions: Vec::new(),
    }
}

#[test]
fn coverage_separates_proofs_from_opaque_or_absent_answers() {
    let known = |bound, lines: &[usize]| row(Outcome::Proven(assessment(bound, lines)));
    let mut rows = vec![
        known(Bound::Const(0), &[]),
        known(Bound::Const(3), &[1]),
        known(Bound::Input("n".into()), &[2]),
        row(Outcome::Partial(assessment(
            Bound::opaque(OpaqueKind::Loop, "x"),
            &[3],
        ))),
        row(Outcome::Unknown { reason: "x".into() }),
        row(Outcome::BudgetExceeded { reason: "x".into() }),
        row(Outcome::Unknown {
            reason: "missing".into(),
        }),
    ];
    assert!(Coverage::new(&[], &rows).input_complete());
    rows[6].evidence = evidence(Err("missing".into()));
    assert!(!Coverage::new(&[], &rows).input_complete());
    let coverage = serde_json::to_value(Coverage::new(&[], &rows)).unwrap();

    assert_eq!(
        coverage["local"],
        json!({"proven": 3, "partial": 1, "unknown": 1,
        "budget_exceeded": 1, "not_available": 1})
    );
    assert_eq!(
        coverage["proven_local_loops"],
        json!({"without_modeled_loops": 1,
        "constant_loop_work": 1, "input_dependent_loop_work": 1})
    );
    assert_eq!(coverage["composed"], json!({"not_available": 7}));
    assert_eq!(coverage["library_steps"], json!({"not_available": 7}));
}

#[test]
fn opaque_library_work_does_not_turn_a_composed_proof_into_complete_runtime() {
    let compositions = [
        Composed::Proven(composition(Bound::Const(0), Bound::Const(0))),
        Composed::Proven(composition(
            Bound::Const(3),
            Bound::opaque(OpaqueKind::Library, "x"),
        )),
        Composed::Proven(composition(Bound::Input("n".into()), Bound::Const(0))),
        Composed::Partial(composition(
            Bound::opaque(OpaqueKind::Call, "x"),
            Bound::Const(0),
        )),
        Composed::Unknown { reason: "x".into() },
        Composed::BudgetExceeded { reason: "x".into() },
    ];
    let rows: Vec<_> = compositions
        .into_iter()
        .map(|composed| {
            let mut item = row(Outcome::Unknown { reason: "x".into() });
            item.evidence.composed = Some(composed);
            item
        })
        .collect();
    let coverage = serde_json::to_value(Coverage::new(&[], &rows)).unwrap();

    assert_eq!(
        coverage["composed"],
        json!({"proven": 3, "partial": 1, "unknown": 1, "budget_exceeded": 1})
    );
    assert_eq!(
        coverage["proven_composed_loops"],
        json!({"zero_loop_work": 1,
        "constant_loop_work": 1, "input_dependent_loop_work": 1})
    );
    assert_eq!(
        coverage["library_steps"],
        json!({"complete": 3, "opaque": 1, "not_available": 2})
    );
}

#[test]
fn top_selection_cannot_hide_unavailable_or_budget_exhausted_capture() {
    let mut records = records();
    assert!(from_records(&records, None).input_complete);
    for input in [
        None,
        Some(Input::BudgetExhausted {
            reason: "limit".into(),
        }),
    ] {
        records[0].bounds = input;
        let all = serde_json::to_value(from_records(&records, None)).unwrap();
        let top = serde_json::to_value(from_records(&records, Some(0))).unwrap();
        assert_eq!(top["rows"], json!([]));
        assert_eq!(top["coverage"], all["coverage"]);
        assert_eq!(top["input_complete"], false);
    }
}

#[test]
fn roster_keeps_excluded_kinds_and_unavailable_records_visible() {
    let mut records = records();
    let Measured::Known(facts) = records[0].measured.clone() else {
        panic!()
    };
    let mut body = records[0].clone();
    body.measured = Measured::Body(facts);
    let mut generated = body.clone();
    generated.measured = Measured::Generated(Box::new(body.measured.observation().unwrap()));
    let mut missing = body.clone();
    missing.measured = Measured::Unavailable("no source".into());
    records.extend([body, generated, missing]);
    let report = serde_json::to_value(from_records(&records, None)).unwrap();

    assert_eq!(
        report["coverage"]["roster"],
        json!({"authored_functions": 2,
        "non_function_bodies": 1, "generated": 1, "unavailable": 1})
    );
    assert_eq!(report["rows"].as_array().unwrap().len(), 2);
    assert_eq!(report["input_complete"], false);
}

#[test]
fn identities_survive_review_field_removal_and_disambiguate_duplicate_names() {
    let mut records = records();
    let original = records[0].clone();
    let mut second = original.clone();
    second.compilation = "another-compilation".into();
    second.test = true;
    records = vec![original, second.without_review_fields()];
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    let rows = report["rows"].as_array().unwrap();

    assert_eq!(rows[0]["identity"], rows[1]["identity"]);
    assert_ne!(rows[0]["compilation"], rows[1]["compilation"]);
    for key in ["function", "root", "line", "column", "path"] {
        assert_ne!(rows[0][key], Value::Null, "{key}");
        assert_eq!(rows[0][key], rows[1][key], "{key}");
    }
    assert_eq!(rows[1]["test"], true);
    assert!(rows[0]["line"].as_u64().unwrap() > 0);
}

#[test]
fn duplicate_summary_keys_never_supply_a_composed_proof() {
    let mut records = records();
    records.push(records[0].clone());
    let report = serde_json::to_value(from_records(&records, None)).unwrap();
    let rows = report["rows"].as_array().unwrap();
    let duplicates = rows
        .iter()
        .filter(|row| row["identity"] == records[0].identity);

    assert_eq!(duplicates.clone().count(), 2);
    assert!(
        duplicates
            .into_iter()
            .all(|row| row["composed"].get("Unknown").is_some())
    );
    let empty = from_records(&[], Some(0));
    assert!(empty.input_complete);
    assert!(empty.rows.is_empty());
}
