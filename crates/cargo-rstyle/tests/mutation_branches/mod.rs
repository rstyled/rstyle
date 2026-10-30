use super::*;
mod boundaries;
mod live;
mod replay;

const BRANCH: &str = "pub fn branch(target: &mut u32, flag: bool) { if flag { *target = 1; } else { *target = 2; } }\n";
const SHADOWED: &str =
    "pub fn shadow()->u32 { let mut value=0u32; value=1; { let mut value=2u32; value=3; } value }";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    policy(directory.path(), 2);
    directory
}

fn policy(directory: &Path, maximum: usize) {
    std::fs::write(
        directory.join("mutation.toml"),
        format!("[mutation_budget]\nmax_mutations_per_target={maximum}\nmax_mutated_targets=1\n"),
    )
    .unwrap();
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--mutation-rules",
        "--mutation-config",
        "mutation.toml",
    ];
    arguments.extend_from_slice(extra);
    clones(directory, &arguments)
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["mutation_analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn rule<'a>(function: &'a Value, id: &str) -> &'a Value {
    function["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["rule_id"] == id)
        .unwrap()
}

fn slice<'a>(source: &'a str, span: &Value) -> &'a str {
    let start = span["start"].as_u64().unwrap() as usize;
    let end = span["end"].as_u64().unwrap() as usize;
    source
        .get(start..end)
        .unwrap_or_else(|| panic!("invalid span {span} in {source}"))
}

struct Counts {
    sites: u64,
    roots: u64,
    escaped: u64,
}

fn assert_exact(function: &Value, expected: Counts) {
    assert_eq!(function["coverage"], "complete", "{function}");
    assert_eq!(function["availability"], "known");
    let evidence = &function["evidence"];
    for (metric, expected) in [
        ("write_sites", expected.sites),
        ("mutated_roots", expected.roots),
        ("escaped_write_sites", expected.escaped),
    ] {
        assert_eq!(evidence[metric]["kind"], "exact");
        assert_eq!(evidence[metric]["value"], expected);
    }
    assert!(evidence["boundaries"].as_array().unwrap().is_empty());
    assert_eq!(function["density"]["outcome"], "not_evaluated");
}

fn replay(directory: &Path, original: &Value, detail: &str) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--mutation-rules",
                "--mutation-config",
                "mutation.toml",
                "--report-detail",
                detail,
                "--fail-on-findings",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RUSTC", directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}
