use super::*;

const PAIR: &str =
    "pub fn first(value:u32)->u32 { (value+1)*2 }\npub fn second(input:u32)->u32 { (input+3)*2 }\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn captured(directory: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["--extraction-profile", "hir-data-v3"];
    args.extend_from_slice(extra);
    let (output, report) = clones(directory, &args);
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["capture_complete"], true);
    assert!(
        !Path::new(report["run_directory"].as_str().unwrap())
            .join("dependency-closure.json")
            .exists()
    );
    report
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> Output {
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

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn assert_frozen(original: &Value, replayed: &Value) {
    assert_eq!(replayed["verification_policy"], "strict-replay");
    for field in [
        "functions",
        "build_contexts",
        "owned_source_inventory",
        "matching",
        "observed_units",
        "expected_compilation_units",
    ] {
        assert_eq!(original[field], replayed[field], "changed {field}");
    }
    assert_eq!(replayed["assessment_complete"], true);
    assert_eq!(replayed["pair_evaluation_complete"], true);
}

#[test]
fn standalone_replay_changes_source_policy_without_recapture_or_source_files() {
    let directory = workspace(PAIR);
    let original = captured(directory.path(), &[]);
    let policy = directory.path().join("analysis.toml");
    std::fs::write(
        &policy,
        "[fn_length]\nmax_lines=50\n[fn_params]\nmax_params=4\n",
    )
    .unwrap();
    let args = [
        "--source-rules",
        "--analysis-config",
        policy.to_str().unwrap(),
        "--fail-on-findings",
    ];
    let output = replay(directory.path(), &original, &args);
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    let loose = report(&output);
    assert_frozen(&original, &loose);
    assert_eq!(loose["analysis"]["findings"], 0);
    std::fs::write(
        &policy,
        "[fn_length]\nmax_lines=0\n[fn_params]\nmax_params=0\n",
    )
    .unwrap();
    for deleted in [false, true] {
        change_original_sources(directory.path(), deleted);
        let output = replay(directory.path(), &original, &args);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let strict = report(&output);
        assert_frozen(&original, &strict);
        assert_eq!(strict["analysis"]["findings"], 4);
        assert_eq!(strict["analysis"]["scope"]["coverage"], "complete");
    }
}

fn change_original_sources(directory: &Path, delete: bool) {
    for file in ["Cargo.toml", "src/lib.rs"] {
        let path = directory.join(file);
        if delete {
            std::fs::remove_file(path).unwrap();
        } else {
            std::fs::write(path, "changed and deliberately invalid Rust/Cargo source").unwrap();
        }
    }
}

#[test]
fn helper_and_guard_consumers_can_be_requested_only_at_replay_time() {
    for (source, flag, field) in [
        (PAIR, "--helper-proposals", "helper_proposals"),
        (
            "pub fn guard(value:u32)->u32 { if value>0 { let next=value+1; next+1 } else { 0 } }",
            "--guard-clauses",
            "analysis",
        ),
    ] {
        let directory = workspace(source);
        let original = captured(directory.path(), &[]);
        assert_eq!(original[field]["execution"], "not_requested");
        change_original_sources(directory.path(), true);
        let output = replay(directory.path(), &original, &[flag, "--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let replayed = report(&output);
        assert_frozen(&original, &replayed);
        assert_eq!(replayed[field]["findings"], 1);
        assert_eq!(replayed[field]["execution"], "completed");
    }
}

#[test]
fn all_target_capture_replays_original_check_and_test_units_without_new_selection() {
    let directory = workspace(
        "pub fn public(value:u32)->u32 { value+1 }\n#[cfg(test)] fn test_only(value:u32)->u32 { value+2 }\n",
    );
    let original = captured(directory.path(), &["--all-targets"]);
    let units = original["observed_units"].as_array().unwrap();
    assert!(units.iter().any(|unit| unit["test"] == true));
    assert!(units.iter().any(|unit| unit["test"] == false));
    change_original_sources(directory.path(), true);
    let output = replay(directory.path(), &original, &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    let replayed = report(&output);
    assert_frozen(&original, &replayed);
    assert_eq!(replayed["scope"]["selection_origin"], "saved_run");
    assert_eq!(replayed["scope"]["all_targets"], Value::Null);
    assert!(
        replayed["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| {
                function["facts"]["name"]
                    .as_str()
                    .unwrap()
                    .ends_with("test_only")
            })
    );
}

#[test]
fn replay_rejects_new_population_and_build_options() {
    let directory = workspace(PAIR);
    let original = captured(directory.path(), &[]);
    for extra in [
        vec!["--manifest-path", "Cargo.toml"],
        vec!["--package", "clone_fixture"],
        vec!["--all-targets"],
        vec!["--features", "extra"],
        vec!["--release"],
        vec!["--locked"],
        vec!["--target", "unavailable-target"],
    ] {
        assert_rejected(&replay(directory.path(), &original, &extra));
    }
}

fn assert_rejected(output: &Output) {
    assert_eq!(output.status.code(), Some(2), "{}", stdout(output));
    assert!(!output.stderr.is_empty() || !output.stdout.is_empty());
    if let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) {
        assert!(value["assessment_complete"] != true || value["capture_complete"] != true);
    }
}

#[test]
fn standalone_replay_refuses_incomplete_missing_and_changed_saved_evidence() {
    let directory = workspace(PAIR);
    let original = captured(directory.path(), &[]);
    let run = Path::new(original["run_directory"].as_str().unwrap());
    let cargo_path = run.join("cargo.jsonl");
    let cargo = std::fs::read(&cargo_path).unwrap();
    let changed = failed_cargo_completion(&cargo);
    std::fs::write(&cargo_path, changed).unwrap();
    assert_rejected(&replay(directory.path(), &original, &[]));
    let missing_completion: Vec<_> = std::str::from_utf8(&cargo)
        .unwrap()
        .lines()
        .filter(|line| serde_json::from_str::<Value>(line).unwrap()["reason"] != "build-finished")
        .collect();
    std::fs::write(&cargo_path, missing_completion.join("\n") + "\n").unwrap();
    assert_rejected(&replay(directory.path(), &original, &[]));
    std::fs::write(&cargo_path, &cargo).unwrap();
    corrupt_ledger(directory.path(), &original, run);
    let bundle = Path::new(
        original["functions"][0]["unit_evidence"]["bundle"]
            .as_str()
            .unwrap(),
    );
    let bytes = std::fs::read(bundle).unwrap();
    std::fs::write(bundle, b"changed saved evidence").unwrap();
    assert_rejected(&replay(directory.path(), &original, &[]));
    std::fs::write(bundle, bytes).unwrap();
    let output = replay(directory.path(), &original, &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn failed_cargo_completion(bytes: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).unwrap();
    let mut found = false;
    let lines: Vec<_> = text
        .lines()
        .map(|line| {
            let mut record: Value = serde_json::from_str(line).unwrap();
            if record["reason"] == "build-finished" {
                record["success"] = false.into();
                found = true;
            }
            serde_json::to_string(&record).unwrap()
        })
        .collect();
    assert!(found, "saved Cargo log has no completion record");
    (lines.join("\n") + "\n").into_bytes()
}

fn corrupt_ledger(directory: &Path, original: &Value, run: &Path) {
    let ledger = std::fs::read_dir(run)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("unit-t")
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
        })
        .unwrap();
    let bytes = std::fs::read(&ledger).unwrap();
    std::fs::remove_file(&ledger).unwrap();
    assert_rejected(&replay(directory, original, &[]));
    let mut changed: Value = serde_json::from_slice(&bytes).unwrap();
    changed["invocation"]
        .as_array_mut()
        .unwrap()
        .push("--cfg=independent_changed_context".into());
    std::fs::write(&ledger, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_rejected(&replay(directory, original, &[]));
    std::fs::write(ledger, bytes).unwrap();
}
