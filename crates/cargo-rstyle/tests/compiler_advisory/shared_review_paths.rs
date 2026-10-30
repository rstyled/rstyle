use super::propagation::checked;
use super::*;

const SHARED: &str = "pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"shared\").ok();}";

fn options(raw: bool) -> Vec<&'static str> {
    let mut options = vec!["--compiler-plans"];
    if raw {
        options.push("--compiler-advisory-only");
    }
    options
}

fn source(directory: &Path, path: &str) {
    std::fs::write(
        directory.join("src/lib.rs"),
        format!(
            "#[path={path:?}] mod shared; pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){{shared::leaf(a,b,c,d,e);}}"
        ),
    )
    .unwrap();
}

fn capture_mode(directory: &Path, raw: bool, extra: &[&str]) -> (Output, Value) {
    let mut args = options(raw);
    args.extend_from_slice(extra);
    super::propagation::capture(directory, &args)
}

fn replay(directory: &Path, original: &Value, raw: bool, extra: &[&str]) -> (Output, Value) {
    let mut args = options(raw);
    args.extend_from_slice(extra);
    super::reviews::replay(directory, original, &args)
}

fn retained(report: &Value, original: &Value) {
    let advisory = &report["compiler_advisory"];
    let previous = &original["compiler_advisory"];
    assert_eq!(
        advisory["units"][0]["evaluation"],
        previous["units"][0]["evaluation"]
    );
    assert_eq!(advisory["planning"], previous["planning"]);
    hidden(report);
}

fn hidden(report: &Value) {
    assert!(
        report["compiler_advisory"]["diagnostic_review"]["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["state"] == "hidden_exact" })
    );
}

fn export_and_dismiss(directory: &Path, original: &Value, raw: bool) {
    let diagnostics = original["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
        .as_array()
        .unwrap();
    let shared = diagnostics
        .iter()
        .position(|row| {
            row["rule_id"] == "fn_params" && row["path"].as_str().unwrap().ends_with("shared.rs")
        })
        .unwrap()
        .to_string();
    let (output, reviewed) = replay(
        directory,
        original,
        raw,
        &[
            "--write-compiler-baseline",
            "baseline.json",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            &shared,
            "--review-reason",
            "Reviewed original shared parameter boundary",
        ],
    );
    checked(&output, 0);
    retained(&reviewed, original);
    canonical_artifacts(directory);
}

fn canonical_artifacts(directory: &Path) {
    let baseline: Value =
        serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap();
    assert_eq!(baseline["entries"].as_array().unwrap().len(), 2);
    assert!(
        baseline["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["owner"]["locator"]["source"] == "shared.rs" })
    );
    let reviews: Value =
        serde_json::from_slice(&std::fs::read(directory.join("reviews.json")).unwrap()).unwrap();
    assert_eq!(
        reviews["compiler_diagnostic_entries"][0]["finding"]["owner"]["locator"]["source"],
        "shared.rs"
    );
}

const LOOKUP: &[&str] = &[
    "--compiler-baseline",
    "baseline.json",
    "--fail-on-new-compiler",
    "--review-file",
    "reviews.json",
];

fn no_new_findings(report: &Value) {
    assert_eq!(
        report["compiler_advisory"]["diagnostic_baseline"]["new_findings"],
        0
    );
    assert_eq!(
        report["compiler_advisory"]["diagnostic_baseline"]["coverage"],
        "complete"
    );
}

fn positive_mode(raw: bool) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("shared.rs"), SHARED).unwrap();
    source(directory.path(), "../shared.rs");
    let (output, original) = capture_mode(directory.path(), raw, &[]);
    checked(&output, 0);
    let unit = &original["compiler_advisory"]["units"][0];
    assert_eq!(unit["observations"].as_array().unwrap().len(), 2);
    assert_eq!(unit["evaluation"]["summary"]["effect_counts"]["io"], 2);
    assert_eq!(
        original["compiler_advisory"]["planning"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    export_and_dismiss(directory.path(), &original, raw);
    source(directory.path(), ".././shared.rs");
    let (output, equivalent) = capture_mode(directory.path(), raw, LOOKUP);
    checked(&output, 0);
    no_new_findings(&equivalent);
    hidden(&equivalent);
    for path in ["src/lib.rs", "shared.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, replayed) = replay(directory.path(), &equivalent, raw, LOOKUP);
    checked(&output, 0);
    no_new_findings(&replayed);
    retained(&replayed, &equivalent);
}

#[test]
fn in_package_parent_relative_shared_reviews_and_baselines_survive_equivalent_paths_and_replay() {
    for raw in [false, true] {
        positive_mode(raw);
    }
}

fn outside_mode(raw: bool) {
    let directory = clone_workspace();
    let external = TempDir::new().unwrap();
    let path = external.path().join("shared.rs");
    std::fs::write(&path, SHARED).unwrap();
    source(directory.path(), path.to_str().unwrap());
    let (output, report) = capture_mode(directory.path(), raw, &["--review-file", "reviews.json"]);
    checked(&output, 2);
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["complete"], false);
    let unit = &advisory["units"][0];
    assert_eq!(unit["expected_functions"], 2);
    assert_eq!(unit["observations"].as_array().unwrap().len(), 1);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 1);
    assert!(!directory.path().join("reviews.json").exists());
    let mut args = vec![
        "clones",
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--replay-run",
        report["run_directory"].as_str().unwrap(),
        "--write-compiler-baseline",
        "baseline.json",
    ];
    args.extend(options(raw));
    let rejected = rstyle(directory.path(), &args);
    checked(&rejected, 2);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Incomplete analysis"));
    assert!(!directory.path().join("baseline.json").exists());
}

#[test]
fn external_shared_source_cannot_become_a_package_review_or_complete_baseline() {
    for raw in [false, true] {
        outside_mode(raw);
    }
}
