use super::mining_options::{checked, remove_sources, saved};
use super::*;

fn fixture() -> (TempDir, Value) {
    let directory = clone_workspace();
    let (capability, rest) = FAMILIES.split_once("pub fn inspect_dog").unwrap();
    let (generic, helpers) = rest.split_once("pub fn tag_left").unwrap();
    for (path, source) in [
        (
            "src/lib.rs",
            "pub mod cap; pub mod generic; pub mod helper;".into(),
        ),
        ("src/cap.rs", capability.into()),
        (
            "src/generic.rs",
            format!("use crate::cap::{{Dog,Cat}}; pub fn inspect_dog{generic}"),
        ),
        ("src/helper.rs", format!("pub fn tag_left{helpers}")),
    ] {
        std::fs::write(directory.path().join(path), source).unwrap();
    }
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
        ],
    );
    checked(&output, 0);
    let candidates = proposals(&report)["evaluation"]["candidates"]
        .as_array()
        .unwrap();
    assert_eq!(candidates.len(), 3);
    for kind in ["capability_set", "generic_fn", "parameterize"] {
        assert!(candidates.iter().any(|candidate| candidate["kind"] == kind));
    }
    (directory, report)
}

fn source_free(directory: &Path) {
    for path in ["src/cap.rs", "src/generic.rs", "src/helper.rs"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    remove_sources(directory);
}

fn unchanged(original: &Value, current: &Value) {
    assert_eq!(
        proposals(current)["evaluation"],
        proposals(original)["evaluation"]
    );
    assert_eq!(proposals(current)["policy"], proposals(original)["policy"]);
    assert_eq!(current["matching"], original["matching"]);
    assert_eq!(current["accepted_pairs"], original["accepted_pairs"]);
    let original_unit = &original["compiler_advisory"]["units"][0];
    let current_unit = &current["compiler_advisory"]["units"][0];
    assert_eq!(current_unit["evaluation"], original_unit["evaluation"]);
    assert_eq!(current_unit["observations"], original_unit["observations"]);
}

fn presentation(report: &Value) -> &Value {
    &proposals(report)["presentation"]
}

#[test]
fn top_limits_display_without_changing_raw_findings_and_default_replay_is_identical() {
    let (directory, original) = fixture();
    assert!(proposals(&original).get("presentation").is_none());
    source_free(directory.path());
    let (output, limited) = saved(
        directory.path(),
        &original,
        &["--top", "1", "--fail-on-findings"],
    );
    checked(&output, 1);
    assert_eq!(presentation(&limited)["top"], 1);
    assert_eq!(presentation(&limited)["min_score_milli"], 0);
    assert_eq!(
        presentation(&limited)["candidate_indices"],
        serde_json::json!([0])
    );
    let groups = proposals(&original)["evaluation"]["signature_groups"]
        .as_array()
        .unwrap();
    assert!(!groups.is_empty());
    assert_eq!(
        presentation(&limited)["signature_group_indices"],
        serde_json::json!([0])
    );
    unchanged(&original, &limited);
    let (output, defaults) = saved(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(proposals(&defaults), proposals(&original));
}

#[test]
fn score_threshold_is_inclusive_and_signature_groups_have_a_separate_mask() {
    let (directory, original) = fixture();
    source_free(directory.path());
    let evaluation = &proposals(&original)["evaluation"];
    let score = evaluation["candidates"][0]["score_milli"].as_u64().unwrap();
    assert!(score > 0);
    for boundary in [score, score + 1] {
        let value = format!("{}.{:03}", boundary / 1000, boundary % 1000);
        let (output, selected) = saved(directory.path(), &original, &["--min-score", &value]);
        checked(&output, 0);
        assert_eq!(presentation(&selected)["min_score_milli"], boundary);
        for (roster, mask) in [
            ("candidates", "candidate_indices"),
            ("signature_groups", "signature_group_indices"),
        ] {
            let expected: Vec<_> = evaluation[roster]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (row["score_milli"].as_u64().unwrap() >= boundary).then_some(index)
                })
                .collect();
            assert_eq!(presentation(&selected)[mask], serde_json::json!(expected));
        }
        assert_eq!(
            presentation(&selected)["candidate_indices"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(0)),
            boundary == score
        );
        unchanged(&original, &selected);
    }
}

#[test]
fn top_refills_after_reviews_before_intersecting_changed_files() {
    let (directory, original) = fixture();
    let candidates = proposals(&original)["evaluation"]["candidates"]
        .as_array()
        .unwrap();
    let first_path = candidates[0]["sites"][0]["path"].as_str().unwrap();
    let second_path = candidates[1]["sites"][0]["path"].as_str().unwrap();
    assert_ne!(first_path, second_path);
    std::fs::write(
        directory.path().join("selection.json"),
        serde_json::to_vec(&serde_json::json!({
            "schema":"rstyle.changed-files/1", "paths":[second_path],
        }))
        .unwrap(),
    )
    .unwrap();
    let filter = [
        "--top",
        "1",
        "--changed-files",
        "selection.json",
        "--select-compiler-patterns",
        "--fail-on-findings",
    ];
    let (output, selected) = saved(directory.path(), &original, &filter);
    checked(&output, 0);
    assert_eq!(
        selected["compiler_advisory"]["changed_scope"]["visible_findings"],
        0
    );
    reviews::dismiss_candidate(directory.path(), &original, &candidates[0], 0);
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    source_free(directory.path());
    let mut args = filter.to_vec();
    args.extend(["--review-file", "reviews.json", "--review-status"]);
    let (output, reviewed) = saved(directory.path(), &original, &args);
    checked(&output, 1);
    assert_eq!(
        presentation(&reviewed)["candidate_indices"],
        serde_json::json!([1])
    );
    assert_eq!(
        reviewed["compiler_advisory"]["changed_scope"]["visible_findings"],
        1
    );
    exact_review_unchanged(directory.path(), &reviewed, &bytes);
    unchanged(&original, &reviewed);
}

fn exact_review_unchanged(directory: &Path, reviewed: &Value, bytes: &[u8]) {
    assert_eq!(
        reviewed["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        reviewed["compiler_advisory"]["pattern_review"]["decisions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        std::fs::read(directory.join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn top_zero_keeps_every_baseline_snapshot_and_does_not_change_baseline_scope() {
    let (directory, original) = fixture();
    source_free(directory.path());
    let (output, hidden) = saved(
        directory.path(),
        &original,
        &[
            "--top",
            "0",
            "--write-compiler-pattern-baseline",
            "hidden-baseline.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        presentation(&hidden)["candidate_indices"],
        serde_json::json!([])
    );
    assert_eq!(
        presentation(&hidden)["signature_group_indices"],
        serde_json::json!([])
    );
    identical_baselines(directory.path(), &original);
    let (output, compared) = saved(
        directory.path(),
        &original,
        &[
            "--top",
            "0",
            "--compiler-pattern-baseline",
            "hidden-baseline.json",
            "--fail-on-new-compiler-patterns",
        ],
    );
    checked(&output, 0);
    let baseline = &compared["compiler_advisory"]["pattern_baseline"];
    assert_eq!(baseline["scope_changed"], false);
    assert_eq!(baseline["existing_findings"], 3);
    assert_eq!(baseline["current"].as_array().unwrap().len(), 3);
    unchanged(&original, &compared);
    zero_text(directory.path(), &original);
}

fn identical_baselines(directory: &Path, original: &Value) {
    let hidden_bytes = std::fs::read(directory.join("hidden-baseline.json")).unwrap();
    let hidden_baseline: Value = serde_json::from_slice(&hidden_bytes).unwrap();
    assert_eq!(hidden_baseline["entries"].as_array().unwrap().len(), 3);
    let (output, _) = saved(
        directory,
        original,
        &["--write-compiler-pattern-baseline", "full-baseline.json"],
    );
    checked(&output, 0);
    let full: Value =
        serde_json::from_slice(&std::fs::read(directory.join("full-baseline.json")).unwrap())
            .unwrap();
    assert_eq!(hidden_baseline, full);
}

fn zero_text(directory: &Path, original: &Value) {
    let text = rstyle(
        directory,
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--top",
            "0",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&text, 0);
    assert!(stdout(&text).contains("0 signature groups"));
    assert!(stdout(&text).contains("0 candidates"));
}

#[test]
fn top_is_applied_once_per_workspace_compilation_population() {
    let directory = propagation::workspace(FAMILIES);
    std::fs::write(directory.path().join("caller/src/lib.rs"), FAMILIES).unwrap();
    let (output, report) = propagation::capture(
        directory.path(),
        &["--compiler-patterns", "--all-targets", "--top", "1"],
    );
    checked(&output, 2);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 4);
    let populations: Vec<_> = units
        .iter()
        .filter_map(|unit| unit.get("patterns"))
        .collect();
    assert_eq!(populations.len(), 2);
    for patterns in populations {
        assert_eq!(patterns["member_units"].as_array().unwrap().len(), 2);
        assert_eq!(
            patterns["evaluation"]["candidates"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            patterns["presentation"]["candidate_indices"],
            serde_json::json!([0])
        );
    }
}

#[test]
fn presentation_cannot_hide_missing_inputs_or_run_without_patterns() {
    let source =
        format!("{FAMILIES}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();");
    let (directory, output, original) = capture(&source, &["--compiler-patterns"]);
    checked(&output, 2);
    remove_sources(directory.path());
    let (output, hidden) = saved(
        directory.path(),
        &original,
        &["--top", "0", "--fail-on-findings"],
    );
    checked(&output, 2);
    assert_eq!(proposals(&hidden)["input_complete"], false);
    assert_eq!(
        presentation(&hidden)["candidate_indices"],
        serde_json::json!([])
    );
    unchanged(&original, &hidden);
    for options in [vec!["--top", "0"], vec!["--min-score", "0.5"]] {
        let mut args = vec!["clones", "--compiler-advisory"];
        args.extend(options);
        let output = rstyle(directory.path(), &args);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("--compiler-patterns"));
    }
}
