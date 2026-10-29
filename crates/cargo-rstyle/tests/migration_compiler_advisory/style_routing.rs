use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn wide(a:u32,b:u32)->u32 { a+b }\n// rstyle-allow(fn_params): explicit API boundary\npub fn boundary(a:u32,b:u32)->u32 { a+b }\n";
const POLICY: &str = "[fn_params]\nmax_params=1\n";

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("rstyle.toml"), POLICY).unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--style-engine", "compiler", "--format", "json"];
    args.extend_from_slice(extra);
    let output = rstyle(directory, &args);
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output, report)
}

fn diagnostics(report: &Value) -> Vec<&Value> {
    report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["evaluation"]["diagnostics"].as_array().unwrap())
        .collect()
}

#[test]
fn root_style_route_uses_workspace_config_suppressions_and_finding_exit() {
    let directory = fixture(SOURCE);
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 1);
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(diagnostics(&report).len(), 1);
    assert_eq!(diagnostics(&report)[0]["rule_id"], "fn_params");
    assert_eq!(diagnostics(&report)[0]["actual"], 2);
    assert_eq!(diagnostics(&report)[0]["limit"], 1);
    assert_eq!(
        report["compiler_advisory"]["planning"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let scope = &report["compiler_advisory"]["units"][0]["diagnostic_scope"];
    assert_eq!(scope["request"]["source_scope"], "authored");
    assert_eq!(scope["request"]["rules"], serde_json::json!([]));
    let legacy = rstyle(directory.path(), &["--format", "json"]);
    checked(&legacy, 1);
    let legacy: Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(
        legacy["diagnostics"],
        serde_json::json!(diagnostics(&report))
    );
}

#[test]
fn root_style_route_honors_explicit_config_and_ignores_pattern_ledger() {
    let directory = fixture(SOURCE);
    std::fs::write(
        directory.path().join("other.toml"),
        "[fn_params]\nmax_params=2\n",
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &["--config", "other.toml", "--ignore-file", "absent.toml"],
    );
    checked(&output, 0);
    assert!(diagnostics(&report).is_empty());
    assert_eq!(
        report["compiler_advisory"]["effective_config"]["fn_params"]["max_params"],
        2
    );
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn root_style_route_retains_invalid_suppression_and_compile_failure_exits() {
    let directory = fixture("// rstyle-allow(no_such_rule): control\npub fn clean() {}\n");
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 1);
    assert_eq!(diagnostics(&report)[0]["rule_id"], "invalid_suppression");
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn broken( {}\n").unwrap();
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["compiler_advisory"]["complete"], false);
}

#[test]
fn root_style_route_preserves_test_compilation_contexts() {
    let directory =
        fixture("pub fn clean() {}\n#[cfg(test)] pub fn test_only(a:u32,b:u32)->u32 { a+b }\n");
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let (output, report) = capture(directory.path(), &["--all-targets"]);
    checked(&output, 1);
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(diagnostics(&report).len(), 1);
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn root_style_route_resolves_manifest_config_and_package_from_other_directory() {
    let directory = fixture(SOURCE);
    let elsewhere = tempfile::tempdir().unwrap();
    let manifest = directory.path().join("Cargo.toml");
    let (output, report) = capture(
        elsewhere.path(),
        &[
            "--manifest-path",
            manifest.to_str().unwrap(),
            "--package",
            "clone_fixture",
        ],
    );
    checked(&output, 1);
    assert_eq!(diagnostics(&report)[0]["limit"], 1);
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn style_backend_option_rejects_subcommand_misrouting() {
    let directory = fixture("pub fn clean() {}\n");
    let output = rstyle(
        directory.path(),
        &["--style-engine", "compiler", "patterns"],
    );
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("default style command only"));
}
