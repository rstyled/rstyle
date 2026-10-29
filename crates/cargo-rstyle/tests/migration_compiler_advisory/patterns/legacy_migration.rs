use super::mining_options::{checked, remove_sources};
use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";

fn fixture(growth: bool) -> (TempDir, String, String) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), HELPERS).unwrap();
    let legacy = patterns_json(directory.path(), &["--no-ignore"]);
    let candidate = &legacy["candidates"][0];
    let id = candidate["id"].as_str().unwrap().to_owned();
    let sites: Vec<_> = candidate["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| {
            format!(
                "{}@{}",
                site["locator"].as_str().unwrap(),
                site["shape"].as_str().unwrap()
            )
        })
        .collect();
    let bytes = format!(
        "# Retain this original review and comment\nversion=1\n[ignore.{id}]\nkind=\"parameterize\"\nallow_growth={growth}\nreason=\"Original reviewed helper\"\nsites={}\n",
        serde_json::to_string(&sites).unwrap()
    );
    std::fs::write(directory.path().join("rstyle-ignore.toml"), &bytes).unwrap();
    (directory, id, bytes)
}

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "patterns",
        "--engine",
        "compiler",
        "--format",
        "json",
        "--preview-legacy-reviews",
    ];
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn migration(report: &Value) -> &Value {
    &report["compiler_advisory"]["pattern_review"]["legacy_migration"]
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
            "--engine",
            "compiler",
            "--format",
            "json",
            "--preview-legacy-reviews",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn selector(original: &Value) -> (String, String) {
    let target = &migration(original)["entries"][0]["matches"][0];
    (
        target["unit"].as_u64().unwrap().to_string(),
        target["candidate_id"].as_str().unwrap().to_owned(),
    )
}

fn import(directory: &Path, original: &Value, id: &str, extra: &[&str]) -> Output {
    let (unit, candidate) = selector(original);
    let mut args = vec![
        "--review-file",
        "reviews.json",
        "--import-legacy-pattern",
        id,
        &unit,
        &candidate,
    ];
    args.extend_from_slice(extra);
    saved(directory, original, &args)
}

#[test]
fn preview_uses_raw_candidates_and_never_creates_a_review_store() {
    let (directory, id, bytes) = fixture(false);
    let custom = directory.path().join("legacy.toml");
    std::fs::rename(directory.path().join("rstyle-ignore.toml"), &custom).unwrap();
    let output = raw(
        directory.path(),
        &[
            "--ignore-file",
            "legacy.toml",
            "--review-file",
            "missing-reviews.json",
            "--top",
            "0",
        ],
    );
    checked(&output, 0);
    let report = json(&output);
    let preview = migration(&report);
    assert_eq!(
        preview["schema"],
        "rstyle.compiler-pattern-legacy-migration/1"
    );
    assert!(
        preview["ledger_path"]
            .as_str()
            .unwrap()
            .ends_with("legacy.toml")
    );
    assert_eq!(preview["scope_complete"], true);
    assert_eq!(preview["complete"], true);
    let entry = &preview["entries"][0];
    assert_eq!(entry["legacy_id"], id);
    assert_eq!(entry["reason"], "Original reviewed helper");
    assert_eq!(entry["state"], "matched");
    assert_eq!(entry["matches"].as_array().unwrap().len(), 1);
    assert_eq!(entry["matches"][0]["snapshot_available"], true);
    assert_eq!(
        proposals(&report)["presentation"]["candidate_indices"],
        serde_json::json!([])
    );
    assert_eq!(
        proposals(&report)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    direct_preview_matches(directory.path(), &report);
    assert!(!directory.path().join("missing-reviews.json").exists());
    assert_eq!(std::fs::read_to_string(custom).unwrap(), bytes);
}

fn direct_preview_matches(directory: &Path, report: &Value) {
    let (output, direct) = clones(
        directory,
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-advisory-only",
            "--replay-run",
            report["run_directory"].as_str().unwrap(),
            "--legacy-pattern-ledger",
            "legacy.toml",
            "--top",
            "0",
        ],
    );
    checked(&output, 0);
    assert_eq!(migration(&direct)["entries"], migration(report)["entries"]);
}

#[test]
fn exact_import_retains_snapshot_reason_and_replays_without_source_or_tools() {
    let (directory, id, bytes) = fixture(false);
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    remove_sources(directory.path());
    let output = import(directory.path(), &original, &id, &["--top", "0"]);
    checked(&output, 0);
    let imported = json(&output);
    assert_eq!(migration(&imported)["imported"]["legacy_id"], id);
    assert_eq!(
        migration(&imported)["imported"]["current_members_only"],
        false
    );
    let path = directory.path().join("reviews.json");
    let store_bytes = std::fs::read(&path).unwrap();
    let store: Value = serde_json::from_slice(&store_bytes).unwrap();
    assert_eq!(
        store["compiler_pattern_entries"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        store["compiler_pattern_entries"][0]["reason"],
        "Original reviewed helper"
    );
    for (flags, expected) in [
        (vec![], "hidden_exact"),
        (vec!["--max-holes", "4"], "stale"),
        (vec!["--hide-implements", "unrelated"], "stale"),
    ] {
        let mut args = vec!["--review-file", "reviews.json", "--review-status"];
        args.extend(flags);
        let output = saved(directory.path(), &original, &args);
        checked(&output, 0);
        let current = json(&output);
        assert_eq!(current["review_status"]["records"][0]["state"], expected);
        assert_eq!(std::fs::read(&path).unwrap(), store_bytes);
    }
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn imported_body_review_becomes_visible_when_current_body_changes() {
    let (directory, id, bytes) = fixture(false);
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    checked(
        &import(
            directory.path(),
            &original,
            &id,
            &["--review-reason", "Explicit migration review"],
        ),
        0,
    );
    let path = directory.path().join("reviews.json");
    let store_bytes = std::fs::read(&path).unwrap();
    let store: Value = serde_json::from_slice(&store_bytes).unwrap();
    assert_eq!(
        store["compiler_pattern_entries"][0]["reason"],
        "Explicit migration review"
    );
    std::fs::write(
        directory.path().join("src/lib.rs"),
        HELPERS.replace("wrapping_add(8)", "wrapping_add(9)"),
    )
    .unwrap();
    let output = raw(
        directory.path(),
        &[
            "--review-file",
            "reviews.json",
            "--review-status",
            "--fail-on-candidates",
        ],
    );
    checked(&output, 1);
    let current = json(&output);
    assert_eq!(current["review_status"]["records"][0]["state"], "stale");
    // Legacy shape IDs omit literal values; the imported full snapshot still detects the edit.
    assert_eq!(migration(&current)["entries"][0]["state"], "matched");
    assert_eq!(std::fs::read(path).unwrap(), store_bytes);
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn growth_reviews_require_explicit_conversion_to_current_members_only() {
    let (directory, id, bytes) = fixture(true);
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    assert_eq!(migration(&original)["entries"][0]["allow_growth"], true);
    assert_eq!(
        migration(&original)["entries"][0]["state"],
        "growth_review_required"
    );
    checked(&import(directory.path(), &original, &id, &[]), 2);
    assert!(!directory.path().join("reviews.json").exists());
    let output = import(
        directory.path(),
        &original,
        &id,
        &["--legacy-current-members-only"],
    );
    checked(&output, 0);
    let imported = json(&output);
    assert_eq!(
        migration(&imported)["imported"]["current_members_only"],
        true
    );
    std::fs::write(directory.path().join("src/lib.rs"), format!("{HELPERS}\npub fn third(a:u128)->u128 {{std::hint::black_box(a.wrapping_add(9)).wrapping_mul(3)}}")).unwrap();
    let output = raw(
        directory.path(),
        &["--review-file", "reviews.json", "--review-status"],
    );
    checked(&output, 0);
    let grown = json(&output);
    assert_eq!(grown["review_status"]["records"][0]["state"], "stale");
    assert_eq!(
        migration(&grown)["entries"][0]["state"],
        "growth_review_required"
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn check_and_test_matches_stay_distinct_and_import_selects_only_one_context() {
    let (directory, id, _) = fixture(false);
    let output = raw(directory.path(), &["--all-targets"]);
    checked(&output, 2);
    let original = json(&output);
    let entry = &migration(&original)["entries"][0];
    assert_eq!(entry["state"], "multiple_contexts");
    let matches = entry["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 2);
    assert_ne!(matches[0]["unit"], matches[1]["unit"]);
    let output = import(directory.path(), &original, &id, &[]);
    checked(&output, 2);
    let imported = json(&output);
    let decisions = imported["compiler_advisory"]["pattern_review"]["decisions"]
        .as_array()
        .unwrap();
    assert_eq!(
        decisions
            .iter()
            .filter(|row| row["state"] == "hidden_exact")
            .count(),
        1
    );
    let store: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("reviews.json")).unwrap())
            .unwrap();
    assert_eq!(
        store["compiler_pattern_entries"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn missing_capture_is_unavailable_rather_than_a_completed_no_match() {
    let (directory, _, bytes) = fixture(false);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn clean(a:u128)->u128 {a}",
    )
    .unwrap();
    let output = raw(directory.path(), &["--review-file", "missing-reviews.json"]);
    checked(&output, 0);
    let complete = json(&output);
    assert_eq!(
        migration(&complete)["entries"][0]["state"],
        "no_current_match"
    );
    assert_eq!(migration(&complete)["scope_complete"], true);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "macro_rules! make {()=>{pub fn generated(){}}} make!();",
    )
    .unwrap();
    let output = raw(directory.path(), &["--review-file", "missing-reviews.json"]);
    checked(&output, 2);
    let missing = json(&output);
    assert_eq!(migration(&missing)["scope_complete"], false);
    assert_eq!(migration(&missing)["entries"][0]["state"], "unavailable");
    assert!(!directory.path().join("missing-reviews.json").exists());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn invalid_selectors_and_malformed_inputs_never_publish_partial_imports() {
    let (directory, id, bytes) = fixture(false);
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = json(&output);
    let (unit, candidate) = selector(&original);
    for (legacy, unit, candidate) in [
        ("000000000000", unit.as_str(), candidate.as_str()),
        (id.as_str(), "999999", candidate.as_str()),
        (id.as_str(), unit.as_str(), "unknown"),
    ] {
        let output = saved(
            directory.path(),
            &original,
            &[
                "--review-file",
                "reviews.json",
                "--import-legacy-pattern",
                legacy,
                unit,
                candidate,
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("reviews.json").exists());
    }
    let ledger = directory.path().join("rstyle-ignore.toml");
    std::fs::write(&ledger, "malformed TOML").unwrap();
    checked(&import(directory.path(), &original, &id, &[]), 2);
    assert!(!directory.path().join("reviews.json").exists());
    assert_eq!(std::fs::read_to_string(&ledger).unwrap(), "malformed TOML");
    std::fs::write(&ledger, &bytes).unwrap();
    checked(
        &saved(
            directory.path(),
            &original,
            &["--ignore-file", "absent-ledger.toml"],
        ),
        2,
    );
    assert!(!directory.path().join("reviews.json").exists());
    let store = directory.path().join("reviews.json");
    std::fs::write(&store, "malformed review JSON").unwrap();
    checked(&import(directory.path(), &original, &id, &[]), 2);
    assert_eq!(
        std::fs::read_to_string(store).unwrap(),
        "malformed review JSON"
    );
    assert_eq!(std::fs::read_to_string(ledger).unwrap(), bytes);
}
