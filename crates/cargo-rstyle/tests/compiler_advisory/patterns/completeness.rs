use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";

#[test]
fn structural_reviews_do_not_require_completed_effect_callees() {
    let directory = propagation::workspace("pub fn leaf() {} ");
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!("{HELPERS} pub fn caller() {{ renamed::leaf(); }}"),
    )
    .unwrap();
    let (output, original) = clones(
        directory.path(),
        &[
            "-p",
            "caller",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
        ],
    );
    assert_independent_completion(&output, &original);
    for path in [
        "caller/src/lib.rs",
        "caller/Cargo.toml",
        "leaf/src/lib.rs",
        "Cargo.toml",
    ] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, saved) = replay(
        directory.path(),
        &original,
        &["--compiler-advisory-only", "--review-file", "reviews.json"],
    );
    assert_independent_completion(&output, &saved);
    assert_eq!(original["compiler_advisory"], saved["compiler_advisory"]);
    assert_eq!(original["matching_execution"], "not_requested");
    assert_eq!(original["strict_assessment_execution"], "not_requested");
}

fn assert_independent_completion(output: &Output, report: &Value) {
    assert_eq!(output.status.code(), Some(2), "{}", stdout(output));
    let advisory = &report["compiler_advisory"];
    let unit = &advisory["units"][0];
    assert_eq!(unit["unavailable"], serde_json::json!([]));
    assert_eq!(
        unit["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    assert_eq!(unit["patterns"]["input_complete"], true);
    assert_eq!(unit["patterns"]["effect_evidence"]["input_complete"], false);
    assert!(candidate_at(&unit["patterns"]["evaluation"], "first").is_some());
    assert_eq!(advisory["pattern_review"]["coverage"], "complete");
    assert_eq!(advisory["diagnostic_review"]["coverage"], "incomplete");
    assert_eq!(advisory["complete"], false);
}

#[test]
fn missing_structural_owners_still_make_pattern_reviews_incomplete() {
    let source = format!(
        "{HELPERS} macro_rules! generated {{ () => {{ pub fn extra() {{}} }}; }} generated!();"
    );
    let (_directory, output, report) = capture(
        &source,
        &["--compiler-patterns", "--review-file", "reviews.json"],
    );
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(proposals(&report)["input_complete"], false);
    assert_eq!(
        report["compiler_advisory"]["pattern_review"]["coverage"],
        "incomplete"
    );
}
