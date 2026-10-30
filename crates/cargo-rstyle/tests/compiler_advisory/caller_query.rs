use super::propagation::{capture, checked, unit, workspace};
use super::*;

fn key(report: &Value, package: &str, name: &str) -> String {
    let unit = unit(report, package);
    unit["observations"]
        .as_array()
        .unwrap()
        .iter()
        .chain(unit["generated_effects"].as_array().into_iter().flatten())
        .find(|row| row["name"].as_str().unwrap().ends_with(name))
        .unwrap()["key"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn query(report: &Value) -> &Value {
    &report["compiler_advisory"]["planning"]["caller_query"]
}

fn members(report: &Value) -> Vec<String> {
    let mut keys: Vec<_> = query(report)["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| key.as_str().unwrap().to_owned())
        .collect();
    keys.sort();
    keys
}

fn replay(directory: &Path, original: &Value, seed: &str, hops: &str) -> (Output, Value) {
    super::reviews::replay(
        directory,
        original,
        &[
            "--compiler-plans",
            "--compiler-callers",
            seed,
            "--compiler-caller-hops",
            hops,
        ],
    )
}

fn clean_fixture() -> (TempDir, Value) {
    let directory = workspace("pub fn leaf(){} pub fn unrelated(){}");
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn direct(){renamed::leaf();} pub fn transit(){direct();} pub fn unrelated(){}",
    )
    .unwrap();
    let (output, original) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    assert_eq!(
        original["compiler_advisory"]["planning"]["groups"],
        serde_json::json!([])
    );
    for path in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.path().join(path)).unwrap();
    }
    for path in ["Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    (directory, original)
}

#[test]
fn clean_cross_unit_callers_follow_saved_graph_reach_without_changing_findings() {
    let (directory, original) = clean_fixture();
    let seed = key(&original, "leaf", "leaf");
    let direct = key(&original, "caller", "direct");
    let transit = key(&original, "caller", "transit");
    for (hops, mut expected) in [
        ("0", vec![seed.clone()]),
        ("1", vec![seed.clone(), direct.clone()]),
        ("2", vec![seed.clone(), direct, transit]),
    ] {
        let (output, report) = replay(directory.path(), &original, &seed, hops);
        checked(&output, 0);
        expected.sort();
        assert_eq!(members(&report), expected);
        assert_eq!(query(&report)["seed"], seed);
        assert_eq!(
            query(&report)["unavailable_authored_facts"],
            serde_json::json!([])
        );
        let mut advisory = report["compiler_advisory"].clone();
        advisory["planning"]
            .as_object_mut()
            .unwrap()
            .remove("caller_query");
        assert_eq!(advisory, original["compiler_advisory"]);
        assert_eq!(report["matching"], original["matching"]);
    }
}

#[test]
fn generated_callers_retain_topology_without_fabricating_authored_facts() {
    let (directory, output, original) = super::capture(
        "pub fn leaf(){} macro_rules! make {()=>{pub fn generated(){leaf();}}} make!();",
        &["--compiler-plans"],
    );
    checked(&output, 2);
    let seed = key(&original, "", "leaf");
    let generated = key(&original, "", "generated");
    let (output, report) = replay(directory.path(), &original, &seed, "1");
    checked(&output, 2);
    let mut expected = vec![seed, generated.clone()];
    expected.sort();
    assert_eq!(members(&report), expected);
    assert_eq!(
        query(&report)["unavailable_authored_facts"],
        serde_json::json!([generated])
    );
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn missing_dependency_targets_do_not_turn_a_known_query_into_complete_scope() {
    let directory = workspace("pub fn leaf(){}");
    let (output, original) = clones(
        directory.path(),
        &[
            "-p",
            "caller",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
        ],
    );
    checked(&output, 2);
    let seed = key(&original, "caller", "caller");
    let (output, report) = replay(directory.path(), &original, &seed, "2");
    checked(&output, 2);
    assert_eq!(members(&report), vec![seed]);
    assert_eq!(
        report["compiler_advisory"]["planning"]["call_graph_complete"],
        false
    );
    assert_eq!(
        unit(&report, "caller")["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
}

#[test]
fn caller_query_rejects_missing_scope_and_malformed_report_ids() {
    let directory = clone_workspace();
    for args in [
        vec!["--compiler-callers", "function:u0:f0"],
        vec!["--compiler-caller-hops", "1"],
        vec![
            "--compiler-callers",
            "function:u0:f0",
            "--compiler-plans",
            "--compiler-advisory",
        ],
    ] {
        let output = rstyle(directory.path(), &[&["clones"][..], &args].concat());
        checked(&output, 2);
    }
    for seed in [
        "function:u0:f",
        "function:u-1:f0",
        "function:u0:f+1",
        "function:u0:f4294967296",
        "function:u0:f0:extra",
    ] {
        let output = rstyle(directory.path(), &["clones", "--compiler-callers", seed]);
        checked(&output, 2);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Expected a saved-report function ID")
        );
    }
}

#[test]
fn out_of_scope_seed_is_an_error_instead_of_an_empty_result() {
    let (directory, _, original) = super::capture("pub fn empty(){}", &["--compiler-plans"]);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
            "--compiler-callers",
            "function:u999:f999",
        ],
    );
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Caller query seed is unavailable"));
}

#[test]
fn caller_query_replays_compact_and_text_without_sources_or_compiler_tools() {
    let (directory, original) = clean_fixture();
    let seed = key(&original, "leaf", "leaf");
    let (output, expected) = replay(directory.path(), &original, &seed, "1");
    checked(&output, 0);
    for format in ["json", "text"] {
        let output = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory.path())
            .args([
                "clones",
                "--format",
                format,
                "--report-detail",
                "compact",
                "--extraction-profile",
                "hir-data-v3",
                "--compiler-advisory",
                "--compiler-plans",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--compiler-callers",
                &seed,
            ])
            .env("PATH", "/no-tools")
            .env("CARGO", "/no-cargo")
            .env("RUSTC", "/no-rustc")
            .env("RSTYLE_DRIVER", "/no-driver")
            .output()
            .unwrap();
        checked(&output, 0);
        if format == "json" {
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(query(&report), query(&expected));
        } else {
            assert!(stdout(&output).contains("Known potential caller query:"));
            assert!(stdout(&output).contains(&seed));
            assert!(stdout(&output).contains("2 members"));
        }
    }
}

#[test]
fn ambiguous_dependency_contexts_never_merge_namesake_callers() {
    let directory = workspace("pub fn leaf(){}");
    let (output, original) = capture(directory.path(), &["--all-targets", "--compiler-plans"]);
    checked(&output, 2);
    let seed = key(&original, "leaf", "leaf");
    let (output, report) = replay(directory.path(), &original, &seed, "2");
    checked(&output, 2);
    assert_eq!(members(&report), vec![seed]);
    assert_eq!(
        report["compiler_advisory"]["planning"]["call_graph_complete"],
        false
    );
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}
