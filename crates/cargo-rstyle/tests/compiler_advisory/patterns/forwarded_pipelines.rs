use super::*;

const PIPELINES: &str = r#"
fn left_first(x:u128)->u128 { x.wrapping_add(1) }
fn left_middle(x:u128)->u128 { x.wrapping_add(2) }
fn left_last(x:u128)->u128 { x.wrapping_add(3) }
fn right_first(x:u128)->u128 { x.wrapping_add(4) }
fn right_middle(x:u128)->u128 { x.wrapping_add(5) }
fn right_last(x:u128)->u128 { x.wrapping_add(6) }
pub fn left(x:u128)->u128 { left_last(left_middle(left_first(x))) }
pub fn right(x:u128)->u128 { right_last(right_middle(right_first(x))) }
"#;

#[test]
fn distinct_forwarded_stages_do_not_propose_a_helper_and_replay_without_sources() {
    let (directory, output, original) = capture(PIPELINES, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let patterns = proposals(&original);
    assert_eq!(patterns["input_complete"], true);
    assert_eq!(patterns["policy_version"], 2);
    for name in ["left", "right"] {
        assert!(candidate_at(&patterns["evaluation"], name).is_none());
    }
    assert!(
        patterns["evaluation"]["suppressed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| site_names(row, "sites") == ["left", "right"]
                && row["reason"]
                    == "call-only forwarding through distinct single-use callee substitutions")
    );
    assert_eq!(patterns["evaluation"], patterns_json(directory.path(), &[]));
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, saved) = replay(directory.path(), &original, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(patterns, proposals(&saved));
    assert_eq!(original["matching"], saved["matching"]);
    assert_eq!(original["accepted_pairs"], saved["accepted_pairs"]);
}

#[test]
fn arithmetic_and_common_stages_keep_shared_pipeline_work_visible() {
    let arithmetic = PIPELINES
        .replace(
            "left_last(left_middle(left_first(x)))",
            "left_last(left_middle(left_first(x))) + 7",
        )
        .replace(
            "right_last(right_middle(right_first(x)))",
            "right_last(right_middle(right_first(x))) + 7",
        );
    let common = PIPELINES.replace(
        "right_last(right_middle(right_first(x)))",
        "left_last(right_middle(right_first(x)))",
    );
    for source in [arithmetic, common] {
        let (directory, output, report) = capture(&source, &["--compiler-patterns"]);
        assert!(output.status.success(), "{}", stdout(&output));
        assert_eq!(proposals(&report)["policy_version"], 2);
        let evaluation = &proposals(&report)["evaluation"];
        assert_eq!(
            candidate_at(evaluation, "left").unwrap()["kind"],
            "parameterize"
        );
        assert_eq!(evaluation, &patterns_json(directory.path(), &[]));
    }
}

#[test]
fn repeated_stage_substitutions_keep_a_parameterized_helper_proposal() {
    let source = r#"
fn left_stage(x:u128)->u128 { x.wrapping_add(1) }
fn right_stage(x:u128)->u128 { x.wrapping_add(2) }
pub fn left(x:u128)->u128 { left_stage(left_stage(x)) }
pub fn right(x:u128)->u128 { right_stage(right_stage(x)) }
"#;
    let (directory, output, report) = capture(source, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(proposals(&report)["policy_version"], 2);
    let evaluation = &proposals(&report)["evaluation"];
    assert_eq!(
        candidate_at(evaluation, "left").unwrap()["kind"],
        "parameterize"
    );
    assert_eq!(evaluation, &patterns_json(directory.path(), &[]));
}

#[test]
fn previous_proposal_policy_reviews_are_readable_but_stale_and_baselines_unsupported() {
    let source = PIPELINES.replace(
        "right_last(right_middle(right_first(x)))",
        "left_last(right_middle(right_first(x)))",
    );
    let (directory, output, original) = capture(&source, &["--compiler-patterns"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(proposals(&original)["policy_version"], 2);
    let candidate = candidate_at(&proposals(&original)["evaluation"], "left").unwrap();
    reviews::dismiss_candidate(directory.path(), &original, candidate, 0);
    let path = directory.path().join("reviews.json");
    let mut store: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let versions = &mut store["compiler_pattern_entries"][0]["finding"]["versions"];
    assert_eq!(versions["detector_version"], 2);
    versions["detector_version"] = 1.into();
    let bytes = serde_json::to_vec(&store).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let (output, stale) = replay(
        directory.path(),
        &original,
        &["--review-file", "reviews.json", "--fail-on-findings"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let decision = stale["compiler_advisory"]["pattern_review"]["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["candidate_id"] == candidate["id"])
        .unwrap();
    assert_eq!(decision["state"], "stale");
    assert!(decision["reason"].as_str().unwrap().contains("Versions"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(stale["matching"], original["matching"]);
    old_baseline_is_unsupported(directory.path(), &original);
}

fn old_baseline_is_unsupported(directory: &Path, original: &Value) {
    let (output, _) = replay(
        directory,
        original,
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let path = directory.join("baseline.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["versions"]["detector_version"], 2);
    saved["versions"]["detector_version"] = 1.into();
    for entry in saved["entries"].as_array_mut().unwrap() {
        entry["versions"]["detector_version"] = 1.into();
    }
    let bytes = serde_json::to_vec(&saved).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let output = rstyle(
        directory,
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--compiler-pattern-baseline",
            "baseline.json",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Unsupported compiler proposal baseline schema, versions or policy")
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
