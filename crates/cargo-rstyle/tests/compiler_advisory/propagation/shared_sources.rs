use super::*;

const PARAMETERS: &str = "a:u128,b:u128,c:u128,d:u128,e:u128";
const SHARED: &str = "pub fn helper(a:u128,b:u128,c:u128,d:u128,e:u128) { std::fs::write(\"/unused\",b\"shared\").ok(); }";

fn fixture(source: &str) -> TempDir {
    let directory = workspace("pub fn unrelated() {}");
    std::fs::create_dir(directory.path().join("shared")).unwrap();
    std::fs::write(directory.path().join("shared/leaf.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!(
            "#[path=\"../../shared/leaf.rs\"] mod shared; pub fn caller({PARAMETERS}) {{ shared::helper(a,b,c,d,e); }}"
        ),
    )
    .unwrap();
    directory
}

#[test]
fn shared_workspace_source_reuses_effects_and_parameter_planning_without_strict_admission() {
    let directory = fixture(SHARED);
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["metrics"], "rstyle.compiler-style-metrics/7");
    assert_eq!(advisory["complete"], true);
    let caller = unit(&report, "caller");
    assert_eq!(caller["observations"].as_array().unwrap().len(), 2);
    assert_eq!(caller["unavailable"], serde_json::json!([]));
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    assert_eq!(io(caller), 2);
    assert!(
        caller["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["path"] == "shared/leaf.rs"
                && diagnostic["rule_id"] == "fn_params")
    );
    assert_eq!(advisory["planning"]["input_complete"], true);
    let groups = advisory["planning"]["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    let steps = groups[0]["evaluation"]["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0]["incoming_sites"], 1);
    assert_eq!(steps[1]["outgoing_sites"], 1);
    assert_eq!(
        steps[1]["pass_through"],
        serde_json::json!(["a", "b", "c", "d", "e"])
    );
    std::fs::remove_dir_all(directory.path().join("shared")).unwrap();
    super::concrete_traits::source_free_replay(directory.path(), &report);
}

#[test]
fn absolute_shared_workspace_source_supplies_original_metrics_and_call_edges() {
    let directory = fixture(SHARED);
    let source = directory.path().join("shared/leaf.rs");
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!(
            "#[path={:?}] mod shared; pub fn caller({PARAMETERS}) {{ shared::helper(a,b,c,d,e); }}",
            source.to_str().unwrap()
        ),
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let caller = unit(&report, "caller");
    assert_eq!(caller["observations"].as_array().unwrap().len(), 2);
    assert_eq!(caller["unavailable"], serde_json::json!([]));
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    assert_eq!(io(caller), 2);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        true
    );
}

#[test]
fn excluded_shared_source_does_not_supply_effects_or_a_completed_plan() {
    let directory = fixture(SHARED);
    std::fs::write(
        directory.path().join("policy.toml"),
        "exclude=['shared/**']\n",
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &[
            "--compiler-plans",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 2);
    let caller = unit(&report, "caller");
    assert_eq!(io(caller), 0);
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["caller::shared::helper"])
    );
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn generated_shared_effects_propagate_without_authorizing_source_metrics_or_plans() {
    let directory = fixture(&format!("macro_rules! make {{()=>{{{SHARED}}}}} make!();"));
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    let caller = unit(&report, "caller");
    assert_eq!(io(caller), 1);
    assert_eq!(caller["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn files_outside_the_workspace_do_not_gain_original_collector_metrics() {
    let directory = fixture(SHARED);
    let outside = TempDir::new().unwrap();
    let source = outside.path().join("foreign.rs");
    std::fs::write(&source, SHARED).unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!(
            "#[path={:?}] mod shared; pub fn caller({PARAMETERS}) {{ shared::helper(a,b,c,d,e); }}",
            source.to_str().unwrap()
        ),
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    let caller = unit(&report, "caller");
    assert_eq!(io(caller), 0);
    assert_eq!(caller["observations"].as_array().unwrap().len(), 1);
    assert_eq!(caller["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn shared_source_check_and_test_owners_keep_their_compilation_contexts() {
    let source =
        format!("#[cfg(not(test))] {SHARED}\n#[cfg(test)] pub fn helper({PARAMETERS}) {{}}");
    let directory = fixture(&source);
    let (_output, report) = capture(directory.path(), &["--all-targets", "--compiler-plans"]);
    let callers: Vec<_> = report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|unit| unit["package_root"].as_str().unwrap().ends_with("caller"))
        .collect();
    assert_eq!(callers.len(), 2);
    assert_ne!(callers[0]["unit"], callers[1]["unit"]);
    for caller in callers {
        let id = caller["unit"].as_u64().unwrap() as usize;
        let observed = report["unit_contexts"][id]["observed_unit"]
            .as_u64()
            .unwrap() as usize;
        let test = report["observed_units"][observed]["test"]
            .as_bool()
            .unwrap();
        assert_eq!(io(caller), if test { 0 } else { 2 });
        assert_eq!(caller["observations"].as_array().unwrap().len(), 2);
        assert_eq!(
            caller["unavailable"].as_array().unwrap().len(),
            usize::from(test)
        );
        for name in ["caller", "shared::helper"] {
            assert!(
                caller["observations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|observation| observation["name"].as_str().unwrap().ends_with(name))
            );
        }
        assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    }
}
