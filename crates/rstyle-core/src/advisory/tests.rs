use super::*;
use crate::facts::{ArgSource, CallEvent, Edge, MutationEvent, ParamFacts, sample};

fn original() -> serde_json::Value {
    let mut facts = sample("f");
    facts.param_list.push(ParamFacts {
        name: "value".into(),
        ty: "u128".into(),
        uses: 2,
    });
    facts.mutations.push(MutationEvent {
        origin: crate::facts::MutationOrigin::Source,
        root: "value".into(),
        field_path: String::new(),
        line: 1,
        escapes: false,
        delegated: false,
    });
    facts.calls.push(CallEvent {
        callee: "external".into(),
        line: 1,
        dynamic: true,
    });
    facts.edges.push(Edge {
        callee: "local".into(),
        line: 1,
        dynamic: false,
        args: vec![ArgSource::Param(0)],
    });

    serde_json::to_value(Observation::Known {
        metrics: METRICS.into(),
        facts: Box::new(facts),
    })
    .unwrap()
}

#[test]
fn compiler_observations_reject_legacy_defaults_instead_of_fabricating_zero_facts() {
    let original = original();
    assert!(Observation::read(&original).is_ok());
    for field in [
        "id",
        "implements",
        "param_list",
        "top_statements",
        "edges",
        "mutated_types",
        "sig",
        "pdg",
    ] {
        let mut changed = original.clone();
        changed["facts"].as_object_mut().unwrap().remove(field);
        assert!(Observation::read(&changed).is_err(), "{field}");
    }
    for (roster, field) in [
        ("param_list", "uses"),
        ("mutations", "delegated"),
        ("mutations", "origin"),
        ("calls", "dynamic"),
        ("edges", "args"),
    ] {
        let mut changed = original.clone();
        changed["facts"][roster][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Observation::read(&changed).is_err(), "{roster}.{field}");
    }
}

#[test]
fn compiler_metric_versions_and_missing_observations_stay_distinct() {
    let mut changed = original();
    changed["metrics"] = LEGACY_METRICS.into();
    let Observation::Known { metrics, .. } = Observation::read(&changed).unwrap() else {
        panic!("Historical observation must remain known");
    };
    assert_eq!(metrics, LEGACY_METRICS);
    changed["metrics"] = "unregistered/2".into();
    assert!(Observation::read(&changed).is_err());
    let unavailable = serde_json::json!({"status":"unavailable", "reason":"No original source"});
    assert!(matches!(
        Observation::read(&unavailable),
        Ok(Observation::Unavailable { .. })
    ));
    assert!(Observation::read(&serde_json::json!({"status":"known", "facts":null})).is_err());
}

#[test]
fn mutation_origins_require_current_evidence_and_preserve_historical_semantics() {
    let mut raw = original();
    raw["facts"]["mutations"][0]["origin"] = "for_desugaring".into();
    assert!(Observation::read(&raw).is_ok());
    raw["metrics"] = LEGACY_METRICS.into();
    assert!(Observation::read(&raw).is_err());
    raw["facts"]["mutations"][0]
        .as_object_mut()
        .unwrap()
        .remove("origin");
    let Observation::Known { facts, .. } = Observation::read(&raw).unwrap() else {
        panic!("Historical record must remain known");
    };
    assert_eq!(
        facts.mutations[0].origin,
        crate::facts::MutationOrigin::Source
    );
    raw["metrics"] = METRICS.into();
    assert!(Observation::read(&raw).is_err());
    raw["facts"]["mutations"][0]["origin"] = "unknown".into();
    assert!(Observation::read(&raw).is_err());
}

#[test]
fn historical_statement_counts_and_v2_origin_requirements_are_preserved() {
    for version in [LEGACY_METRICS, V2_METRICS, V3_METRICS, V4_METRICS, METRICS] {
        let mut raw = original();
        raw["metrics"] = version.into();
        raw["facts"]["statements"] = 5.into();
        let Observation::Known { metrics, facts } = Observation::read(&raw).unwrap() else {
            panic!("Known historical compiler observation");
        };
        assert_eq!(metrics, version);
        assert_eq!(facts.statements, 5);
        raw["facts"]["mutations"][0]
            .as_object_mut()
            .unwrap()
            .remove("origin");
        assert_eq!(Observation::read(&raw).is_ok(), version == LEGACY_METRICS);
    }
}
