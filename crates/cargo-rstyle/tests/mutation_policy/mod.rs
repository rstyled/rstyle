use super::*;
mod exclusions;
mod replay;
mod suppressions;

const WRITES: &str = "pub fn many() -> u32 { let mut first=0u32; let mut second=0u32; first=1; first=2; first=3; second=1; second=2; second=3; first + second }\n";
const OPAQUE: &str = "pub fn many(callback: fn()) { callback(); }\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    policy(directory.path(), &[]);
    directory
}

fn policy(directory: &Path, excluded: &[&str]) {
    let exclude = serde_json::to_string(excluded).unwrap();
    std::fs::write(directory.join("mutation.toml"),format!(
        "exclude={exclude}\n[mutation_budget]\nmax_mutations_per_target=2\nmax_mutated_targets=1\n"
    )).unwrap();
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--mutation-rules",
        "--mutation-config",
        "mutation.toml",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn function(report: &Value) -> &Value {
    let functions = report["mutation_analysis"]["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 1);
    &functions[0]
}

fn decision<'a>(function: &'a Value, id: &str) -> &'a Value {
    function["policy"]["rule_decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decision| decision["rule_id"] == id)
        .unwrap()
}

fn assert_raw_same(left: &Value, right: &Value) {
    for field in [
        "evidence",
        "rules",
        "coverage",
        "execution",
        "availability",
        "outcome",
        "density",
    ] {
        assert_eq!(left[field], right[field], "raw {field}");
    }
}

fn counts(report: &Value, raw: usize, visible: usize, suppressed: usize) {
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["schema"], "rstyle.mutations/2");
    assert_eq!(analysis["findings"], raw);
    assert_eq!(analysis["visible_findings"], visible);
    assert_eq!(analysis["suppressed_findings"], suppressed);
}

fn replay_command(directory: &Path, original: &Value, format: &str) -> Command {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            format,
            "--extraction-profile",
            "hir-data-v3",
            "--mutation-rules",
            "--mutation-config",
            "mutation.toml",
            "--fail-on-findings",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    command
}

fn replay(directory: &Path, original: &Value, detail: &str) -> (Output, Value) {
    let output = diagnostic_process::output(
        replay_command(directory, original, "json").args(["--report-detail", detail]),
    )
    .unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}
