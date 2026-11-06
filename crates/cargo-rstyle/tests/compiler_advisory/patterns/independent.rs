use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";

fn run(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--compiler-patterns"];
    args.extend_from_slice(extra);
    advisory(directory, &args)
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--replay-run", original["run_directory"].as_str().unwrap()];
    args.extend_from_slice(extra);
    run(directory, &args)
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn only_patterns(report: &Value) {
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["diagnostic_execution"], "not_requested");
    assert!(
        advisory["units"]
            .as_array()
            .unwrap()
            .iter()
            .all(|unit| { unit.get("evaluation").is_none() && unit.get("patterns").is_some() })
    );
}

#[test]
fn independent_patterns_reuse_combined_proposals_without_numeric_evaluation() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), ANIMALS).unwrap();
    let (output, independent) = run(directory.path(), &["--fail-on-findings"]);
    checked(&output, 1);
    only_patterns(&independent);
    let (output, combined) = saved(directory.path(), &independent, &["--compiler-advisory"]);
    checked(&output, 0);
    assert_eq!(proposals(&independent), proposals(&combined));
    assert!(combined["compiler_advisory"]["units"][0]["evaluation"].is_object());
    assert_eq!(
        proposals(&independent)["evaluation"],
        patterns_json(directory.path(), &[])
    );
}

#[test]
fn independent_patterns_keep_unresolved_effect_evidence_without_incomplete_exit() {
    let directory = propagation::workspace("pub fn leaf() {}");
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!("{HELPERS} pub fn caller() {{ renamed::leaf(); }}"),
    )
    .unwrap();
    let (output, independent) = run(directory.path(), &["-p", "caller"]);
    checked(&output, 0);
    only_patterns(&independent);
    let unit = &independent["compiler_advisory"]["units"][0];
    assert_eq!(
        unit["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    assert_eq!(unit["patterns"]["input_complete"], true);
    assert_eq!(unit["patterns"]["effect_evidence"]["input_complete"], false);
    assert_eq!(independent["compiler_advisory"]["complete"], true);
    let (output, finding) = saved(directory.path(), &independent, &["--fail-on-findings"]);
    checked(&output, 1);
    assert_eq!(proposals(&finding), proposals(&independent));
    let (output, combined) = saved(directory.path(), &independent, &["--compiler-advisory"]);
    checked(&output, 2);
    assert_eq!(combined["compiler_advisory"]["complete"], false);
    assert_eq!(proposals(&combined), proposals(&independent));
}

#[test]
fn independent_pattern_replay_selects_findings_without_source_or_tools() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), HELPERS).unwrap();
    let (output, original) = run(directory.path(), &[]);
    checked(&output, 0);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .args([
            "advisory",
            "--format",
            "json",
            "--compiler-patterns",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--select-compiler-patterns",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ])
        .env("PATH", directory.path().join("no-tools"));
    for (paths, code) in [
        (vec![], 0),
        (vec!["src/lib.rs"], 1),
        (vec!["missing.rs"], 2),
    ] {
        std::fs::write(
            directory.path().join("selection.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"rstyle.changed-files/1", "paths":paths,
            }))
            .unwrap(),
        )
        .unwrap();
        let output = diagnostic_process::output(&mut command).unwrap();
        checked(&output, code);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        only_patterns(&report);
        assert_eq!(
            proposals(&report)["evaluation"],
            proposals(&original)["evaluation"]
        );
    }
}

#[test]
fn independent_pattern_reviews_and_baselines_reuse_saved_proposals() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), HELPERS).unwrap();
    let (output, original) = run(
        directory.path(),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 0);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    let (output, reviewed) = saved(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            candidate["id"].as_str().unwrap(),
            "--review-reason",
            "Fixture semantic review",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    only_patterns(&reviewed);
    assert_eq!(
        proposals(&reviewed)["visible_candidates"],
        serde_json::json!([])
    );
    assert_eq!(
        proposals(&reviewed)["evaluation"],
        proposals(&original)["evaluation"]
    );
    let (output, compared) = saved(
        directory.path(),
        &original,
        &[
            "--compiler-pattern-baseline",
            "baseline.json",
            "--fail-on-new-compiler-patterns",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["existing_findings"],
        1
    );
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["new_findings"],
        0
    );
}

#[test]
fn independent_patterns_preserve_requested_strict_matching() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn first(a:u32)->u32{a+1} pub fn second(a:u32)->u32{a+1}",
    )
    .unwrap();
    let (output, original) = advisory(directory.path(), &["--compiler-patterns"]);
    checked(&output, 0);
    assert_eq!(
        original["compiler_advisory"]["diagnostic_execution"],
        "not_requested"
    );
    assert!(
        original["compiler_advisory"]["units"][0]
            .get("evaluation")
            .is_none()
    );
    let (output, combined) = replay(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(proposals(&original), proposals(&combined));
}
