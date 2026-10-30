use super::propagation::{checked, workspace};
use super::*;

const WIDE: &str = "pub fn wide(a:u32,b:u32)->u32 {a+b}\n";
const COMPARE: &[&str] = &["--baseline", "baseline.json", "--fail-on-new"];

fn fixture() -> TempDir {
    let directory = workspace(WIDE);
    std::fs::write(directory.path().join("caller/src/lib.rs"), WIDE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> Output {
    let flags = [
        &["--style-engine", "compiler", "--format", "json"][..],
        extra,
    ]
    .concat();
    rstyle(directory, &flags)
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn baseline(report: &Value) -> &Value {
    &report["compiler_advisory"]["diagnostic_baseline"]
}

fn export(directory: &Path) -> Value {
    let output = capture(directory, &["--write-baseline", "baseline.json"]);
    checked(&output, 1);
    let report = report(&output);
    assert_eq!(report["summary"]["violations"], 2);
    assert_eq!(baseline(&report)["coverage"], "complete");
    let saved: Value =
        serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap();
    assert_eq!(saved["schema"], "rstyle.compiler-baseline/1");
    assert_eq!(saved["entries"].as_array().unwrap().len(), 2);
    assert_eq!(saved["contexts"].as_array().unwrap().len(), 2);
    report
}

fn replay(directory: &Path, original: &Value, format: &str, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args(["--style-engine", "compiler", "--format", format])
        .args(["--replay-run", original["run_directory"].as_str().unwrap()])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn unchanged(original: &Value, compared: &Value) {
    for field in [
        "diagnostics",
        "diagnostic_units",
        "summary",
        "functions",
        "function_units",
        "fix_groups",
    ] {
        assert_eq!(original[field], compared[field], "{field}");
    }
    let before = original["compiler_advisory"]["units"].as_array().unwrap();
    let after = compared["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after) {
        assert_eq!(a["unit"], b["unit"]);
        assert_eq!(a["evaluation"], b["evaluation"]);
    }
}

#[test]
fn root_style_baseline_replays_without_sources_and_changes_only_the_requested_gate() {
    let directory = fixture();
    let original = export(directory.path());
    for package in ["leaf", "caller"] {
        std::fs::remove_dir_all(directory.path().join(package).join("src")).unwrap();
        std::fs::remove_file(directory.path().join(package).join("Cargo.toml")).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = replay(directory.path(), &original, "json", COMPARE);
    checked(&output, 0);
    let compared = report(&output);
    unchanged(&original, &compared);
    assert_eq!(baseline(&compared)["existing_findings"], 2);
    assert_eq!(baseline(&compared)["new_findings"], 0);
    assert_eq!(baseline(&compared)["scope_changed"], false);
    assert_eq!(compared["summary_complete"], true);
    let output = replay(
        directory.path(),
        &original,
        "json",
        &["--baseline", "baseline.json"],
    );
    checked(&output, 1);
    unchanged(&original, &report(&output));
    let output = replay(directory.path(), &original, "text", COMPARE);
    checked(&output, 0);
    assert!(stdout(&output).contains("Compiler diagnostic baseline: Complete coverage; 2 existing, 0 new selected visible findings"));
}

fn select(directory: &Path, path: &str) {
    std::fs::write(
        directory.join("changes.json"),
        serde_json::to_vec(&serde_json::json!({"schema":"rstyle.changed-files/1", "paths":[path]}))
            .unwrap(),
    )
    .unwrap();
}

#[test]
fn root_style_baseline_detects_changed_values_and_selects_only_changed_file_findings() {
    let directory = fixture();
    export(directory.path());
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn wide(a:u32,b:u32,c:u32)->u32 {a+b+c}\n",
    )
    .unwrap();
    let output = capture(directory.path(), COMPARE);
    checked(&output, 1);
    let changed = report(&output);
    assert_eq!(baseline(&changed)["existing_findings"], 1);
    assert_eq!(baseline(&changed)["new_findings"], 1);
    assert_eq!(baseline(&changed)["scope_changed"], false);
    assert!(
        changed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["actual"] == 3)
    );
    select(directory.path(), "leaf/src/lib.rs");
    let flags = [COMPARE, &["--changed-files", "changes.json"]].concat();
    let output = replay(directory.path(), &changed, "json", &flags);
    checked(&output, 0);
    let selected = report(&output);
    assert_eq!(baseline(&selected)["existing_findings"], 1);
    assert_eq!(baseline(&selected)["new_findings"], 0);
    assert_eq!(selected["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(selected["diagnostics"][0]["path"], "leaf/src/lib.rs");
    assert_eq!(selected["summary_complete"], true);
    select(directory.path(), "unobserved.rs");
    let output = replay(directory.path(), &changed, "json", &flags);
    checked(&output, 2);
    let missing = report(&output);
    assert_eq!(missing["diagnostics"], serde_json::json!([]));
    assert_eq!(missing["summary_complete"], false);
    assert_eq!(baseline(&missing)["new_findings"], 0);
}

#[test]
fn root_style_baseline_keeps_package_and_policy_drift_failing_without_new_findings() {
    let directory = fixture();
    let original = export(directory.path());
    let output = capture(
        directory.path(),
        &["-p", "leaf", "--baseline", "baseline.json", "--fail-on-new"],
    );
    checked(&output, 1);
    let selected = report(&output);
    assert_eq!(baseline(&selected)["new_findings"], 0);
    assert_eq!(baseline(&selected)["existing_findings"], 1);
    assert_eq!(baseline(&selected)["scope_changed"], true);
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=2\n",
    )
    .unwrap();
    let output = replay(directory.path(), &original, "json", COMPARE);
    checked(&output, 1);
    let report = report(&output);
    assert_eq!(report["diagnostics"], serde_json::json!([]));
    assert_eq!(report["summary"]["violations"], 0);
    assert_eq!(report["summary_complete"], true);
    assert_eq!(baseline(&report)["scope_changed"], true);
}

#[test]
fn root_style_baseline_preserves_a_complete_empty_inventory() {
    let directory = fixture();
    for package in ["leaf", "caller"] {
        std::fs::write(
            directory.path().join(package).join("src/lib.rs"),
            "pub fn clean() {}\n",
        )
        .unwrap();
    }
    let output = capture(directory.path(), &["--write-baseline", "baseline.json"]);
    checked(&output, 0);
    let original = report(&output);
    assert_eq!(original["summary"]["functions"], 2);
    assert_eq!(original["summary_complete"], true);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("baseline.json")).unwrap())
            .unwrap();
    assert_eq!(saved["entries"], serde_json::json!([]));
    assert_eq!(saved["contexts"].as_array().unwrap().len(), 2);
    let output = replay(directory.path(), &original, "json", COMPARE);
    checked(&output, 0);
    let compared = report(&output);
    unchanged(&original, &compared);
    assert_eq!(baseline(&compared)["coverage"], "complete");
    assert_eq!(baseline(&compared)["existing_findings"], 0);
    assert_eq!(baseline(&compared)["new_findings"], 0);
    assert_eq!(baseline(&compared)["scope_changed"], false);
}

#[test]
fn root_style_baseline_rejects_invalid_inputs_and_never_overwrites_an_inventory() {
    let directory = fixture();
    let original = export(directory.path());
    let path = directory.path().join("baseline.json");
    let bytes = std::fs::read(&path).unwrap();
    let output = replay(
        directory.path(),
        &original,
        "json",
        &["--write-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    for invalid in ["not JSON", "{\"schema\":\"unsupported\"}"] {
        std::fs::write(directory.path().join("bad.json"), invalid).unwrap();
        let output = replay(
            directory.path(),
            &original,
            "json",
            &["--baseline", "bad.json", "--fail-on-new"],
        );
        checked(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("bad.json"));
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn root_style_baseline_cannot_publish_incomplete_dependency_effect_inputs() {
    let directory = workspace("pub fn leaf() {let _=std::fs::read(\"/unused\");}\n");
    let output = capture(
        directory.path(),
        &["-p", "caller", "--write-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Incomplete"));
}
