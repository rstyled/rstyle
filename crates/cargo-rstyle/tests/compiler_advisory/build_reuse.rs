use super::propagation::{capture, checked, workspace};
use super::*;

fn live(directory: &Path, extra: &[&str], code: i32) -> Value {
    let mut args = vec!["--compiler-advisory-only"];
    args.extend_from_slice(extra);
    let (output, report) = capture(directory, &args);
    checked(&output, code);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    report
}

fn artifacts(report: &Value) -> Vec<Value> {
    let run = Path::new(report["run_directory"].as_str().unwrap());
    std::fs::read_to_string(run.join("cargo.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|record| record["reason"] == "compiler-artifact")
        .collect()
}

fn fresh(records: &[Value], name: &str, expected: bool) {
    let matches: Vec<_> = records
        .iter()
        .filter(|r| r["target"]["name"] == name)
        .collect();
    assert!(!matches.is_empty(), "missing {name}");
    assert!(
        matches.iter().all(|r| r["fresh"] == expected),
        "{matches:?}"
    );
}

fn owned(report: &Value) {
    let root = Path::new(report["run_directory"].as_str().unwrap());
    for function in report["functions"].as_array().unwrap() {
        let bundle = Path::new(function["unit_evidence"]["bundle"].as_str().unwrap());
        assert!(bundle.starts_with(root), "{}", bundle.display());
    }
}

#[test]
fn advisory_builds_reuse_dependencies_and_owners_but_refresh_changed_source() {
    let directory = workspace("pub fn leaf() {}\n");
    let first = live(directory.path(), &["-p", "caller"], 2);
    let second = live(directory.path(), &["-p", "caller"], 2);
    assert_ne!(first["run_directory"], second["run_directory"]);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);
    fresh(&artifacts(&first), "leaf", false);
    fresh(&artifacts(&second), "leaf", true);
    fresh(&artifacts(&second), "caller", true);
    owned(&first);
    owned(&second);

    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn changed() {}\n",
    )
    .unwrap();
    let changed = live(directory.path(), &["-p", "caller"], 0);
    let observations = &changed["compiler_advisory"]["units"][0]["observations"];
    assert_eq!(observations.as_array().unwrap().len(), 1);
    assert!(
        observations[0]["name"]
            .as_str()
            .unwrap()
            .ends_with("changed")
    );
    fresh(&artifacts(&changed), "leaf", true);
    fresh(&artifacts(&changed), "caller", false);
}

fn feature_workspace() -> TempDir {
    let directory = clone_workspace();
    let manifest = directory.path().join("Cargo.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str("[features]\nextra=[]\n");
    std::fs::write(manifest, text).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn plain() {}\n#[cfg(feature=\"extra\")] pub fn featured() {}\n#[cfg(test)] fn test_only() {}\n").unwrap();
    directory
}

#[test]
fn advisory_builds_refresh_features_collectors_and_check_test_contexts() {
    let directory = feature_workspace();
    let ordinary = live(directory.path(), &[], 0);
    let all = live(
        directory.path(),
        &[
            "--all-targets",
            "--features",
            "extra",
            "--compiler-patterns",
        ],
        2,
    );
    let again = live(directory.path(), &[], 0);
    let test_first = live(directory.path(), &["--all-targets"], 2);
    assert_eq!(ordinary["compiler_advisory"], again["compiler_advisory"]);
    assert_eq!(
        ordinary["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        all["compiler_advisory"]["units"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        test_first["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let names = all["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|u| u["observations"].as_array().unwrap())
        .map(|o| o["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(names.iter().any(|n| n.ends_with("featured")));
    assert!(names.iter().any(|n| n.ends_with("test_only")));
    for report in [&ordinary, &all, &again, &test_first] {
        fresh(&artifacts(report), "clone_fixture", false);
        owned(report);
    }
}

#[test]
fn advisory_build_lane_serializes_concurrent_captures_with_separate_run_owners() {
    let directory = workspace("pub fn leaf() {}\n");
    let _warm = live(directory.path(), &["-p", "caller"], 2);
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| live(directory.path(), &["-p", "caller"], 2));
        let second = scope.spawn(|| live(directory.path(), &["-p", "caller"], 2));
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_ne!(first["run_directory"], second["run_directory"]);
    assert_eq!(first["compiler_advisory"], second["compiler_advisory"]);
    for report in [&first, &second] {
        owned(report);
        fresh(&artifacts(report), "leaf", true);
        fresh(&artifacts(report), "caller", true);
    }
}

#[test]
fn advisory_saved_replay_survives_removal_of_shared_builds_and_sources() {
    let directory = clone_workspace();
    let original = live(directory.path(), &[], 0);
    std::fs::remove_dir_all(directory.path().join("target/rstyle/clones/advisory-build")).unwrap();
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, replay) =
        super::reviews::replay(directory.path(), &original, &["--compiler-advisory-only"]);
    checked(&output, 0);
    assert_eq!(replay["compiler_advisory"], original["compiler_advisory"]);
    assert_eq!(replay["capture_complete"], true);
}

#[test]
fn strict_capture_keeps_its_private_build_after_advisory_reuse() {
    let directory = clone_workspace();
    let advisory = live(directory.path(), &[], 0);
    let (output, strict) = capture(directory.path(), &[]);
    checked(&output, 0);
    let run = Path::new(strict["run_directory"].as_str().unwrap());
    assert!(run.join("build").is_dir());
    assert!(!run.join("cargo-clean.stderr").exists());
    assert_ne!(strict["strict_assessment_execution"], "not_requested");
    assert_eq!(advisory["compiler_advisory"], strict["compiler_advisory"]);
}

#[test]
fn advisory_dependency_capture_keeps_its_private_build() {
    let directory = workspace("pub fn leaf() {}\n");
    let report = live(
        directory.path(),
        &["-p", "caller", "--capture-dependency", "leaf"],
        0,
    );
    let run = Path::new(report["run_directory"].as_str().unwrap());
    assert!(run.join("build").is_dir());
    assert!(!run.join("cargo-clean.stderr").exists());
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(report["strict_assessment_execution"], "not_requested");
}

#[test]
fn advisory_builds_refresh_host_target_and_release_variants() {
    let directory = clone_workspace();
    let first = live(directory.path(), &[], 0);
    let plan: Value = serde_json::from_slice(
        &std::fs::read(Path::new(first["run_directory"].as_str().unwrap()).join("plan.json"))
            .unwrap(),
    )
    .unwrap();
    let host = plan["compiler_identity"]
        .as_str()
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    let target = live(directory.path(), &["--target", host], 0);
    let release = live(directory.path(), &["--release"], 0);
    let debug = live(directory.path(), &[], 0);
    let target_again = live(directory.path(), &["--target", host], 0);
    assert_eq!(first["compiler_advisory"], debug["compiler_advisory"]);
    assert_eq!(
        target["compiler_advisory"],
        target_again["compiler_advisory"]
    );
    for report in [&first, &target, &release, &debug, &target_again] {
        fresh(&artifacts(report), "clone_fixture", false);
        owned(report);
    }
    std::fs::create_dir(directory.path().join(".cargo")).unwrap();
    std::fs::write(
        directory.path().join(".cargo/config.toml"),
        format!("[build]\ntarget='{host}'\n"),
    )
    .unwrap();
    let configured = live(directory.path(), &[], 0);
    let configured_again = live(directory.path(), &[], 0);
    assert_eq!(
        configured["compiler_advisory"],
        configured_again["compiler_advisory"]
    );
    fresh(&artifacts(&configured_again), "clone_fixture", true);
}

#[test]
fn advisory_builds_refresh_host_proc_macros_with_configured_target() {
    let directory = clone_workspace();
    let manifest = directory.path().join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap() + "[lib]\nproc-macro=true\n";
    std::fs::write(manifest, text).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), "extern crate proc_macro;\n#[proc_macro] pub fn pass(input: proc_macro::TokenStream) -> proc_macro::TokenStream { input }\n").unwrap();
    let first = live(directory.path(), &[], 0);
    let run = Path::new(first["run_directory"].as_str().unwrap());
    let plan: Value =
        serde_json::from_slice(&std::fs::read(run.join("plan.json")).unwrap()).unwrap();
    let host = plan["compiler_identity"]
        .as_str()
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    std::fs::create_dir(directory.path().join(".cargo")).unwrap();
    std::fs::write(
        directory.path().join(".cargo/config.toml"),
        format!("[build]\ntarget='{host}'\n"),
    )
    .unwrap();
    let configured = live(directory.path(), &[], 0);
    let again = live(directory.path(), &[], 0);
    assert_eq!(first["compiler_advisory"], configured["compiler_advisory"]);
    assert_eq!(configured["compiler_advisory"], again["compiler_advisory"]);
    fresh(&artifacts(&again), "clone_fixture", true);
    owned(&again);
}
