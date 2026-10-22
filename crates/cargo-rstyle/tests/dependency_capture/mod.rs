//! Explicit dependency capture preserves unit boundaries without granting call trust.
use super::*;

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::create_dir_all(directory.path().join("outside/src")).unwrap();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nname='clone_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\nexclude=['outside']\n[dependencies]\nrenamed={package='scalar_dependency',path='outside',features=['selected']}\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("outside/Cargo.toml"),
        "[package]\nname='scalar_dependency'\nversion='0.1.0'\nedition='2021'\n[features]\nselected=[]\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("outside/src/lib.rs"),
        "#[cfg(feature=\"selected\")] pub fn selected_value(x: u8) -> u8 { x }\n#[cfg(not(feature=\"selected\"))] pub fn inactive_value(x: u8) -> u8 { x }\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn caller(x: u8) -> u8 { renamed::selected_value(x) }\npub fn local(x: u8) -> u8 { x }\n",
    )
    .unwrap();
    directory
}

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn inventory(report: &Value, dependency_count: usize, observed_count: usize) {
    assert_eq!(report["capture_complete"], true);
    assert_eq!(
        report["intended_primary_units"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        report["intended_dependency_units"]
            .as_array()
            .unwrap()
            .len(),
        dependency_count
    );
    assert_eq!(
        report["observed_units"].as_array().unwrap().len(),
        observed_count
    );
    assert!(report["capture_failures"].as_array().unwrap().is_empty());
}

fn context(function: &Value, package: &str, source: &Path, features: Value) {
    let facts = &function["facts"];
    let options = &facts["build"]["frontend_options"];
    let selected = &options["rstyle.nominal_selected_unit"];
    assert_eq!(selected["package"]["name"], package);
    assert_eq!(
        selected["root_source"],
        source.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(facts["build"]["features"], features);
    assert_eq!(
        function["nominal_unit_binding"]["package_id"],
        selected["package"]["package_id"]
    );
    assert_eq!(
        function["nominal_unit_binding"]["manifest_digest"],
        selected["package"]["manifest_digest"]
    );
    assert!(
        selected["package"]["manifest_bytes"]
            .as_str()
            .unwrap()
            .contains(package)
    );
}

fn replay_units(report: &Value) {
    for unit in report["observed_units"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let mut selected = report.clone();
        selected["functions"] = Value::Array(
            report["functions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|function| function["unit_evidence"]["report_unit"] == unit.0)
                .cloned()
                .collect(),
        );
        assert!(!selected["functions"].as_array().unwrap().is_empty());
        check_aggregate_documents(&selected);
    }
}

fn contexts(captured: &Value, directory: &Path) {
    context(
        named(captured, "local"),
        "clone_fixture",
        &directory.join("src/lib.rs"),
        serde_json::json!([]),
    );
    context(
        named(captured, "selected_value"),
        "scalar_dependency",
        &directory.join("outside/src/lib.rs"),
        serde_json::json!(["selected"]),
    );
}

#[test]
fn explicit_dependency_capture_keeps_features_and_external_calls_separate() {
    let directory = workspace();
    let (output, baseline) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    inventory(&baseline, 0, 1);
    assert_eq!(baseline["functions"].as_array().unwrap().len(), 2);

    let (output, captured) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--capture-dependency",
            "scalar_dependency",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    inventory(&captured, 1, 2);
    assert_eq!(captured["functions"].as_array().unwrap().len(), 3);
    assert_eq!(named(&captured, "selected_value")["outcome"], "admitted");
    assert_eq!(named(&captured, "local")["outcome"], "admitted");
    assert_eq!(named(&captured, "caller")["outcome"], "excluded");
    assert_eq!(named(&baseline, "caller")["outcome"], "excluded");
    contexts(&captured, directory.path());
    replay_units(&captured);
}

#[test]
fn unknown_dependency_selection_cannot_publish_complete_capture() {
    let directory = workspace();
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--capture-dependency",
            "missing_dependency",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["assessment_complete"], false);
    assert!(report["observed_units"].as_array().unwrap().is_empty());
    assert!(
        report["capture_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| {
                failure
                    .as_str()
                    .is_some_and(|message| message.contains("missing_dependency"))
            })
    );
}
