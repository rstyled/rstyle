use super::mining_options::{checked, remove_sources};
use super::*;

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn raw(directory: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["patterns", "--engine", "compiler", "--format", "json"];
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
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "patterns",
            "--engine",
            "compiler",
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
    let output = diagnostic_process::output(&mut command).unwrap();
    let value = report(&output);
    (output, value)
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
            "--compiler-advisory-only",
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
    let (output, _) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
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
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    remove_sources(directory.path());
    let (output, reviewed) = saved(
        directory.path(),
        &original,
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
        proposals(&original)["evaluation"]
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn compiler_route_retains_missing_generated_source_even_when_display_is_empty() {
    let directory = fixture(&format!(
        "{FAMILIES}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();"
    ));
    let output = raw(directory.path(), &["--top", "0", "--fail-on-candidates"]);
    checked(&output, 2);
    let original = report(&output);
    only_patterns(&original);
    assert_eq!(proposals(&original)["input_complete"], false);
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
    checked(&output, 2);
    assert_eq!(proposals(&replayed), proposals(&original));
}

#[test]
fn compiler_route_preserves_legacy_defaults_and_requires_explicit_ledger_bypass() {
    let directory = fixture(FAMILIES);
    let original = patterns_json(directory.path(), &["--no-ignore"]);
    let candidate = &original["candidates"][0];
    let bytes = format!(
        "# Preserve this reviewed ledger\nversion = 1\n[ignore.{}]\nkind = {:?}\nreason = \"Original legacy review\"\n",
        candidate["id"].as_str().unwrap(),
        candidate["kind"].as_str().unwrap()
    );
    let path = directory.path().join("rstyle-ignore.toml");
    std::fs::write(&path, &bytes).unwrap();
    let legacy = patterns_json(directory.path(), &[]);
    assert!(legacy.get("compiler_advisory").is_none());
    assert_eq!(legacy["ignored_count"], 1);
    assert_eq!(legacy["candidates"].as_array().unwrap().len(), 2);
    let explicit_legacy = patterns_json(directory.path(), &["--engine", "legacy"]);
    assert_eq!(legacy, explicit_legacy);
    let rejected = raw(directory.path(), &[]);
    checked(&rejected, 2);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("--no-ignore"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
    let bypassed = raw(directory.path(), &["--no-ignore"]);
    checked(&bypassed, 0);
    let bypassed = report(&bypassed);
    assert_eq!(
        proposals(&bypassed)["evaluation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
    std::fs::write(&path, "malformed legacy ledger").unwrap();
    checked(&raw(directory.path(), &[]), 2);
    checked(&raw(directory.path(), &["--no-ignore"]), 0);
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "malformed legacy ledger"
    );
}

#[test]
fn compiler_route_preserves_check_and_test_contexts_on_saved_replay() {
    let directory = fixture("pub fn original(){} #[cfg(test)] pub fn test_only(){}");
    let output = raw(directory.path(), &["--all-targets"]);
    checked(&output, 2);
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
    checked(&output, 2);
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

#[test]
fn compiler_only_route_flags_and_legacy_listing_reject_unsupported_combinations() {
    let directory = clone_workspace();
    for options in [
        vec!["--replay-run", "missing"],
        vec!["--review-file", "reviews.json"],
        vec!["--review-file", "reviews.json", "--review-status"],
        vec!["--locked"],
    ] {
        let mut args = vec!["patterns"];
        args.extend(options);
        let output = rstyle(directory.path(), &args);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("compiler"));
    }
    let output = raw(directory.path(), &["--show-ignored"]);
    checked(&output, 2);
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("--show-ignored"));
    assert!(message.contains("--review-status"));
}
