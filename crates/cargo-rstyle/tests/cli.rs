//! End-to-end tests: real `cargo check` through the real rustc driver.

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// `cargo test -p cargo-rstyle` does not build sibling binaries, so build the driver once.
fn driver() -> &'static Path {
    static DRIVER: OnceLock<PathBuf> = OnceLock::new();
    DRIVER.get_or_init(|| {
        let status = Command::new("cargo")
            .args(["build", "--quiet", "-p", "rstyle-driver"])
            .current_dir(workspace_root())
            .status()
            .unwrap();
        assert!(status.success(), "building rstyle-driver failed");
        cargo_bin("cargo-rstyle").with_file_name("rstyle-driver")
    })
}

/// Copies a fixture into a fresh temp dir so runs never share build state.
fn fixture(name: &str) -> TempDir {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&source, dir.path());
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn rstyle(dir: &Path, args: &[&str]) -> Output {
    Command::new(cargo_bin("cargo-rstyle"))
        .args(args)
        .current_dir(dir)
        .env("RSTYLE_DRIVER", driver())
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn json(dir: &Path) -> Value {
    let output = rstyle(dir, &["--format", "json"]);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "bad json ({e}): {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn diagnostics_for<'a>(report: &'a Value, function: &str) -> Vec<&'a Value> {
    let all = report["diagnostics"].as_array().unwrap();
    all.iter()
        .filter(|d| {
            d["message"]
                .as_str()
                .unwrap()
                .contains(&format!("`{function}`"))
        })
        .collect()
}

fn rules_for(report: &Value, function: &str) -> Vec<String> {
    let found = diagnostics_for(report, function);
    found
        .iter()
        .map(|d| d["rule_id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn clean_crate_exits_zero() {
    let dir = fixture("clean");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("0 violations"));
    let report = json(dir.path());
    assert!(report["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(report["summary"]["functions"], 4);
}

#[test]
fn violations_exit_one_and_match_golden_output() {
    let dir = fixture("violations");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    insta::assert_snapshot!(stdout(&output));
}

#[test]
fn json_output_reports_resolved_mutations_and_effects() {
    let dir = fixture("violations");
    let report = json(dir.path());
    // 4 mutations over 4 statements is also a density of 1000, above the default 500.
    assert_eq!(
        rules_for(&report, "Account::churn"),
        ["mutation_per_target", "side_effect_density"]
    );
    let churn = diagnostics_for(&report, "Account::churn")[0];
    // `self.bump()` is an autoref'd `&mut self` call: seen by HIR, invisible to name heuristics.
    assert_eq!(churn["details"][0]["label"], "self");
    assert_eq!(churn["details"][0]["count"], 4);
    assert!(rules_for(&report, "spread").contains(&"mutated_targets".to_string()));
    assert!(rules_for(&report, "too_many").contains(&"fn_params".to_string()));
    assert!(rules_for(&report, "very_long").contains(&"fn_length".to_string()));
    assert!(rules_for(&report, "Account::peek").is_empty());
    assert!(rules_for(&report, "pure").is_empty());
}

#[test]
fn effects_are_classified_from_resolved_paths() {
    let dir = fixture("violations");
    let report = json(dir.path());
    let labels = |function: &str| -> Vec<String> {
        diagnostics_for(&report, function)
            .iter()
            .filter(|d| d["rule_id"] == "side_effect_density")
            .flat_map(|d| d["details"].as_array().unwrap().clone())
            .map(|d| d["label"].as_str().unwrap().to_string())
            .collect()
    };
    let noisy = labels("noisy");
    assert!(
        noisy.contains(&"io: std::fs::write".to_string()),
        "{noisy:?}"
    );
    assert!(
        noisy.iter().any(|l| l.starts_with("io: std::io::")),
        "{noisy:?}"
    );
    let dirty = labels("dirty");
    assert!(
        dirty.contains(&"unsafe: unsafe block".to_string()),
        "{dirty:?}"
    );
    assert!(
        dirty.iter().any(|l| l.starts_with("global: static mut")),
        "{dirty:?}"
    );
    assert!(
        dirty.contains(&"unknown: <fn pointer>".to_string()),
        "{dirty:?}"
    );
}

#[test]
fn suppressions_apply_only_with_a_reason() {
    let report = json(fixture("violations").path());
    assert!(rules_for(&report, "allowed").is_empty());
    let missing = rules_for(&report, "missing_reason");
    assert!(missing.contains(&"fn_params".to_string()), "{missing:?}");
    assert!(
        missing.contains(&"invalid_suppression".to_string()),
        "{missing:?}"
    );
}

#[test]
fn derived_code_is_not_analyzed() {
    let report = json(fixture("violations").path());
    let summary = report["summary"]["functions"].as_u64().unwrap();
    // 11 hand-written fns; derive(Debug, Clone, PartialEq) would add more if leaked.
    assert_eq!(summary, 11);
}

#[test]
fn two_runs_are_byte_identical() {
    let dir = fixture("violations");
    let first = rstyle(dir.path(), &["--format", "json"]);
    let second = rstyle(dir.path(), &["--format", "json"]);
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.stdout.is_empty());
}

#[test]
fn config_change_reevaluates_without_recompiling() {
    let dir = fixture("violations");
    let before = json(dir.path());
    let facts = dir.path().join("target/rstyle/facts/violations-lib.json");
    let modified = std::fs::metadata(&facts).unwrap().modified().unwrap();
    std::fs::write(
        dir.path().join("rstyle.toml"),
        "[fn_params]\nmax_params = 10\n",
    )
    .unwrap();
    let after = json(dir.path());
    assert!(rules_for(&before, "too_many").contains(&"fn_params".to_string()));
    assert!(!rules_for(&after, "too_many").contains(&"fn_params".to_string()));
    assert_eq!(
        std::fs::metadata(&facts).unwrap().modified().unwrap(),
        modified
    );
}

#[test]
fn compile_errors_exit_two() {
    let dir = fixture("broken");
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cargo check failed"));
}

#[test]
fn bad_config_exits_two() {
    let dir = fixture("clean");
    std::fs::write(dir.path().join("rstyle.toml"), "typo = 1\n").unwrap();
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("rstyle.toml"));
}

#[test]
fn this_workspace_passes_its_own_rules() {
    let output = rstyle(&workspace_root(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_package_exits_two() {
    let dir = fixture("clean");
    let output = rstyle(dir.path(), &["-p", "nope"]);
    assert_eq!(output.status.code(), Some(2));
}
