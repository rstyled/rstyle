use super::*;

const SOURCE: &str = r#"
pub fn straight() -> u128 { 1 }
pub fn nested(a: bool, b: bool) { if a { if b {} } }
pub fn flat(a: bool, b: bool) { if a {} if b {} }
pub fn recursion(x: u128) -> u128 { if x == 0 { return 0; } recursion(x - 1) }
"#;
const TOP: &[&str] = &["--compiler-complexity-top", "10"];

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn ranking(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["complexity_ranking"]
}

fn names(ranking: &Value) -> Vec<&str> {
    ranking["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect()
}

fn unchanged(original: &Value, ranked: &Value) {
    for field in ["matching", "accepted_pairs", "functions"] {
        assert_eq!(original[field], ranked[field], "{field}");
    }
    let before = &original["compiler_advisory"]["units"][0];
    let after = &ranked["compiler_advisory"]["units"][0];
    for field in ["evaluation", "observations", "patterns"] {
        assert_eq!(before[field], after[field], "{field}");
    }
    assert_eq!(
        original["compiler_advisory"]["planning"],
        ranked["compiler_advisory"]["planning"]
    );
    assert_eq!(
        original["compiler_advisory"]["findings"],
        ranked["compiler_advisory"]["findings"]
    );
}

#[test]
fn complexity_top_requires_advisory_and_a_positive_limit() {
    let directory = clone_workspace();
    for args in [
        vec!["clones", "--compiler-complexity-top", "1"],
        vec![
            "clones",
            "--compiler-advisory",
            "--compiler-complexity-top",
            "0",
        ],
        vec![
            "clones",
            "--compiler-advisory",
            "--compiler-complexity-top",
            "-1",
        ],
        vec![
            "clones",
            "--compiler-advisory",
            "--compiler-complexity-top",
            "no",
        ],
    ] {
        let output = rstyle(directory.path(), &args);
        checked(&output, 2);
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn complexity_ranking_includes_clean_scores_without_creating_findings() {
    let (directory, output, original) =
        capture(SOURCE, &["--compiler-patterns", "--compiler-plans"]);
    checked(&output, 0);
    assert!(
        original["compiler_advisory"]["units"][0]
            .get("complexity_ranking")
            .is_none()
    );
    let (output, ranked) = reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-patterns",
            "--compiler-plans",
            "--compiler-complexity-top",
            "10",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    unchanged(&original, &ranked);
    let ranking = ranking(&ranked);
    assert_eq!(ranking["schema"], "rstyle.compiler-complexity-ranking/1");
    assert_eq!(ranking["requested_limit"], 10);
    assert_eq!(ranking["known_functions"], 4);
    assert_eq!(ranking["measured_functions"], 4);
    assert_eq!(ranking["unavailable_measurements"], 0);
    assert_eq!(ranking["unavailable_owners"], 0);
    assert_eq!(ranking["input_complete"], true);
    assert_eq!(names(ranking), ["nested", "flat", "recursion", "straight"]);
    for (row, owner) in ranking["functions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(ranking["owners"].as_array().unwrap())
    {
        let fact = ranked["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|fact| fact["key"] == *owner)
            .unwrap();
        assert_eq!(row["name"], fact["facts"]["name"]);
        assert_eq!(
            row["complexity"],
            fact["facts"]["build"]["frontend_options"][rstyle_core::advisory::KEY]["facts"]["complexity"]
        );
    }
}

#[test]
fn complexity_ranking_limits_rows_and_uses_source_order_for_score_ties() {
    let source = "pub fn zebra(a:bool){if a {}}\npub fn alpha(a:bool){if a {}}\npub fn clean(){}";
    let (directory, output, original) = capture(source, TOP);
    checked(&output, 0);
    assert_eq!(names(ranking(&original)), ["zebra", "alpha", "clean"]);
    let (output, limited) = reviews::replay(
        directory.path(),
        &original,
        &["--compiler-complexity-top", "1"],
    );
    checked(&output, 0);
    let ranking = ranking(&limited);
    assert_eq!(names(ranking), ["zebra"]);
    assert_eq!(ranking["owners"].as_array().unwrap().len(), 1);
    assert_eq!(ranking["known_functions"], 3);
    assert_eq!(ranking["measured_functions"], 3);
    assert_eq!(ranking["requested_limit"], 1);
    unchanged(&original, &limited);
}

#[test]
fn complexity_ranking_replays_advisory_only_without_sources_or_compiler_tools() {
    let (directory, output, original) = capture(
        SOURCE,
        &[
            "--compiler-advisory-only",
            "--compiler-complexity-top",
            "10",
        ],
    );
    checked(&output, 0);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--compiler-complexity-top",
            "10",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--fail-on-findings",
        ])
        .env("PATH", directory.path().join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 0);
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(ranking(&replayed), ranking(&original));
    assert_eq!(replayed["matching_execution"], "not_requested");
    assert_eq!(replayed["strict_assessment_execution"], "not_requested");
}

#[test]
fn complexity_ranking_applies_policy_exclusion_but_keeps_changed_file_selection_global() {
    let (directory, output, original) = capture(SOURCE, TOP);
    checked(&output, 0);
    std::fs::write(
        directory.path().join("selection.json"),
        r#"{"schema":"rstyle.changed-files/1","paths":[]}"#,
    )
    .unwrap();
    let (output, selected) = reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-complexity-top",
            "10",
            "--changed-files",
            "selection.json",
        ],
    );
    checked(&output, 0);
    assert_eq!(ranking(&selected), ranking(&original));
    std::fs::write(directory.path().join("policy.toml"), "exclude=['src/**']\n").unwrap();
    let (output, excluded) = reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-complexity-top",
            "10",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 0);
    assert_eq!(ranking(&excluded)["known_functions"], 0);
    assert_eq!(ranking(&excluded)["measured_functions"], 0);
    assert_eq!(ranking(&excluded)["functions"], serde_json::json!([]));
    assert_eq!(ranking(&excluded)["owners"], serde_json::json!([]));
    assert_eq!(ranking(&excluded)["input_complete"], true);
}

#[test]
fn complexity_ranking_does_not_hide_missing_compiler_owners() {
    let source = "macro_rules! make {()=>{pub fn generated(){}}} make!(); pub fn original(){}";
    let (_directory, output, report) = capture(source, &["--compiler-complexity-top", "1"]);
    checked(&output, 2);
    assert_eq!(ranking(&report)["known_functions"], 1);
    assert_eq!(ranking(&report)["measured_functions"], 1);
    assert_eq!(ranking(&report)["unavailable_owners"], 1);
    assert_eq!(ranking(&report)["input_complete"], false);
    assert_eq!(names(ranking(&report)), ["original"]);
    let directory = clone_workspace();
    let (output, original) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    checked(&output, 0);
    let (output, missing) = reviews::replay(directory.path(), &original, TOP);
    checked(&output, 2);
    assert_eq!(ranking(&missing)["known_functions"], 0);
    assert_eq!(ranking(&missing)["unavailable_owners"], 4);
    assert_eq!(ranking(&missing)["input_complete"], false);
    assert_eq!(ranking(&missing)["functions"], serde_json::json!([]));
}

#[test]
fn complexity_ranking_limits_each_compilation_context_independently() {
    let source = "pub fn common(a:bool){if a {}} #[cfg(test)] pub fn test_only(a:bool,b:bool){if a {if b {}}}";
    let (_directory, _output, report) =
        capture(source, &["--all-targets", "--compiler-complexity-top", "1"]);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut top_names = Vec::new();
    for unit in units {
        let ranking = &unit["complexity_ranking"];
        assert_eq!(ranking["requested_limit"], 1);
        assert_eq!(ranking["functions"].as_array().unwrap().len(), 1);
        assert_eq!(ranking["owners"].as_array().unwrap().len(), 1);
        top_names.push(names(ranking)[0]);
    }
    top_names.sort_unstable();
    assert_eq!(top_names, ["common", "test_only"]);
}

fn missing_scores(wire: &Value, metrics: &str) -> Value {
    let mut wire = wire.clone();
    for fact in wire["facts"].as_array_mut().unwrap() {
        let observation = &mut fact["build"]["frontend_options"][rstyle_core::advisory::KEY];
        observation["metrics"] = metrics.into();
        if metrics != rstyle_core::advisory::METRICS {
            observation["facts"]
                .as_object_mut()
                .unwrap()
                .remove("complexity");
        } else if observation["facts"]["name"] == "straight" {
            observation["facts"]["complexity"] = Value::Null;
        }
    }
    wire
}

fn assert_missing_scores(report: &Value, metrics: &str) {
    assert_eq!(report["compiler_advisory"]["metrics"], metrics);
    let ranking = ranking(report);
    let modern = metrics == rstyle_core::advisory::METRICS;
    assert_eq!(ranking["known_functions"], 2);
    assert_eq!(ranking["measured_functions"], usize::from(modern));
    assert_eq!(
        ranking["unavailable_measurements"],
        if modern { 1 } else { 2 }
    );
    assert_eq!(ranking["unavailable_owners"], 0);
    assert_eq!(ranking["input_complete"], false);
    assert_eq!(ranking["functions"].as_array().unwrap().len(), 1);
    assert_eq!(names(ranking), [if modern { "nested" } else { "straight" }]);
    if modern {
        assert_eq!(ranking["functions"][0]["complexity"]["cognitive"], 3);
    } else {
        assert!(ranking["functions"][0]["complexity"].is_null());
    }
}

fn missing_score_text(directory: &Path, original: &Value) {
    let output = rstyle(
        directory,
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--compiler-complexity-top",
            "2",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&output, 2);
    let text = stdout(&output);
    assert!(
        text.contains("Compiler complexity ranking for unit"),
        "{text}"
    );
    assert!(text.contains("CC  COG  LOCATION  FUNCTION"), "{text}");
    assert!(text.contains("input complete=false"), "{text}");
    assert!(text.contains("   ?    ?"), "{text}");
    assert!(text.contains("straight"), "{text}");
}

#[test]
fn historical_and_modern_missing_scores_remain_unknown_in_replayed_rankings() {
    let source = "pub fn straight(){} pub fn nested(a:bool,b:bool){if a {if b {}}}";
    let (directory, output, original) = capture(source, &["--compiler-advisory-only"]);
    checked(&output, 0);
    let ledger = super::only::ledger_path(&original);
    let bundle = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let wire: Value = serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap();
    for metrics in [
        rstyle_core::advisory::LEGACY_METRICS,
        rstyle_core::advisory::V2_METRICS,
        rstyle_core::advisory::V3_METRICS,
        rstyle_core::advisory::V4_METRICS,
        rstyle_core::advisory::V5_METRICS,
        rstyle_core::advisory::METRICS,
    ] {
        let changed = missing_scores(&wire, metrics);
        super::only::reseal_raw(&original, &ledger, &changed);
        let (output, report) = reviews::replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--compiler-complexity-top",
                "1",
                "--fail-on-findings",
            ],
        );
        checked(&output, 2);
        assert_missing_scores(&report, metrics);
        missing_score_text(directory.path(), &original);
    }
    super::only::reseal_raw(&original, &ledger, &wire);
    let (output, restored) = reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-only",
            "--compiler-complexity-top",
            "1",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    assert_eq!(ranking(&restored)["input_complete"], true);
}
