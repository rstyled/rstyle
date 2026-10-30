use super::mining_options::{checked, remove_sources};
use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
const THIRD: &str =
    "pub fn third(a:u128)->u128 {std::hint::black_box(a.wrapping_add(9)).wrapping_mul(3)}";

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--engine", "compiler", "--format", "json"];
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

fn fixture(source: &str) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let output = raw(directory.path(), &["--no-ignore"]);
    checked(&output, 0);
    (directory, json(&output))
}

fn ledger(directory: &Path, candidate: &Value, growth: bool) -> String {
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
    let definitions: Vec<_> = candidate["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["locator"].as_str().unwrap())
        .collect();
    let bytes = format!(
        "# Preserve the repository review and comment\nversion=1\n[ignore.{}]\nkind={:?}\nallow_growth={growth}\nreason=\"Reviewed helper\"\nsites={}\ndefinitions={}\n",
        candidate["id"].as_str().unwrap(),
        candidate["kind"].as_str().unwrap(),
        serde_json::to_string(&sites).unwrap(),
        serde_json::to_string(&definitions).unwrap()
    );
    std::fs::write(directory.join("rstyle-ignore.toml"), &bytes).unwrap();
    bytes
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    saved_format(directory, original, "json", extra)
}

fn saved_format(directory: &Path, original: &Value, format: &str, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
            "--engine",
            "compiler",
            "--format",
            format,
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

fn hidden(report: &Value, count: usize) {
    let patterns = proposals(report);
    assert_eq!(patterns["evaluation"]["ignored_count"], count);
    assert_eq!(
        patterns["evaluation"]["ignored"].as_array().unwrap().len(),
        count
    );
    assert_eq!(
        patterns["repository_visible_candidates"],
        serde_json::json!([])
    );
}

#[test]
fn repository_exact_review_replays_without_source_or_tools_and_preserves_raw_candidates() {
    let (directory, original) = fixture(HELPERS);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    let bytes = ledger(directory.path(), candidate, false);
    remove_sources(directory.path());
    let output = saved(directory.path(), &original, &["--fail-on-candidates"]);
    checked(&output, 0);
    let reviewed = json(&output);
    hidden(&reviewed, 1);
    shown_ignored(directory.path(), &original);
    assert_eq!(
        proposals(&reviewed)["evaluation"]["candidates"],
        proposals(&original)["evaluation"]["candidates"]
    );
    let ignored = &proposals(&reviewed)["evaluation"]["ignored"][0];
    assert_eq!(ignored["entry_id"], candidate["id"]);
    assert_eq!(ignored["matched_by"], "exact");
    assert_eq!(ignored["reason"], "Reviewed helper");
    let output = saved(
        directory.path(),
        &original,
        &["--no-ignore", "--fail-on-candidates"],
    );
    checked(&output, 1);
    assert_eq!(
        proposals(&json(&output))["evaluation"],
        proposals(&original)["evaluation"]
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
    assert!(!directory.path().join("reviews.json").exists());
}

fn shown_ignored(directory: &Path, original: &Value) {
    let plain = saved_format(directory, original, "text", &[]);
    checked(&plain, 0);
    assert!(!stdout(&plain).contains("Reviewed helper"));
    let shown = saved_format(directory, original, "text", &["--show-ignored"]);
    checked(&shown, 0);
    assert!(stdout(&shown).contains("Reviewed helper"));
    assert!(stdout(&shown).contains("\n\n1 ignored candidates"));
}

#[test]
fn repository_growth_review_reuses_member_matching_and_exact_reviews_do_not_hide_growth() {
    let (directory, original) = fixture(HELPERS);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    ledger(directory.path(), candidate, false);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{HELPERS}\n{THIRD}"),
    )
    .unwrap();
    let output = raw(directory.path(), &["--fail-on-candidates"]);
    checked(&output, 1);
    let grown = json(&output);
    assert_eq!(proposals(&grown)["evaluation"]["ignored_count"], 0);
    assert_eq!(
        proposals(&grown)["evaluation"]["candidates"][0]["sites"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let bytes = ledger(directory.path(), candidate, true);
    remove_sources(directory.path());
    let output = saved(directory.path(), &grown, &["--fail-on-candidates"]);
    checked(&output, 0);
    let reviewed = json(&output);
    hidden(&reviewed, 1);
    assert_eq!(
        proposals(&reviewed)["evaluation"]["ignored"][0]["matched_by"],
        serde_json::json!({"growth":{"added":1}})
    );
    assert_eq!(
        proposals(&reviewed)["evaluation"]["candidates"],
        proposals(&grown)["evaluation"]["candidates"]
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("rstyle-ignore.toml")).unwrap(),
        bytes
    );
}

#[test]
fn explicit_repository_ledger_matches_root_route_and_top_does_not_change_review_matching() {
    let (directory, original) = fixture(HELPERS);
    ledger(
        directory.path(),
        &proposals(&original)["evaluation"]["candidates"][0],
        false,
    );
    std::fs::rename(
        directory.path().join("rstyle-ignore.toml"),
        directory.path().join("reviewed.toml"),
    )
    .unwrap();
    let output = saved(
        directory.path(),
        &original,
        &["--ignore-file", "reviewed.toml", "--top", "0"],
    );
    checked(&output, 0);
    let routed = json(&output);
    hidden(&routed, 1);
    let (output, direct) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-advisory-only",
            "--compiler-pattern-scope",
            "authored",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--pattern-ledger",
            "reviewed.toml",
            "--top",
            "0",
        ],
    );
    checked(&output, 0);
    assert_eq!(proposals(&direct), proposals(&routed));
}

#[test]
fn malformed_repository_ledger_fails_without_mutation_and_no_ignore_bypasses_it() {
    let (directory, original) = fixture(HELPERS);
    let path = directory.path().join("rstyle-ignore.toml");
    std::fs::write(&path, "malformed TOML").unwrap();
    remove_sources(directory.path());
    checked(&saved(directory.path(), &original, &[]), 2);
    let output = saved(
        directory.path(),
        &original,
        &["--no-ignore", "--fail-on-candidates"],
    );
    checked(&output, 1);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "malformed TOML");
    let output = saved(
        directory.path(),
        &original,
        &["--ignore-file", "absent.toml", "--fail-on-candidates"],
    );
    checked(&output, 1);
    assert_eq!(proposals(&json(&output))["evaluation"]["ignored_count"], 0);
}

#[test]
fn repository_and_scoped_reviews_intersect_before_top_without_changing_raw_findings() {
    let (directory, original) = fixture(FAMILIES);
    let candidates = proposals(&original)["evaluation"]["candidates"]
        .as_array()
        .unwrap();
    ledger(directory.path(), &candidates[0], false);
    remove_sources(directory.path());
    let output = saved(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            candidates[1]["id"].as_str().unwrap(),
            "--review-reason",
            "Reviewed independently",
            "--top",
            "1",
        ],
    );
    checked(&output, 0);
    let reviewed = json(&output);
    let patterns = proposals(&reviewed);
    assert_eq!(
        patterns["evaluation"]["candidates"],
        proposals(&original)["evaluation"]["candidates"]
    );
    assert_eq!(
        patterns["repository_visible_candidates"],
        serde_json::json!([1, 2])
    );
    assert_eq!(
        patterns["presentation"]["candidate_indices"],
        serde_json::json!([2])
    );
    assert_eq!(patterns["evaluation"]["ignored_count"], 1);
    assert_eq!(
        reviewed["compiler_advisory"]["pattern_review"]["decisions"][1]["state"],
        "hidden_exact"
    );
}

#[test]
fn repository_review_filters_before_top_without_scoped_reviews() {
    let (directory, original) = fixture(FAMILIES);
    let candidates = proposals(&original)["evaluation"]["candidates"]
        .as_array()
        .unwrap();
    assert!(candidates.len() >= 2);
    ledger(directory.path(), &candidates[0], false);
    remove_sources(directory.path());
    let output = saved(
        directory.path(),
        &original,
        &["--top", "1", "--fail-on-candidates"],
    );
    checked(&output, 1);
    let reviewed = json(&output);
    assert_eq!(
        proposals(&reviewed)["presentation"]["candidate_indices"],
        serde_json::json!([1])
    );
    assert_eq!(
        proposals(&reviewed)["evaluation"]["candidates"],
        proposals(&original)["evaluation"]["candidates"]
    );
}
