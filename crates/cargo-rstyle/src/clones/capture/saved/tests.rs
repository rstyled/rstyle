use super::*;
use serde_json::json;

struct Fixture {
    directory: tempfile::TempDir,
    plan: Plan,
    graph: Value,
    cargo: Vec<Value>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let target = Target {
            package: "sample@1.0.0".into(),
            manifest_dir: "/sample".into(),
            package_context: None,
            source: "/sample/src/lib.rs".into(),
            name: "sample".into(),
            kind: vec!["lib".into()],
            test: true,
        };
        let root = json!({"pkg_id": target.package, "target": {
            "src_path": target.source, "name": target.name, "kind": target.kind, "test": true
        }, "features": [], "mode": "check", "profile": {
            "opt_level": "0", "debuginfo": 2, "debug_assertions": true, "overflow_checks": true
        }});

        let plan = Plan {
            compiler_advisory: false,
            compiler_patterns: false,
            ledger_schema: ledger::SCHEMA.into(),
            extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile::DataV3,
            root: "/sample".into(),
            driver: "/unavailable/driver".into(),
            compiler_identity: "saved rustc".into(),
            cargo_identity: "saved cargo".into(),
            targets: vec![target],
            primary_units: vec![root.clone()],
            dependency_units: vec![],
        };
        let artifact = json!({"reason":"compiler-artifact", "package_id": root["pkg_id"],
            "target": root["target"], "features": [], "filenames":["/saved/libsample.rmeta"],
            "profile": {"test":false, "opt_level":"0", "debuginfo":2,
                "debug_assertions":true, "overflow_checks":true}});
        let fixture = Self {
            directory,
            plan,
            graph: json!({"version":1,"roots":[0],"units":[root]}),
            cargo: vec![artifact, json!({"reason":"build-finished","success":true})],
        };
        fixture.write_ledger(false);
        fixture
    }

    fn ledger(&self, test: bool) -> PathBuf {
        ledger::path(
            &self.plan,
            self.directory.path(),
            &self.plan.targets[0],
            test,
        )
        .unwrap()
    }

    fn write_ledger(&self, test: bool) {
        let directory = self
            .directory
            .path()
            .join(if test { "test" } else { "check" });
        std::fs::create_dir_all(&directory).unwrap();
        let mut invocation = vec!["--emit=metadata".into()];
        if test {
            invocation.push("--test".into());
        }
        let unit = Unit {
            extraction_profile: self.plan.extraction_profile,
            target: self.plan.targets[0].clone(),
            test,
            invocation,
            directory,
            artifacts: vec![],
        };
        write_json(&self.ledger(test), &unit).unwrap();
    }

    fn load(&self) -> Result<Capture, String> {
        write_json(
            &self.directory.path().join("expected-unit-graph.json"),
            &self.graph,
        )
        .unwrap();
        let bytes = self
            .cargo
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(self.directory.path().join("cargo.jsonl"), bytes).unwrap();
        load_saved(self.directory.path(), self.plan.clone())
    }

    fn change_ledger(&self, change: impl FnOnce(&mut Unit)) {
        let mut unit: Unit = read_json(&self.ledger(false)).unwrap();
        change(&mut unit);
        write_json(&self.ledger(false), &unit).unwrap();
    }
}

#[test]
fn standalone_reconciliation_requires_no_closure_or_executable() {
    let fixture = Fixture::new();
    let capture = fixture.load().unwrap();
    assert_eq!(capture.units.len(), 1);
    assert!(capture.failures.is_empty());
    assert!(
        !fixture
            .directory
            .path()
            .join("dependency-closure.json")
            .exists()
    );
}

#[test]
fn cargo_completion_must_be_final_successful_and_unique() {
    for completion in [
        json!({}),
        json!({"reason":"build-finished","success":false}),
    ] {
        let mut fixture = Fixture::new();
        fixture.cargo[1] = completion;
        assert!(fixture.load().is_err());
    }
    let mut fixture = Fixture::new();
    fixture.cargo.pop();
    assert!(fixture.load().is_err());
    fixture
        .cargo
        .push(json!({"reason":"build-finished","success":true}));
    fixture
        .cargo
        .push(json!({"reason":"build-finished","success":true}));
    assert!(fixture.load().is_err());
}

#[test]
fn saved_graph_roots_are_exact_unique_nonempty_and_in_bounds() {
    for roots in [
        json!([]),
        json!([0, 0]),
        json!([1]),
        json!([-1]),
        json!(["0"]),
    ] {
        let mut fixture = Fixture::new();
        fixture.graph["roots"] = roots;
        assert!(fixture.load().is_err());
    }
    let mut fixture = Fixture::new();
    fixture.graph["version"] = json!(2);
    assert!(fixture.load().is_err());
    fixture.graph["version"] = json!(1);
    fixture.graph["units"][0]["features"] = json!(["changed"]);
    assert!(fixture.load().is_err());
}

#[test]
fn dependencies_old_schema_and_unexplained_or_duplicate_targets_fail() {
    let mut fixture = Fixture::new();
    fixture
        .plan
        .dependency_units
        .push(fixture.plan.primary_units[0].clone());
    assert!(fixture.load().is_err());
    fixture.plan.dependency_units.clear();

    fixture.plan.ledger_schema = "old".into();
    assert!(fixture.load().is_err());
    fixture.plan.ledger_schema = ledger::SCHEMA.into();
    fixture.plan.targets.push(fixture.plan.targets[0].clone());
    assert!(fixture.load().is_err());
    fixture.plan.targets[1].name = "unselected".into();
    assert!(fixture.load().is_err());
}

#[test]
fn missing_duplicate_and_foreign_ledgers_fail() {
    let mut fixture = Fixture::new();
    fixture.cargo.insert(0, fixture.cargo[0].clone());
    assert!(fixture.load().is_err());
    fixture.cargo.remove(0);
    fixture.change_ledger(|unit| unit.directory = "/another/run/unit".into());
    assert!(fixture.load().is_err());
    std::fs::remove_file(fixture.ledger(false)).unwrap();
    assert!(fixture.load().is_err());
}

#[test]
fn changed_artifact_context_and_ledger_invocation_fail() {
    let mut fixture = Fixture::new();
    fixture.cargo[0]["profile"]["debug_assertions"] = json!(false);
    assert!(fixture.load().is_err());
    fixture.cargo[0]["profile"]["debug_assertions"] = json!(true);

    fixture.change_ledger(|unit| unit.invocation.push("--test".into()));
    assert!(fixture.load().is_err());
}

#[test]
fn unselected_graph_dependencies_do_not_expand_saved_scope() {
    let mut fixture = Fixture::new();
    let mut dependency = fixture.graph["units"][0].clone();
    dependency["pkg_id"] = json!("other@2.0.0");
    fixture.graph["units"]
        .as_array_mut()
        .unwrap()
        .push(dependency);
    assert_eq!(fixture.load().unwrap().units.len(), 1);
}

#[test]
fn check_and_test_modes_retain_both_ledgers_and_reject_missing_one() {
    let mut fixture = Fixture::new();
    fixture.write_ledger(true);
    let test_root = fixture.graph["units"][0].clone();
    fixture.graph["units"]
        .as_array_mut()
        .unwrap()
        .push(test_root.clone());
    fixture.graph["roots"] = json!([0, 1]);
    fixture.plan.primary_units.push(test_root);
    let mut test_artifact = fixture.cargo[0].clone();
    test_artifact["profile"]["test"] = json!(true);
    fixture.cargo.insert(1, test_artifact);

    assert_eq!(fixture.load().unwrap().units.len(), 2);
    std::fs::remove_file(fixture.ledger(true)).unwrap();
    assert!(fixture.load().is_err());
}
