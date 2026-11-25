use super::propagation::{capture, checked, workspace};
use super::*;

fn live(directory: &Path, extra: &[&str], code: i32) -> Value {
    let mut args = vec![];
    args.extend_from_slice(extra);
    let (output, report) = capture(directory, &args);
    checked(&output, code);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    report
}

fn run(report: &Value) -> &Path {
    Path::new(report["run_directory"].as_str().unwrap())
}

fn index(report: &Value) -> std::path::PathBuf {
    run(report).with_file_name("advisory-owners.json")
}

fn fresh(report: &Value, name: &str, expected: bool) {
    let text = std::fs::read_to_string(run(report).join("cargo.jsonl")).unwrap();
    let records: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["reason"] == "compiler-artifact" && row["target"]["name"] == name)
        .collect();
    assert!(!records.is_empty(), "missing {name}: {text}");
    assert!(
        records.iter().all(|row| row["fresh"] == expected),
        "{records:?}"
    );
}

fn owned(report: &Value, reused: bool) {
    let functions = report["functions"].as_array().unwrap();
    assert!(!functions.is_empty());
    for function in functions {
        let bundle = Path::new(function["unit_evidence"]["bundle"].as_str().unwrap());
        assert!(bundle.starts_with(run(report)), "{}", bundle.display());
        assert!(bundle.is_file(), "{}", bundle.display());
        if reused {
            assert!(
                bundle
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("invocation-reused-")
            );
        }
    }
}

#[test]
fn completed_advisory_owners_reuse_current_fresh_artifacts_in_new_run_evidence() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let second = live(directory.path(), &[], 0);
    assert_ne!(first["run_directory"], second["run_directory"]);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);
    fresh(&first, "clone_fixture", false);
    fresh(&second, "clone_fixture", true);
    owned(&first, false);
    owned(&second, true);
    assert!(!run(&second).join("cargo-clean.stderr").exists());
}

#[test]
fn reused_advisory_owners_reevaluate_consumer_thresholds_without_compilation() {
    let directory = clone_workspace();
    let original = live(directory.path(), &[], 0);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_params]\nmax_params=0\n",
    )
    .unwrap();
    let changed = live(
        directory.path(),
        &["--compiler-advisory-config", "policy.toml"],
        0,
    );
    fresh(&changed, "clone_fixture", true);
    owned(&changed, true);
    let before = &original["compiler_advisory"]["units"][0];
    let after = &changed["compiler_advisory"]["units"][0];
    assert_eq!(before["observations"], after["observations"]);
    assert_ne!(
        before["evaluation"]["diagnostics"],
        after["evaluation"]["diagnostics"]
    );
    assert!(
        after["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["rule_id"] == "fn_params")
    );
}

/// Every saved run of a workspace, newest last.
fn saved_runs(directory: &Path) -> Vec<std::path::PathBuf> {
    let parent = directory.join("target/rstyle/runs");
    let mut runs: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_str().unwrap();
            name.starts_with("run-")
        })
        .collect();
    runs.sort_by_key(|run| {
        std::fs::metadata(run.join("plan.json"))
            .unwrap()
            .modified()
            .unwrap()
    });
    runs
}

/// Whether the newest run compiled anything of the workspace itself.
fn compiled_the_workspace(directory: &Path) -> bool {
    let newest = saved_runs(directory).pop().unwrap();
    let text = std::fs::read_to_string(newest.join("cargo.jsonl")).unwrap();
    text.lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["reason"] == "compiler-artifact")
        .filter(|row| row["package_id"].as_str().unwrap().starts_with("path+file"))
        .any(|row| row["fresh"] == false)
}

#[test]
fn every_route_shares_one_capture() {
    let directory = clone_workspace();
    let root = directory.path();
    let style = rstyle(root, &["--format", "json"]);
    assert!(compiled_the_workspace(root), "{style:?}");

    let patterns = rstyle(root, &["patterns", "--format", "json", "--no-ignore"]);
    assert_eq!(patterns.status.code(), Some(0), "{patterns:?}");
    assert!(!compiled_the_workspace(root), "patterns compiled again");

    let bounds = rstyle(
        root,
        &[
            "patterns",
            "--complexity",
            "--format",
            "json",
            "--no-ignore",
        ],
    );
    assert_eq!(bounds.status.code(), Some(0), "{bounds:?}");
    assert!(!compiled_the_workspace(root), "--complexity compiled again");

    let again = rstyle(root, &["--format", "json"]);
    assert!(!compiled_the_workspace(root), "style compiled again");
    assert_eq!(
        serde_json::from_slice::<Value>(&style.stdout).unwrap()["diagnostics"],
        serde_json::from_slice::<Value>(&again.stdout).unwrap()["diagnostics"]
    );

    // A fresh capture of the same source reports what the reused one does.
    let fresh = clone_workspace();
    let compiled = rstyle(
        fresh.path(),
        &[
            "patterns",
            "--complexity",
            "--format",
            "json",
            "--no-ignore",
        ],
    );
    let reports = [&bounds, &compiled].map(|output| {
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        report["compiler_advisory"]["units"].clone()
    });
    assert_eq!(rows(&reports[0]), rows(&reports[1]));
}

/// Each function's bounds and the candidates, by what doesn't depend on where the run is.
fn rows(units: &Value) -> Vec<Value> {
    units
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| {
            let bounds = unit["runtime_bounds"]["rows"].as_array().cloned();
            let candidates = unit["patterns"]["evaluation"]["candidates"]
                .as_array()
                .cloned();
            bounds.into_iter().chain(candidates).flatten()
        })
        .map(|mut row| {
            let fields = row.as_object_mut().unwrap();
            fields.remove("key");
            row
        })
        .collect()
}

#[test]
fn source_edits_replace_completed_advisory_owner_observations() {
    let directory = clone_workspace();
    let _first = live(directory.path(), &[], 0);
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    let changed = live(directory.path(), &[], 0);
    fresh(&changed, "clone_fixture", false);
    let observations = changed["compiler_advisory"]["units"][0]["observations"]
        .as_array()
        .unwrap();
    assert_eq!(observations.len(), 1);
    assert!(
        observations[0]["name"]
            .as_str()
            .unwrap()
            .ends_with("changed")
    );
    let again = live(directory.path(), &[], 0);
    fresh(&again, "clone_fixture", true);
    assert_eq!(changed["compiler_advisory"], again["compiler_advisory"]);
}

/// Every file of the workspace but its build output, a little newer than it was: what a fresh
/// checkout leaves, or a build directory restored onto one.
fn touch_sources(directory: &Path) {
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "target" {
                    pending.push(path);
                }
            } else {
                let file = std::fs::File::options().write(true).open(&path).unwrap();
                file.set_modified(later).unwrap();
            }
        }
    }
}

#[test]
fn a_fresh_checkout_s_file_times_keep_the_owners_it_captured() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    touch_sources(directory.path());
    let second = live(directory.path(), &[], 0);
    fresh(&second, "clone_fixture", true);
    owned(&second, true);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);

    // Content still counts.
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    let changed = live(directory.path(), &[], 0);
    fresh(&changed, "clone_fixture", false);
}

#[test]
fn a_build_directory_restored_at_another_path_keeps_the_owners_it_captured() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let original = directory.path().to_str().unwrap().to_owned();
    // What a CI cache or a fresh worker does: the same source at another path, with the build
    // directory and its runs restored into it, and nothing left where they were made.
    let elsewhere = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let moved = elsewhere.path().join("checkout");
    std::fs::create_dir(&moved).unwrap();
    copy_dir(directory.path(), &moved);
    drop(directory);

    let second = live(&moved, &[], 0);
    fresh(&second, "clone_fixture", true);
    owned(&second, true);
    let relocated = first["compiler_advisory"]
        .to_string()
        .replace(&original, moved.to_str().unwrap());
    assert_eq!(relocated, second["compiler_advisory"].to_string());
}

#[test]
fn an_index_from_another_build_of_the_tools_forces_recapture() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let path = index(&first);
    let mut written: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let tools = written["tools"].as_array_mut().unwrap();
    assert_eq!(tools.len(), 2, "{tools:?}");
    tools[1]["checksum"] = "xxh3-64:0000000000000000".into();
    std::fs::write(&path, written.to_string()).unwrap();
    let second = live(directory.path(), &[], 0);
    fresh(&second, "clone_fixture", false);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);
}

#[test]
fn missing_corrupt_and_incomplete_owner_indexes_force_recapture() {
    let directory = clone_workspace();
    let mut previous = live(directory.path(), &[], 0);
    for replacement in [None, Some("{"), Some("{\"schema\":2}")] {
        let path = index(&previous);
        match replacement {
            None => std::fs::remove_file(path).unwrap(),
            Some(text) => std::fs::write(path, text).unwrap(),
        }
        let current = live(directory.path(), &[], 0);
        fresh(&current, "clone_fixture", false);
        assert_eq!(previous["compiler_advisory"], current["compiler_advisory"]);
        previous = current;
    }
}

#[test]
fn another_report_reuses_the_capture_it_reads() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let patterns = live(directory.path(), &["--compiler-patterns"], 0);
    fresh(&patterns, "clone_fixture", true);
    let ordinary = live(directory.path(), &[], 0);
    fresh(&ordinary, "clone_fixture", true);
    assert_eq!(first["compiler_advisory"], ordinary["compiler_advisory"]);
}

#[test]
fn partial_failed_build_discards_old_owner_index_before_successful_sibling_compiles() {
    let directory = workspace("pub fn leaf() {}\n");
    let first = live(directory.path(), &[], 0);
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        "pub fn leaf() {} pub fn added() {}\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn broken() { this_does_not_exist(); }\n",
    )
    .unwrap();
    let (output, failed) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(failed["capture_complete"], false);
    fresh(&failed, "leaf", false);
    assert!(!index(&first).exists());
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn caller() { renamed::leaf(); }\n",
    )
    .unwrap();
    let recovered = live(directory.path(), &[], 0);
    fresh(&recovered, "leaf", false);
    fresh(&recovered, "caller", false);
    let leaf = super::propagation::unit(&recovered, "leaf");
    assert!(
        leaf["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["name"].as_str().unwrap().ends_with("added"))
    );
}

/// The units whose documents the run took from an earlier one.
fn reused_units(report: &Value) -> usize {
    std::fs::read_dir(run(report))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            name.to_str().unwrap().starts_with("invocation-reused-")
        })
        .count()
}

#[test]
fn a_manifest_edit_recaptures_its_package_and_reuses_the_rest() {
    let directory = workspace("pub fn leaf() {}\n");
    let first = live(directory.path(), &[], 0);
    let before = runs(&first);
    // Cargo compiles nothing differently for it, but the caller's documents record its manifest.
    let manifest = directory.path().join("caller/Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        format!("{text}[package.metadata.note]\nedited=true\n"),
    )
    .unwrap();
    let edited = live(directory.path(), &[], 0);
    assert_eq!(runs(&edited), before + 1, "one attempt");
    fresh(&edited, "leaf", true);
    fresh(&edited, "caller", false);
    assert!(reused_units(&edited) > 0);

    // The same as capturing every unit again.
    std::fs::remove_file(index(&edited)).unwrap();
    let recaptured = live(directory.path(), &[], 0);
    fresh(&recaptured, "leaf", false);
    assert_eq!(reused_units(&recaptured), 0);
    assert_eq!(edited["compiler_advisory"], recaptured["compiler_advisory"]);
}

/// Captures `directory`'s workspace in a process group of its own and, once `ready` says so, asks the group to stop,
/// as a job past its time does.
#[cfg(unix)]
fn stopped_capture(directory: &Path, ready: impl Fn() -> bool) -> Output {
    use std::os::unix::process::CommandExt;
    let child = Command::new(cargo_bin("cargo-rstyle"))
        .args(["advisory", "--format", "json", "--compiler-advisory"])
        .current_dir(directory)
        .env("RSTYLE_DRIVER", driver())
        .env_remove("CARGO_TARGET_DIR")
        .process_group(0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let group = child.id() as libc::pid_t;
    let waited = std::time::Instant::now();
    while !ready() {
        if waited.elapsed().as_secs() > 120 {
            // SAFETY: the capture's process group is the child's own, made above.
            unsafe { libc::killpg(group, libc::SIGKILL) };
            panic!("the capture never got there");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // SAFETY: as above.
    unsafe { libc::killpg(group, libc::SIGTERM) };
    child.wait_with_output().unwrap()
}

/// Every ledger the captures under `directory` published.
fn ledgers(directory: &Path) -> usize {
    let runs = directory.join("target/rstyle/runs");
    let Ok(entries) = std::fs::read_dir(runs) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|run| std::fs::read_dir(run.path()).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().unwrap().starts_with("unit-t"))
        .count()
}

#[cfg(unix)]
#[test]
fn a_capture_stopped_while_compiling_keeps_what_it_finished_for_the_next() {
    let directory = workspace("pub fn leaf() {}\n");
    // The caller's build script holds until the capture is stopped: by then the leaf, which needs no build script, is
    // captured.
    let caller = directory.path().join("caller");
    let started = directory.path().join("build-script-started");
    let script = format!(
        "fn main() {{ if std::path::Path::new(\"hold\").exists() {{ std::fs::write({started:?}, \"\").unwrap(); \
         std::thread::sleep(std::time::Duration::from_secs(600)); }} }}\n"
    );
    std::fs::write(caller.join("build.rs"), script).unwrap();
    std::fs::write(caller.join("hold"), "").unwrap();
    let stopped = stopped_capture(directory.path(), || {
        // Long enough after the leaf's ledger appears for Cargo to have reported it finished.
        started.exists() && ledgers(directory.path()) >= 1 && {
            std::thread::sleep(std::time::Duration::from_secs(2));
            true
        }
    });
    // The report says so, among the capture's failures.
    let said = String::from_utf8_lossy(&stopped.stdout);
    let kept = ledgers(directory.path());
    let message = format!("Capture stopped: kept {kept} finished owners for the next capture");
    assert!(said.contains(&message), "{:?} {said}", stopped.status);
    let runs_directory = directory.path().join("target/rstyle/runs");
    assert!(runs_directory.join("advisory-owners.json").is_file());

    std::fs::remove_file(caller.join("hold")).unwrap();
    let next = live(directory.path(), &[], 0);
    assert_eq!(runs(&next), 2, "one attempt after the stopped one");
    fresh(&next, "leaf", true);
    fresh(&next, "caller", false);
    assert_eq!(
        reused_units(&next),
        kept,
        "the leaf's owners, as the stopped capture kept them"
    );

    // The same as capturing every unit again.
    std::fs::remove_file(index(&next)).unwrap();
    let recaptured = live(directory.path(), &[], 0);
    fresh(&recaptured, "leaf", false);
    assert_eq!(next["compiler_advisory"], recaptured["compiler_advisory"]);
}

fn rustflags(directory: &Path, value: &str) {
    std::fs::create_dir_all(directory.join(".cargo")).unwrap();
    std::fs::write(
        directory.join(".cargo/config.toml"),
        format!("[build]\nrustflags=['--cfg','{value}']\n"),
    )
    .unwrap();
}

fn runs(report: &Value) -> usize {
    std::fs::read_dir(run(report).parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().unwrap().starts_with("run-"))
        .count()
}

#[test]
fn alternating_rustflags_recover_in_a_new_attempt_without_duplicate_ledgers() {
    let directory = clone_workspace();
    rustflags(directory.path(), "owner_a");
    let first = live(directory.path(), &[], 0);
    rustflags(directory.path(), "owner_b");
    let second = live(directory.path(), &[], 0);
    let before = runs(&second);
    rustflags(directory.path(), "owner_a");
    let third = live(directory.path(), &[], 0);
    assert_eq!(
        runs(&third),
        before + 2,
        "old fresh Cargo artifact requires a separate retry namespace"
    );
    fresh(&third, "clone_fixture", false);
    owned(&third, false);
    assert_eq!(first["compiler_advisory"], third["compiler_advisory"]);
    let fourth = live(directory.path(), &[], 0);
    fresh(&fourth, "clone_fixture", true);
    owned(&fourth, true);
}

#[test]
fn saved_reused_run_replays_without_original_run_sources_or_shared_builds() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let reused = live(directory.path(), &[], 0);
    fresh(&reused, "clone_fixture", true);
    owned(&reused, true);
    std::fs::remove_dir_all(run(&first)).unwrap();
    for path in ["src", "target/rstyle/runs/advisory-build"] {
        std::fs::remove_dir_all(directory.path().join(path)).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    std::fs::remove_file(index(&reused)).unwrap();
    let (output, replay) = super::reviews::replay(directory.path(), &reused, &[]);
    checked(&output, 0);
    assert_eq!(replay["capture_complete"], true);
    assert_eq!(reused["compiler_advisory"], replay["compiler_advisory"]);
}

fn limited(directory: &Path, keep: &str, extra: &[&str]) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args(["advisory", "--format", "json"])
            .args(["--compiler-advisory"])
            .args(extra)
            .current_dir(directory)
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_KEEP_RUNS", keep)
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap()
}

fn limited_report(directory: &Path, keep: &str, extra: &[&str]) -> Value {
    let output = limited(directory, keep, extra);
    checked(&output, 0);
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn runs_beyond_the_limit_are_removed_and_a_replay_of_one_says_why() {
    let directory = clone_workspace();
    let only = [];
    let first = limited_report(directory.path(), "2", &only);
    let second = limited_report(directory.path(), "2", &only);
    let third = limited_report(directory.path(), "2", &only);

    assert!(!run(&first).exists());
    assert!(run(&second).is_dir());
    assert!(run(&third).is_dir());
    let (output, replay) = super::reviews::replay(directory.path(), &second, &only);
    checked(&output, 0);
    assert_eq!(second["compiler_advisory"], replay["compiler_advisory"]);

    let gone = first["run_directory"].as_str().unwrap();
    let output = rstyle(directory.path(), &["advisory", "--replay-run", gone]);
    assert_ne!(output.status.code(), Some(0));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("RSTYLE_KEEP_RUNS"), "{error}");
}

#[test]
fn every_run_stays_when_the_limit_is_all() {
    let directory = clone_workspace();
    let first = limited_report(directory.path(), "all", &[]);
    let second = limited_report(directory.path(), "all", &[]);

    assert!(run(&first).is_dir());
    assert_eq!(runs(&second), 2);
}

#[test]
fn an_invalid_limit_is_refused_before_anything_is_removed() {
    let directory = clone_workspace();
    let first = limited_report(directory.path(), "1", &[]);

    let output = limited(directory.path(), "0", &[]);

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("RSTYLE_KEEP_RUNS"), "{error}");
    assert!(run(&first).is_dir());
    assert_eq!(runs(&first), 1);
}
