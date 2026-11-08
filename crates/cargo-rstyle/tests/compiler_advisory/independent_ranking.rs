use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn straight(){} pub fn nested(a:bool,b:bool){if a {if b {}}}";
const RANK: &[&str] = &["--compiler-complexity-top", "10"];

fn capture_rank(source: &str, extra: &[&str]) -> (TempDir, Output, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let mut args = RANK.to_vec();
    args.extend_from_slice(extra);
    let (output, report) = advisory(directory.path(), &args);
    (directory, output, report)
}

fn replay_rank(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = RANK.to_vec();
    args.extend(["--replay-run", original["run_directory"].as_str().unwrap()]);
    args.extend_from_slice(extra);
    advisory(directory, &args)
}

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn rank_only(report: &Value) {
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["diagnostic_execution"], "not_requested");
    assert_eq!(advisory["effect_execution"], "not_requested");
    assert_eq!(advisory["findings"], 0);
    assert!(advisory.get("planning").is_none());
    for unit in advisory["units"].as_array().unwrap() {
        assert!(unit.get("evaluation").is_none());
        assert!(unit.get("patterns").is_none());
    }
}

#[test]
fn ranking_only_completes_without_resolving_unselected_workspace_callees() {
    let directory = propagation::workspace("pub fn leaf(){} ");
    let mut args = RANK.to_vec();
    args.extend(["--package", "caller"]);
    let (output, report) = advisory(directory.path(), &args);
    checked(&output, 0);
    rank_only(&report);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    let caller = propagation::unit(&report, "caller");
    assert_eq!(caller["complexity_ranking"]["input_complete"], true);
    assert_eq!(caller["complexity_ranking"]["measured_functions"], 1);
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    let (output, numeric) = replay_rank(directory.path(), &report, &["--compiler-advisory"]);
    checked(&output, 2);
    assert_eq!(
        numeric["compiler_advisory"]["effect_execution"],
        "completed"
    );
    assert_eq!(numeric["compiler_advisory"]["complete"], false);
    assert_eq!(
        propagation::unit(&numeric, "caller")["complexity_ranking"],
        caller["complexity_ranking"]
    );
}

#[test]
fn standalone_ranking_reuses_exclusions_and_preserves_combined_numeric_findings() {
    let source = "pub fn many(a:u128,b:u128,c:u128,d:u128,e:u128){} pub fn nested(a:bool,b:bool){if a {if b {}}}";
    let (directory, output, ranked) = capture_rank(source, &[]);
    checked(&output, 0);
    rank_only(&ranked);
    let (output, numeric) = replay_rank(directory.path(), &ranked, &["--compiler-advisory"]);
    checked(&output, 0);
    assert_eq!(
        unit(&numeric)["complexity_ranking"],
        unit(&ranked)["complexity_ranking"]
    );
    assert!(numeric["compiler_advisory"]["findings"].as_u64().unwrap() > 0);
    assert_eq!(
        numeric["compiler_advisory"]["effect_execution"],
        "completed"
    );
    let (output, plain) = reviews::replay(directory.path(), &ranked, &[]);
    checked(&output, 0);
    assert_eq!(unit(&numeric)["evaluation"], unit(&plain)["evaluation"]);
    assert_eq!(
        numeric["compiler_advisory"]["findings"],
        plain["compiler_advisory"]["findings"]
    );
    std::fs::write(
        directory.path().join("excluded.toml"),
        "exclude=['src/**']\n",
    )
    .unwrap();
    let (output, excluded) = replay_rank(
        directory.path(),
        &ranked,
        &["--compiler-advisory-config", "excluded.toml"],
    );
    checked(&output, 0);
    rank_only(&excluded);
    assert_eq!(unit(&excluded)["complexity_ranking"]["known_functions"], 0);
    assert_eq!(
        unit(&excluded)["complexity_ranking"]["functions"],
        serde_json::json!([])
    );
    assert_eq!(
        unit(&excluded)["complexity_ranking"]["input_complete"],
        true
    );
}

#[test]
fn standalone_ranking_replays_without_sources_or_compiler_tools() {
    let (directory, output, original) = capture_rank(SOURCE, &[]);
    checked(&output, 0);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .arg("advisory")
        .args(RANK)
        .args([
            "--format",
            "json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.path().join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 0);
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    rank_only(&replayed);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn standalone_ranking_retains_each_compilation_context() {
    let source = "pub fn common(a:bool){if a {}} #[cfg(test)] pub fn test_only(a:bool,b:bool){if a {if b {}}}";
    let (_directory, _output, report) = capture_rank(source, &["--all-targets"]);
    rank_only(&report);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    let mut counts: Vec<_> = units
        .iter()
        .map(|unit| {
            unit["complexity_ranking"]["measured_functions"]
                .as_u64()
                .unwrap()
        })
        .collect();
    counts.sort_unstable();
    assert_eq!(counts, [1, 2]);
    let mut complete = true;
    for unit in units {
        let ranking = &unit["complexity_ranking"];
        let available =
            ranking["unavailable_owners"] == 0 && ranking["unavailable_measurements"] == 0;
        assert_eq!(ranking["input_complete"], available);
        complete &= available;
    }
    assert_eq!(report["compiler_advisory"]["complete"], complete);
}
