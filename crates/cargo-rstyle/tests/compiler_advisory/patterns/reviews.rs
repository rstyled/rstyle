use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";

pub(super) fn dismiss_candidate(
    directory: &Path,
    original: &Value,
    candidate: &Value,
    unit: usize,
) -> Value {
    let (output, report) = replay(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            &unit.to_string(),
            candidate["id"].as_str().unwrap(),
            "--review-reason",
            "Fixture semantic review",
        ],
    );
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    report
}

fn decisions(report: &Value) -> &[Value] {
    report["compiler_advisory"]["pattern_review"]["decisions"]
        .as_array()
        .unwrap()
}

#[test]
fn exact_compiler_reviews_cover_all_five_core_proposal_families() {
    for source in [FAMILIES, ANIMALS] {
        let (directory, _output, original) = capture(source, &["--compiler-patterns"]);
        let candidates = proposals(&original)["evaluation"]["candidates"]
            .as_array()
            .unwrap();
        let mut kinds = std::collections::BTreeSet::new();
        for candidate in candidates {
            let reviewed = dismiss_candidate(directory.path(), &original, candidate, 0);
            assert!(decisions(&reviewed).iter().any(
                |row| row["candidate_id"] == candidate["id"] && row["state"] == "hidden_exact"
            ));
            assert_eq!(reviewed["matching"], original["matching"]);
            assert_eq!(
                proposals(&reviewed)["evaluation"],
                proposals(&original)["evaluation"]
            );
            kinds.insert(candidate["kind"].as_str().unwrap());
        }
        let expected: std::collections::BTreeSet<_> = if source == FAMILIES {
            ["capability_set", "generic_fn", "parameterize"]
                .into_iter()
                .collect()
        } else {
            ["trait_method", "enum_dispatch"].into_iter().collect()
        };
        assert_eq!(kinds, expected);
        let store: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
                .unwrap();
        assert_eq!(store["schema_version"], 8);
        assert_eq!(
            store["compiler_pattern_entries"].as_array().unwrap().len(),
            kinds.len()
        );
    }
}

#[test]
fn compiler_reviews_survive_movement_binders_and_source_free_replay() {
    let (directory, _output, original) = capture(HELPERS, &["--compiler-patterns"]);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    dismiss_candidate(directory.path(), &original, candidate, 0);
    let moved = format!(
        "\n\n// unrelated movement\n{}",
        HELPERS
            .replace("a:u128", "renamed:u128")
            .replace("a.wrapping_add", "renamed.wrapping_add")
    );
    std::fs::write(directory.path().join("src/lib.rs"), moved).unwrap();
    let (output, current) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
            "--fail-on-findings",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(decisions(&current)[0]["state"], "hidden_exact");
    assert_eq!(
        proposals(&current)["visible_candidates"],
        serde_json::json!([])
    );
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, saved) = replay(
        directory.path(),
        &current,
        &[
            "--review-file",
            "reviews.json",
            "--fail-on-findings",
            "--review-status",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(decisions(&saved)[0]["state"], "hidden_exact");
    assert_eq!(
        saved["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(saved["matching"], current["matching"]);
}

#[test]
fn changed_compiler_graphs_stale_reviews_and_failed_selection_is_atomic() {
    let (directory, _output, original) = capture(HELPERS, &["--compiler-patterns"]);
    dismiss_candidate(
        directory.path(),
        &original,
        &proposals(&original)["evaluation"]["candidates"][0],
        0,
    );
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        HELPERS.replace("wrapping_add(8)", "wrapping_add(9)"),
    )
    .unwrap();
    let (_output, current) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
            "--review-status",
        ],
    );
    assert_eq!(decisions(&current)[0]["state"], "stale");
    assert_eq!(current["review_status"]["records"][0]["state"], "stale");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--replay-run",
            current["run_directory"].as_str().unwrap(),
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            "unknown",
            "--review-reason",
            "Not a finding",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn compiler_reviews_keep_check_test_contexts_separate() {
    let (directory, _output, original) = capture(HELPERS, &["--compiler-patterns"]);
    dismiss_candidate(
        directory.path(),
        &original,
        &proposals(&original)["evaluation"]["candidates"][0],
        0,
    );
    let (_output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--all-targets",
            "--review-file",
            "reviews.json",
        ],
    );
    assert_eq!(decisions(&report).len(), 2);
    assert_eq!(
        decisions(&report)
            .iter()
            .filter(|row| row["state"] == "hidden_exact")
            .count(),
        1
    );
    assert_eq!(
        decisions(&report)
            .iter()
            .filter(|row| row["state"] == "visible_new")
            .count(),
        1
    );
}

#[test]
fn missing_proposal_does_not_prune_a_retained_compiler_review() {
    let (directory, _output, original) = capture(HELPERS, &["--compiler-patterns"]);
    dismiss_candidate(
        directory.path(),
        &original,
        &proposals(&original)["evaluation"]["candidates"][0],
        0,
    );
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(
        directory.path().join("exclude.toml"),
        "exclude=['src/**']\n",
    )
    .unwrap();
    let (output, report) = replay(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--review-prune",
            "--compiler-advisory-config",
            "exclude.toml",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "unavailable"
    );
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn compiler_trait_reviews_survive_impl_renumbering_and_definition_body_edits() {
    let (directory, _output, original) = capture(ANIMALS, &["--compiler-patterns"]);
    let candidate = candidate_at(&proposals(&original)["evaluation"], "emit_dogs").unwrap();
    dismiss_candidate(directory.path(), &original, candidate, 0);
    let changed = format!(
        "pub struct Unrelated; impl Unrelated {{ pub fn unchanged(&self) -> u128 {{ 42 }} }}\n{}",
        ANIMALS.replace("{}: woof", "{}: a different sound")
    );
    assert!(changed.contains("{}: a different sound"));
    std::fs::write(directory.path().join("src/lib.rs"), changed).unwrap();
    let (_output, current) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
            "--review-status",
        ],
    );
    let trait_candidate = candidate_at(&proposals(&current)["evaluation"], "emit_dogs").unwrap();
    assert!(
        decisions(&current)
            .iter()
            .any(|row| row["candidate_id"] == trait_candidate["id"]
                && row["state"] == "hidden_exact")
    );
    assert_eq!(
        current["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
}

#[test]
fn compiler_review_status_retains_omitted_families_and_lookup_never_creates_files() {
    let (directory, _output, original) = capture(HELPERS, &["--compiler-patterns"]);
    let path = directory.path().join("reviews.json");
    let (_output, report) = replay(
        directory.path(),
        &original,
        &["--review-file", "reviews.json"],
    );
    assert_eq!(decisions(&report)[0]["state"], "visible_new");
    assert!(!path.exists());
    dismiss_candidate(
        directory.path(),
        &original,
        &proposals(&original)["evaluation"]["candidates"][0],
        0,
    );
    let bytes = std::fs::read(&path).unwrap();
    let (output, status) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--source-rules",
            "--review-file",
            "reviews.json",
            "--review-status",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        status["review_status"]["records"][0]["state"],
        "not_requested"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
