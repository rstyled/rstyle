use super::propagation::checked;
use super::*;

const FLAGS: &[&str] = &[
    "--compiler-advisory-config",
    "compiler.toml",
    "--mutation-rules",
    "--mutation-config",
    "mutation.toml",
    "--review-file",
    "reviews.json",
];
const SOURCE: &str = "pub fn owned(mut value:u32)->u32 { value=1; value+=2; value=3; value }\npub fn borrowed(value:&mut u32) { *value=1; *value+=2; *value=3; }\npub fn counted()->u32 { let mut value=0u32; for _ in 0u32..3u32 { value+=1; } value }\npub fn conditional(mut value:u32)->u32 { while value<3 { value+=1; } value }\npub fn repeated()->u32 { let mut value=0u32; loop { value+=1; if value>2 { break; } } value }\n";
const MANY: &str = "pub fn many()->u32 { let mut value=0u32; value=1; value=2; value=3; value }\n";

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("compiler.toml"),
        "[fn_spacing]\nmin_statements=100\n[mutation_budget]\nmax_mutations_per_target=100\nmax_mutated_targets=100\n",
    )
    .unwrap();
    policy(directory.path(), 2);
    directory
}

fn policy(directory: &Path, writes: usize) {
    std::fs::write(
        directory.join("mutation.toml"),
        format!("[mutation_budget]\nmax_mutations_per_target={writes}\nmax_mutated_targets=1\n"),
    )
    .unwrap();
}

fn capture_mode(directory: &Path, raw: bool) -> (Output, Value) {
    let mut flags = FLAGS.to_vec();
    if raw {
        flags.push("--compiler-advisory-only");
    }
    super::propagation::capture(directory, &flags)
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["mutation_analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap()
}

fn raw_contract(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert!(report.get("matching").is_none());
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["pair_enumeration_complete"], false);
    assert_eq!(report["accepted_pairs"], serde_json::json!([]));
}

fn exact_count(report: &Value, name: &str, writes: usize, provenance: &str) {
    let row = function(report, name);
    assert_eq!(row["coverage"], "complete");
    assert_eq!(row["availability"], "known");
    assert_eq!(
        row["evidence"]["write_sites"],
        serde_json::json!({"kind":"exact", "value":writes})
    );
    for site in row["evidence"]["sites"].as_array().unwrap() {
        assert_eq!(site["root"]["provenance"], provenance);
        assert_eq!(site["caller_visible"], provenance == "formal_borrowed");
    }
}

#[test]
fn advisory_only_mutations_reuse_exact_scalar_formal_and_loop_collection() {
    let directory = fixture(SOURCE);
    let (output, ordinary) = capture_mode(directory.path(), false);
    checked(&output, 0);
    let (output, raw) = capture_mode(directory.path(), true);
    checked(&output, 0);
    raw_contract(&raw);
    assert_eq!(raw["mutation_analysis"], ordinary["mutation_analysis"]);
    assert_eq!(raw["mutation_analysis"]["coverage"], "complete");
    assert_eq!(raw["mutation_analysis"]["assessed_functions"], 5);
    for (name, writes, provenance) in [
        ("owned", 3, "formal_owned"),
        ("borrowed", 3, "formal_borrowed"),
        ("counted", 1, "local_owned"),
        ("conditional", 1, "formal_owned"),
        ("repeated", 1, "local_owned"),
    ] {
        exact_count(&raw, name, writes, provenance);
    }
    let plan = Path::new(raw["run_directory"].as_str().unwrap()).join("plan.json");
    let plan: Value = serde_json::from_slice(&std::fs::read(plan).unwrap()).unwrap();
    assert_eq!(plan["local_source_facts"], true);
    assert_eq!(plan["local_mutation_facts"], true);
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
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
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(FLAGS)
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

fn remove_sources(directory: &Path) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
}

fn same_evidence(left: &Value, right: &Value) {
    assert_eq!(
        function(left, "many")["evidence"],
        function(right, "many")["evidence"]
    );
    raw_contract(left);
}

#[test]
fn raw_mutation_replay_reuses_facts_without_sources_tools_or_strict_proofs() {
    let directory = fixture(MANY);
    let (output, original) = capture_mode(directory.path(), true);
    checked(&output, 0);
    assert_eq!(original["mutation_analysis"]["findings"], 1);
    let key = function(&original, "many")["key"].as_str().unwrap();
    let (output, hidden) = replay(
        directory.path(),
        &original,
        &[
            "--dismiss-mutation-per-target",
            key,
            "--review-reason",
            "Reviewed scalar write budget",
        ],
    );
    checked(&output, 0);
    same_evidence(&hidden, &original);
    assert_eq!(hidden["mutation_review"]["hidden_findings"], 1);
    remove_sources(directory.path());
    for detail in ["full", "compact"] {
        let (output, report) = replay(
            directory.path(),
            &original,
            &["--fail-on-findings", "--report-detail", detail],
        );
        checked(&output, 0);
        same_evidence(&report, &original);
        assert_eq!(report["mutation_review"]["hidden_findings"], 1);
    }
    policy(directory.path(), 3);
    let (output, relaxed) = replay(directory.path(), &original, &["--fail-on-findings"]);
    checked(&output, 0);
    same_evidence(&relaxed, &original);
    assert_eq!(relaxed["mutation_analysis"]["findings"], 0);
}

#[test]
fn raw_mutation_nonprimitive_formals_preserve_native_binding_consistency() {
    let directory = fixture(
        "pub fn tuple(value:(u32,u32))->u32 { let mut count=0u32; count+=1; value.0+count }\npub fn shared(value:&u32)->u32 { let mut count=0u32; count+=1; *value+count }\n",
    );
    let (output, ordinary) = capture_mode(directory.path(), false);
    checked(&output, 2);
    let (output, raw) = capture_mode(directory.path(), true);
    checked(&output, 2);
    raw_contract(&raw);
    assert_eq!(raw["mutation_analysis"], ordinary["mutation_analysis"]);
    for name in ["tuple", "shared"] {
        assert_eq!(function(&raw, name)["coverage"], "incomplete");
        assert!(
            !function(&raw, name)["evidence"]["boundaries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn raw_mutation_unknown_calls_preserve_witnessed_lower_bounds() {
    let directory = fixture(
        "pub fn many(callback:fn()) { let mut value=0u32; value=1; value=2; value=3; callback(); }\n",
    );
    let (output, ordinary) = capture_mode(directory.path(), false);
    checked(&output, 2);
    let (output, raw) = capture_mode(directory.path(), true);
    checked(&output, 2);
    raw_contract(&raw);
    assert_eq!(raw["mutation_analysis"], ordinary["mutation_analysis"]);
    assert_eq!(raw["mutation_analysis"]["coverage"], "incomplete");
    let evidence = &function(&raw, "many")["evidence"];
    assert_eq!(
        evidence["write_sites"],
        serde_json::json!({"kind":"lower_bound", "value":3})
    );
    assert!(!evidence["boundaries"].as_array().unwrap().is_empty());
}

#[test]
fn source_only_raw_capture_cannot_invent_missing_mutation_type_facts_on_replay() {
    let directory = fixture("pub fn many(value:&mut u32) { while *value<3 { *value+=1; } }\n");
    let (output, original) = super::propagation::capture(
        directory.path(),
        &["--compiler-advisory-only", "--source-rules"],
    );
    checked(&output, 0);
    remove_sources(directory.path());
    let (output, report) = replay(directory.path(), &original, &[]);
    checked(&output, 2);
    raw_contract(&report);
    assert_eq!(report["mutation_analysis"]["coverage"], "incomplete");
    let row = function(&report, "many");
    assert_eq!(row["outcome"], "not_evaluated");
    assert!(!row["evidence"]["boundaries"].as_array().unwrap().is_empty());
}

#[test]
fn raw_mutation_policy_preserves_source_suppression_and_replays_path_exclusions() {
    let source = format!("// rstyle-allow(mutation_per_target): deliberate scalar writes\n{MANY}");
    let directory = fixture(&source);
    let (output, ordinary) = capture_mode(directory.path(), false);
    checked(&output, 0);
    let (output, original) = capture_mode(directory.path(), true);
    checked(&output, 0);
    assert_eq!(original["mutation_analysis"], ordinary["mutation_analysis"]);
    assert_eq!(original["mutation_analysis"]["suppressed_findings"], 1);
    assert_eq!(original["mutation_analysis"]["visible_findings"], 0);
    remove_sources(directory.path());
    let (output, suppressed) = replay(directory.path(), &original, &["--fail-on-findings"]);
    checked(&output, 0);
    same_evidence(&suppressed, &original);
    assert_eq!(
        suppressed["mutation_analysis"],
        original["mutation_analysis"]
    );
    std::fs::write(directory.path().join("mutation.toml"), "exclude=[\"src/lib.rs\"]\n[mutation_budget]\nmax_mutations_per_target=2\nmax_mutated_targets=1\n").unwrap();
    let (output, excluded) = replay(directory.path(), &original, &["--fail-on-findings"]);
    checked(&output, 0);
    same_evidence(&excluded, &original);
    assert_eq!(excluded["mutation_analysis"]["excluded_findings"], 1);
    assert_eq!(excluded["mutation_analysis"]["visible_findings"], 0);
}
