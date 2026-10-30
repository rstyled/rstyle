use super::*;

const PAIR: &str =
    "pub fn first(value:u32)->u32 { (value+1)*2 }\npub fn second(input:u32)->u32 { (input+3)*2 }\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["--extraction-profile", "hir-data-v3", "--helper-proposals"];
    args.extend_from_slice(extra);
    let (output, report) = clones(directory, &args);
    assert!(output.status.success(), "{}", stdout(&output));
    report
}

fn key<'a>(report: &'a Value, suffix: &str) -> &'a str {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["facts"]["name"]
                .as_str()
                .unwrap()
                .ends_with(suffix)
        })
        .unwrap()["key"]
        .as_str()
        .unwrap()
}

fn replay(directory: &Path, original: &Value, file: &Path, extra: &[&str]) -> Output {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let mut command = Command::new(executable);
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--review-file",
            file.to_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn dismiss(directory: &Path, original: &Value, file: &Path, pair: [&str; 2]) {
    let output = replay(
        directory,
        original,
        file,
        &[
            "--dismiss-helper-pair",
            key(original, pair[0]),
            key(original, pair[1]),
            "--review-reason",
            "Reviewed this bounded repeated computation.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(
        parsed(&output)["review"]["pairs"][0]["state"],
        "hidden_exact"
    );
}

fn assert_counts(report: &Value, findings: usize, hidden: usize, visible: usize) {
    assert_eq!(report["review"]["execution"], "completed");
    assert_eq!(report["review"]["findings"], findings);
    assert_eq!(report["review"]["hidden_findings"], hidden);
    assert_eq!(report["review"]["visible_findings"], visible);
    assert_eq!(report["helper_proposals"]["findings"], findings);
}

#[test]
fn explicit_saved_pair_dismissal_is_read_only_during_source_free_replay() {
    let directory = workspace(PAIR);
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    let output = replay(directory.path(), &original, &file, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        parsed(&output)["review"]["pairs"][0]["state"],
        "visible_new"
    );
    assert!(!file.exists(), "lookup must not create a review store");
    dismiss(directory.path(), &original, &file, ["first", "second"]);
    let bytes = std::fs::read(&file).unwrap();
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = replay(directory.path(), &original, &file, &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_eq!(report["verification_policy"], "strict-replay");
    assert_eq!(report["review"]["coverage"], "complete");
    assert_eq!(report["review"]["pairs"][0]["state"], "hidden_exact");
    assert_counts(&report, 1, 1, 0);
    assert_eq!(bytes, std::fs::read(file).unwrap());
}

#[test]
fn review_survives_line_and_binder_changes_but_relevant_literal_edit_is_stale() {
    let directory = workspace(PAIR);
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    dismiss(directory.path(), &original, &file, ["first", "second"]);
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"),
        "\n\n// line movement is presentation\npub fn first(renamed:u32)->u32 { (renamed+1)*2 }\npub fn second(changed:u32)->u32 { (changed+3)*2 }\n").unwrap();
    let moved = capture(directory.path(), &[]);
    let output = replay(directory.path(), &moved, &file, &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_counts(&report, 1, 1, 0);
    assert_eq!(report["review"]["pairs"][0]["state"], "hidden_exact");
    std::fs::write(
        directory.path().join("src/lib.rs"),
        PAIR.replace("input+3", "input+4"),
    )
    .unwrap();
    let edited = capture(directory.path(), &[]);
    let output = replay(directory.path(), &edited, &file, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = parsed(&output);
    assert_counts(&report, 1, 0, 1);
    assert_eq!(report["review"]["pairs"][0]["state"], "stale");
    assert_eq!(bytes, std::fs::read(file).unwrap());
}

#[test]
fn changed_feature_context_does_not_inherit_saved_dismissal() {
    let directory = workspace(PAIR);
    let manifest = directory.path().join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text + "\n[features]\nextra=[]\n").unwrap();
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    dismiss(directory.path(), &original, &file, ["first", "second"]);
    let changed = capture(directory.path(), &["--features", "extra"]);
    let output = replay(directory.path(), &changed, &file, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = parsed(&output);
    assert_counts(&report, 1, 0, 1);
    assert_eq!(report["review"]["pairs"][0]["state"], "stale");
}

#[test]
fn same_leaf_names_in_distinct_modules_keep_separate_review_entries() {
    let source = format!(
        "pub mod left {{ {PAIR} }}\npub mod right {{ pub fn first(value:u32)->u32 {{ (value^1)&15 }} pub fn second(input:u32)->u32 {{ (input^3)&15 }} }}\n"
    );
    let directory = workspace(&source);
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    let output = replay(
        directory.path(),
        &original,
        &file,
        &[
            "--dismiss-helper-pair",
            key(&original, "left::first"),
            key(&original, "left::second"),
            "--review-reason",
            "Only the left module pair was reviewed.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_counts(&report, 2, 1, 1);
    let right = [
        key(&original, "right::first"),
        key(&original, "right::second"),
    ];
    let row = report["review"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            right.contains(&pair["first"].as_str().unwrap())
                && right.contains(&pair["second"].as_str().unwrap())
        })
        .unwrap();
    assert_eq!(row["state"], "visible_new");
}

#[test]
fn unsupported_owner_keeps_review_incomplete_without_pruning_saved_entries() {
    let directory = workspace(PAIR);
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    dismiss(directory.path(), &original, &file, ["first", "second"]);
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!(
            "{PAIR}\npub fn opaque(value:u32,callback:fn(u32)->u32)->u32 {{ callback(value) }}"
        ),
    )
    .unwrap();
    let (output, mixed) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert_eq!(output.status.code(), Some(2));
    let output = replay(directory.path(), &mixed, &file, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    assert_eq!(report["review"]["coverage"], "incomplete");
    assert_counts(&report, 1, 1, 0);
    assert_eq!(bytes, std::fs::read(file).unwrap());
}

#[test]
fn invalid_dismissal_and_store_fields_fail_without_writes() {
    let directory = workspace(PAIR);
    let original = capture(directory.path(), &[]);
    let file = directory.path().join("review.json");
    for (selector, reason) in [
        ("function:u999:f999", "reviewed"),
        (key(&original, "first"), ""),
    ] {
        let output = replay(
            directory.path(),
            &original,
            &file,
            &[
                "--dismiss-helper-pair",
                selector,
                key(&original, "second"),
                "--review-reason",
                reason,
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(!file.exists());
    }
    dismiss(directory.path(), &original, &file, ["first", "second"]);
    let bytes = std::fs::read(&file).unwrap();
    let mut invalid: Value = serde_json::from_slice(&bytes).unwrap();
    invalid["unrecognized_review_authority"] = true.into();
    let invalid = serde_json::to_vec(&invalid).unwrap();
    std::fs::write(&file, &invalid).unwrap();
    let output = replay(directory.path(), &original, &file, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(invalid, std::fs::read(&file).unwrap());
    std::fs::write(&file, bytes).unwrap();
}

#[test]
fn completed_helpers_without_unique_free_function_locators_remain_unreviewable() {
    for source in [
        format!("pub struct Owner; impl Owner {{ {PAIR} }}"),
        format!("#[cfg(any())] pub fn first(value:u32)->u32 {{ (value+1)*2 }}\n{PAIR}"),
    ] {
        let directory = workspace(&source);
        let original = capture(directory.path(), &[]);
        let file = directory.path().join("review.json");
        let output = replay(directory.path(), &original, &file, &["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
        let report = parsed(&output);
        assert_eq!(report["helper_proposals"]["coverage"], "complete");
        assert_eq!(report["review"]["coverage"], "incomplete");
        assert_counts(&report, 1, 0, 1);
        assert_eq!(report["review"]["pairs"][0]["state"], "unavailable");
        assert!(!file.exists());
    }
}
