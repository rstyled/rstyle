use super::propagation::checked;
use super::*;

const SOURCE: &str = r#"
pub fn left() { let _ = std::fs::read("/unused"); }
pub fn right() { let _ = std::fs::read("/unused"); }
// rstyle-allow(side_effect_density): reviewed IO boundary
pub fn allowed() { let _ = std::fs::read("/unused"); }
pub fn wide(a:u32,b:u32)->u32 { a+b }
"#;
const POLICY: &str = "[fn_params]\nmax_params=1\n[side_effects]\nmin_lines=0\nmin_statements=1\nmax_density_milli=1\ntransitive_depth=0\n";

fn fixture() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(directory.path().join("rstyle.toml"), POLICY).unwrap();
    directory
}

fn current(directory: &Path, flags: &[&str]) -> (Output, Value) {
    let output = rstyle(directory, &[&["--format", "json"][..], flags].concat());
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

fn replay(directory: &Path, original: &Value, flags: &[&str]) -> (Output, Value) {
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory)
            .args([
                "--format",
                "json",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .args(flags)
            .env("PATH", directory.join("no-tools")),
    )
    .unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

fn dismiss(directory: &Path, original: &Value, name: &str) -> (Output, Value) {
    let index = original["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| {
            row["message"]
                .as_str()
                .unwrap()
                .contains(&format!("`{name}`"))
        })
        .unwrap();
    let unit = original["diagnostic_units"][index].to_string();
    let diagnostic = original["diagnostic_indices"][index].to_string();
    replay(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            &unit,
            &diagnostic,
            "--review-reason",
            "Reviewed individual IO boundary",
        ],
    )
}

fn raw_diagnostics(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
}

fn default_contract(original: &Value) {
    assert_eq!(original["summary"]["violations"], 3);
    assert_eq!(
        original["compiler_advisory"]["metrics"],
        "rstyle.compiler-style-metrics/7"
    );
    assert_eq!(original["strict_assessment_execution"], "not_requested");
    assert!(
        !original["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["message"].as_str().unwrap().contains("`allowed`"))
    );
}

#[test]
fn default_style_blocks_density_and_reuses_individual_reviews_and_source_allows() {
    let directory = fixture();
    let (output, original) = current(directory.path(), &[]);
    checked(&output, 1);
    default_contract(&original);

    let (output, unreviewed) = current(directory.path(), &["--review-file", "reviews.json"]);
    checked(&output, 1);
    assert_eq!(original["diagnostics"], unreviewed["diagnostics"]);
    assert!(!directory.path().join("reviews.json").exists());
    let (output, reviewed) = dismiss(directory.path(), &original, "left");
    checked(&output, 1);
    assert_eq!(reviewed["summary"]["violations"], 2);
    assert_eq!(raw_diagnostics(&original), raw_diagnostics(&reviewed));
    assert!(
        reviewed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["rule_id"] == "side_effect_density")
    );
    assert!(
        reviewed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["rule_id"] == "fn_params")
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("src/lib.rs")).unwrap(),
        SOURCE
    );
    let (output, applied) = current(directory.path(), &["--review-file", "reviews.json"]);
    checked(&output, 1);
    assert_eq!(reviewed["diagnostics"], applied["diagnostics"]);
    let (output, both) = dismiss(directory.path(), &original, "right");
    checked(&output, 1);
    assert_eq!(both["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(both["diagnostics"][0]["rule_id"], "fn_params");
    assert_eq!(raw_diagnostics(&original), raw_diagnostics(&both));
}

#[test]
fn default_style_reviews_replay_without_sources_and_reveal_changed_policy_findings() {
    let directory = fixture();
    let (_, original) = current(directory.path(), &[]);
    let (_, reviewed) = dismiss(directory.path(), &original, "left");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    for path in ["src", "Cargo.toml", "Cargo.lock"] {
        let path = directory.path().join(path);
        if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    let flags = ["--review-file", "reviews.json", "--review-status"];
    let (output, restored) = replay(directory.path(), &original, &flags);
    checked(&output, 1);
    assert_eq!(reviewed["diagnostics"], restored["diagnostics"]);
    assert_eq!(
        reviewed["diagnostic_indices"],
        restored["diagnostic_indices"]
    );
    assert_eq!(raw_diagnostics(&original), raw_diagnostics(&restored));
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
    std::fs::write(
        directory.path().join("rstyle.toml"),
        POLICY.replace("max_density_milli=1", "max_density_milli=2"),
    )
    .unwrap();
    let (output, changed) = replay(directory.path(), &original, &flags);
    checked(&output, 1);
    assert_eq!(changed["summary"]["violations"], 3);
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn default_style_clean_and_source_allowed_density_exit_zero() {
    let directory = fixture();
    std::fs::write(directory.path().join("src/lib.rs"),
        "// rstyle-allow(side_effect_density): reviewed IO boundary\npub fn allowed() { let _ = std::fs::read(\"/unused\"); }\npub fn clean(value:u32)->u32 { value }\n").unwrap();
    let (output, report) = current(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(report["summary"]["violations"], 0);
    assert_eq!(report["summary"]["functions"], 2);
    assert_eq!(report["summary_complete"], true);
}
