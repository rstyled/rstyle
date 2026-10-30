use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn guarded(a:u32,b:u32)->u32 { if a>0 { let next=a+b; next+1 } else { 0 } }\npub fn caller(value:u32)->u32 { guarded(value,2) }\n";
const FLAGS: &[&str] = &[
    "--source-rules",
    "--guard-clauses",
    "--parameter-plans",
    "--guard-plans",
    "--analysis-config",
    "analysis.toml",
    "--fail-on-findings",
];

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn capture_mode(directory: &Path, raw: bool) -> (Output, Value) {
    let mut flags = FLAGS.to_vec();
    if raw {
        flags.push("--compiler-advisory-only");
    }
    super::propagation::capture(directory, &flags)
}

fn no_strict_work(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert!(report.get("matching").is_none());
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["pair_enumeration_complete"], false);
    assert_eq!(report["accepted_pairs"], serde_json::json!([]));
}

fn same_analysis(report: &Value, expected: &Value) {
    for field in [
        "analysis",
        "parameter_planning",
        "guard_planning",
        "edit_coordination",
    ] {
        assert_eq!(report[field], expected[field], "{field}");
    }
}

fn positive_plans(report: &Value) {
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["analysis"]["findings"], 2);
    assert_eq!(report["guard_planning"]["coverage"], "complete");
    assert_eq!(report["guard_planning"]["expected_plans"], 1);
    assert_eq!(report["parameter_planning"]["coverage"], "incomplete");
    let parameter = &report["parameter_planning"]["plans"][0];
    assert_eq!(parameter["callers"].as_array().unwrap().len(), 1);
    assert_eq!(parameter["signature"]["resource"], "src/lib.rs");
    let guard = &report["guard_planning"]["plans"][0];
    assert_eq!(guard["function"], parameter["function"]);
    assert_eq!(guard["region"]["resource"], "src/lib.rs");
}

fn replay_without_tools(directory: &Path, original: &Value, detail: &str) -> Output {
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
            "--report-detail",
            detail,
        ])
        .args(FLAGS)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

fn source_free_replays(directory: &Path, original: &Value) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay_without_tools(directory, original, detail);
        checked(&output, 2);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        no_strict_work(&report);
        same_analysis(&report, original);
    }
    let bundle = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(bundle).unwrap();
    assert!(rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).is_err());
    std::fs::remove_file(bundle).unwrap();
    let missing = replay_without_tools(directory, original, "compact");
    checked(&missing, 2);
}

#[test]
fn advisory_only_reuses_source_guard_and_planner_results_then_replays_without_sources_or_tools() {
    let directory = fixture(SOURCE);
    let (output, combined) = capture_mode(directory.path(), false);
    checked(&output, 2);
    let (output, raw) = capture_mode(directory.path(), true);
    checked(&output, 2);
    no_strict_work(&raw);
    same_analysis(&raw, &combined);
    positive_plans(&raw);
    source_free_replays(directory.path(), &raw);
}

#[test]
fn advisory_only_known_clean_source_and_guard_analysis_exits_successfully() {
    let directory = fixture("pub fn clean(value:u32)->u32 { value }\n");
    let (output, report) = capture_mode(directory.path(), true);
    checked(&output, 0);
    no_strict_work(&report);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["analysis"]["findings"], 0);
    assert_eq!(report["parameter_planning"]["expected_plans"], 0);
    assert_eq!(report["guard_planning"]["expected_plans"], 0);
}

#[test]
fn advisory_only_completed_source_findings_exit_one_without_requesting_plans() {
    let directory = fixture(SOURCE);
    let (output, report) = super::propagation::capture(
        directory.path(),
        &[
            "--compiler-advisory-only",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    no_strict_work(&report);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["analysis"]["findings"], 1);
    assert!(report.get("parameter_planning").is_none());
    assert!(report.get("guard_planning").is_none());
}

fn incomplete_guard(report: &Value) {
    no_strict_work(report);
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(report["analysis"]["findings"], 0);
    assert_eq!(report["guard_planning"]["coverage"], "incomplete");
    assert!(report["guard_planning"]["expected_plans"].is_null());
    assert!(
        report["analysis"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| { function["guard_clause"]["outcome"] == "not_evaluated" })
    );
}

#[test]
fn advisory_only_generated_and_closure_bodies_cannot_become_clean_guard_results() {
    for source in [
        "macro_rules! make {()=>{pub fn generated(){}}} make!();",
        "pub fn opaque(value:u32)->u32 { let read=||value; read() }",
    ] {
        let directory = fixture(source);
        let (output, report) = capture_mode(directory.path(), true);
        checked(&output, 2);
        incomplete_guard(&report);
    }
}

fn local_source_facts(report: &Value) -> Value {
    let plan = Path::new(report["run_directory"].as_str().unwrap()).join("plan.json");
    let plan: Value = serde_json::from_slice(&std::fs::read(plan).unwrap()).unwrap();
    plan["local_source_facts"].clone()
}

#[test]
fn compiler_only_saved_capture_cannot_recover_source_ownership_from_live_files() {
    let directory = fixture(SOURCE);
    let (output, original) =
        super::propagation::capture(directory.path(), &["--compiler-advisory-only"]);
    checked(&output, 0);
    assert_eq!(local_source_facts(&original), false);
    let mut flags = FLAGS.to_vec();
    flags.push("--compiler-advisory-only");
    let (output, missing) = super::reviews::replay(directory.path(), &original, &flags);
    checked(&output, 2);
    no_strict_work(&missing);
    assert!(directory.path().join("src/lib.rs").is_file());
    assert_eq!(missing["analysis"]["scope"]["coverage"], "incomplete");
    for function in missing["analysis"]["functions"].as_array().unwrap() {
        for assessment in function["assessments"].as_array().unwrap() {
            assert_eq!(assessment["availability"], "unknown");
            assert_eq!(assessment["outcome"], "not_evaluated");
            assert!(assessment["actual"].is_null());
        }
    }
    let (output, fresh) = capture_mode(directory.path(), true);
    checked(&output, 2);
    assert_eq!(local_source_facts(&fresh), true);
    positive_plans(&fresh);
}

#[test]
fn explicit_source_capture_preserves_the_existing_source_file_inventory_limit() {
    let directory = fixture("pub fn original(){}\n");
    let mut root = String::from("pub fn original(){}\n");
    for index in 0..257 {
        root.push_str(&format!("mod file_{index};\n"));
        std::fs::write(directory.path().join(format!("src/file_{index}.rs")), "").unwrap();
    }
    std::fs::write(directory.path().join("src/lib.rs"), root).unwrap();
    let (output, compiler) =
        super::propagation::capture(directory.path(), &["--compiler-advisory-only"]);
    checked(&output, 0);
    assert_eq!(compiler["capture_complete"], true);
    assert_eq!(local_source_facts(&compiler), false);
    let (output, source) = capture_mode(directory.path(), true);
    checked(&output, 2);
    no_strict_work(&source);
    assert_eq!(source["capture_complete"], false);
    assert!(!source["capture_failures"].as_array().unwrap().is_empty());
    assert_eq!(source["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(local_source_facts(&source), true);
}
