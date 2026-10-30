use super::mining_options::{checked, remove_sources};
use super::*;

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--format", "json"];
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let output = saved_output(directory, original, extra);
    let value = report(&output);
    (output, value)
}

fn saved_output(directory: &Path, original: &Value, extra: &[&str]) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
            "--format",
            "json",
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

fn only_patterns(value: &Value) {
    assert_eq!(value["matching_execution"], "not_requested");
    assert_eq!(value["strict_assessment_execution"], "not_requested");
    assert_eq!(
        value["compiler_advisory"]["diagnostic_execution"],
        "not_requested"
    );
    for unit in value["compiler_advisory"]["units"].as_array().unwrap() {
        assert!(unit.get("evaluation").is_none());
        assert!(unit["patterns"].is_object());
    }
}

#[test]
fn compiler_route_reuses_shared_capture_options_and_source_free_replay() {
    let directory = fixture(FAMILIES);
    std::fs::write(
        directory.path().join("Cargo.lock"),
        "version = 4\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let options = [
        "--locked",
        "--max-holes",
        "4",
        "--min-score",
        "0.1",
        "--top",
        "1",
        "--fail-on-candidates",
    ];
    let output = raw(directory.path(), &options);
    checked(&output, 1);
    let original = report(&output);
    only_patterns(&original);
    assert_eq!(
        proposals(&original)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(proposals(&original)["policy"]["max_holes"], 4);
    direct_entry_agrees(directory.path(), &original);
    remove_sources(directory.path());
    let (output, replayed) = saved(directory.path(), &original, &options[1..]);
    checked(&output, 1);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
    let (output, hidden) = saved(
        directory.path(),
        &original,
        &["--top", "0", "--fail-on-candidates"],
    );
    checked(&output, 0);
    assert_eq!(
        proposals(&hidden)["evaluation"],
        proposals(&original)["evaluation"]
    );
    assert_eq!(
        proposals(&hidden)["presentation"]["candidate_indices"],
        serde_json::json!([])
    );
}

fn direct_entry_agrees(directory: &Path, original: &Value) {
    let (output, explicit) = clones(
        directory,
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--pattern-ledger",
            "rstyle-ignore.toml",
            "--compiler-advisory-only",
            "--report-detail",
            "compact",
            "--compiler-pattern-scope",
            "authored",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--max-holes",
            "4",
            "--min-score",
            "0.1",
            "--top",
            "1",
        ],
    );
    checked(&output, 0);
    assert_eq!(proposals(&explicit), proposals(original));
}

#[test]
fn compiler_route_reuses_scoped_reviews_without_requiring_strict_matching() {
    let directory = fixture(ANIMALS);
    let output = raw(directory.path(), &[]);
    checked(&output, 0);
    let original = report(&output);
    let candidate = &proposals(&original)["evaluation"]["candidates"][0];
    remove_sources(directory.path());
    invalid_review_preserves_store(directory.path(), &original, None);
    let (output, dismissed) = saved(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            candidate["id"].as_str().unwrap(),
            "--review-reason",
            "Independent route review",
        ],
    );
    checked(&output, 0);
    only_patterns(&dismissed);
    assert_eq!(
        dismissed["compiler_advisory"]["pattern_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        proposals(&dismissed)["evaluation"],
        proposals(&original)["evaluation"]
    );
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    invalid_review_preserves_store(directory.path(), &original, Some(&bytes));
    review_status_is_stable(directory.path(), &original, &bytes);
}

fn review_status_is_stable(directory: &Path, original: &Value, bytes: &[u8]) {
    let (output, reviewed) = saved(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--review-status",
            "--top",
            "1",
        ],
    );
    checked(&output, 0);
    only_patterns(&reviewed);
    assert_eq!(
        reviewed["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        proposals(&reviewed)["evaluation"],
        proposals(original)["evaluation"]
    );
    assert_eq!(
        std::fs::read(directory.join("reviews.json")).unwrap(),
        bytes
    );
}

fn invalid_review_preserves_store(directory: &Path, original: &Value, expected: Option<&[u8]>) {
    let output = saved_output(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            "unknown-candidate",
            "--review-reason",
            "Invalid selector must not publish",
        ],
    );
    checked(&output, 2);
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Selected saved-run compiler proposal is not a current finding")
    );
    let actual = std::fs::read(directory.join("reviews.json"));
    if let Some(expected) = expected {
        assert_eq!(actual.unwrap(), expected);
    } else {
        assert_eq!(actual.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}

#[test]
fn compiler_route_defaults_to_authored_scope_and_reports_generated_owners() {
    let directory = fixture(&format!(
        "{FAMILIES}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();"
    ));
    let output = raw(directory.path(), &["--top", "0", "--fail-on-candidates"]);
    checked(&output, 0);
    let original = report(&output);
    only_patterns(&original);
    assert_eq!(proposals(&original)["input_complete"], true);
    assert_eq!(proposals(&original)["source_scope"]["kind"], "authored");
    assert!(
        !proposals(&original)["source_scope"]["outside_scope_functions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        proposals(&original)["presentation"]["candidate_indices"],
        serde_json::json!([])
    );
    remove_sources(directory.path());
    let (output, replayed) = saved(
        directory.path(),
        &original,
        &["--top", "0", "--fail-on-candidates"],
    );
    checked(&output, 0);
    assert_eq!(proposals(&replayed), proposals(&original));
}

#[test]
fn explicit_reference_engine_and_compiler_both_apply_repository_reviews() {
    let directory = fixture(FAMILIES);
    let original = patterns_json(directory.path(), &["--no-ignore"]);
    let candidate = &original["candidates"][0];
    let bytes = format!(
        "# Preserve this repository review\nversion = 1\n[ignore.{}]\nkind = {:?}\nreason = \"Reviewed proposal\"\n",
        candidate["id"].as_str().unwrap(),
        candidate["kind"].as_str().unwrap()
    );
    let path = directory.path().join("rstyle-ignore.toml");
    std::fs::write(&path, &bytes).unwrap();
    let current = patterns_json(directory.path(), &[]);
    assert!(current.get("compiler_advisory").is_none());
    assert_eq!(current["ignored_count"], 1);
    assert_eq!(current, patterns_json(directory.path(), &[]));
    let output = raw(directory.path(), &["--show-ignored"]);
    checked(&output, 0);
    let compiler = report(&output);
    assert_eq!(proposals(&compiler)["evaluation"]["ignored_count"], 1);
    assert_eq!(
        proposals(&compiler)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), bytes);
}

#[test]
fn compiler_route_preserves_check_and_test_contexts_on_saved_replay() {
    let directory = fixture("pub fn original(){} #[cfg(test)] pub fn test_only(){}");
    let output = raw(directory.path(), &["--all-targets"]);
    checked(&output, 0);
    let original = report(&output);
    only_patterns(&original);
    let units = original["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut counts: Vec<_> = units
        .iter()
        .map(|unit| unit["patterns"]["observed_graphs"].as_u64().unwrap())
        .collect();
    counts.sort();
    assert_eq!(counts, [1, 2]);
    let observed = original["observed_units"].as_array().unwrap();
    assert!(observed.iter().any(|unit| unit["test"] == true));
    assert!(observed.iter().any(|unit| unit["test"] == false));
    remove_sources(directory.path());
    let (output, replayed) = saved(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn compiler_route_reuses_package_manifest_and_default_or_explicit_config() {
    let directory = propagation::workspace(FAMILIES);
    std::fs::write(directory.path().join("caller/src/lib.rs"), FAMILIES).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "exclude=['caller/src/**']\n",
    )
    .unwrap();
    let manifest = directory.path().join("Cargo.toml");
    let args = [
        "-p",
        "caller",
        "--manifest-path",
        manifest.to_str().unwrap(),
    ];
    let output = raw(directory.path(), &args);
    checked(&output, 0);
    let excluded = report(&output);
    assert_eq!(
        excluded["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(proposals(&excluded)["policy_selected_functions"], 0);
    std::fs::write(directory.path().join("custom.toml"), "exclude=[]\n").unwrap();
    let mut args = args.to_vec();
    args.extend(["--config", "custom.toml"]);
    let output = raw(directory.path(), &args);
    checked(&output, 0);
    let included = report(&output);
    only_patterns(&included);
    assert_eq!(
        included["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        included["compiler_advisory"]["units"][0]["package_root"]
            .as_str()
            .unwrap()
            .ends_with("caller")
    );
    assert_eq!(
        proposals(&included)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
