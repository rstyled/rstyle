use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn guarded(a:u32,b:u32)->u32 { if a>0 { let next=a+b; next+1 } else { 0 } }\npub fn caller(value:u32)->u32 { guarded(value,2) }\n";
const SOURCE_FLAGS: &[&str] = &[
    "--source-rules",
    "--guard-clauses",
    "--analysis-config",
    "source.toml",
];
const MUTATION_FLAGS: &[&str] = &[
    "--mutation-rules",
    "--mutation-effects",
    "--effect-density",
    "--mutation-config",
    "mutation.toml",
    "--effect-config",
    "effects.toml",
];

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    for (name, config) in [
        ("source.toml", "[fn_params]\nmax_params=1\n"),
        (
            "mutation.toml",
            "[mutation_budget]\nmax_mutations_per_target=1\n",
        ),
        (
            "effects.toml",
            "[side_effects]\nmin_statements=0\nmin_lines=0\n",
        ),
    ] {
        std::fs::write(directory.path().join(name), config).unwrap();
    }
    directory
}

fn capture_local(directory: &Path, flags: &[&str], numeric: bool) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory-only",
    ];
    args.extend_from_slice(flags);
    if numeric {
        args.push("--compiler-advisory");
    }
    clones(directory, &args)
}

fn no_compiler_work(report: &Value) {
    assert!(report.get("compiler_advisory").is_none());
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert!(report.get("matching").is_none());
    assert_eq!(report["accepted_pairs"], serde_json::json!([]));
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "not_requested");
        assert_eq!(
            function["unit_evidence"]["artifact_kind"],
            "advisory-raw-facts"
        );
    }
}

fn raw(report: &Value) -> Value {
    let bundle = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

fn only_requested_capture(report: &Value, mutations: bool) {
    let run = Path::new(report["run_directory"].as_str().unwrap());
    let plan: Value =
        serde_json::from_slice(&std::fs::read(run.join("plan.json")).unwrap()).unwrap();
    assert_eq!(plan["compiler_advisory"], false);
    assert_eq!(plan["compiler_advisory_only"], true);
    assert_eq!(plan["local_source_facts"], true);
    assert_eq!(plan["local_mutation_facts"], mutations);
    let ledger = only::ledger_path(report);
    let ledger: Value = serde_json::from_slice(&std::fs::read(ledger).unwrap()).unwrap();
    assert_eq!(ledger["compiler_advisory_only"], true);
    let wire = raw(report);
    assert_eq!(wire["phase"], "compiler-advisory-only");
    for fact in wire["facts"].as_array().unwrap() {
        let options = &fact["build"]["frontend_options"];
        assert!(options.get(rstyle_core::advisory::KEY).is_none());
        assert!(options.get(rstyle_core::advisory::patterns::KEY).is_none());
    }
}

fn replay_without_tools(directory: &Path, original: &Value, flags: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(flags)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

fn remove_sources(directory: &Path) {
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
}

#[test]
fn local_only_source_and_guard_results_match_numeric_combination_without_compiler_observations() {
    let directory = fixture(SOURCE);
    let (output, combined) = capture_local(directory.path(), SOURCE_FLAGS, true);
    checked(&output, 0);
    let (output, local) = capture_local(directory.path(), SOURCE_FLAGS, false);
    checked(&output, 0);
    no_compiler_work(&local);
    only_requested_capture(&local, false);
    assert_eq!(local["analysis"], combined["analysis"]);
    assert_eq!(local["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(local["analysis"]["findings"], 2);
    remove_sources(directory.path());
    let (output, replayed) = replay_without_tools(directory.path(), &local, SOURCE_FLAGS);
    checked(&output, 0);
    no_compiler_work(&replayed);
    assert_eq!(replayed["analysis"], local["analysis"]);
}

#[test]
fn local_only_mutation_and_density_results_reuse_existing_collectors_and_replay_without_tools() {
    let directory = fixture(
        "fn leaf(target:&mut u32){*target=1;*target=2;}\npub fn caller(target:&mut u32){leaf(target);leaf(target);}\n",
    );
    let (output, combined) = capture_local(directory.path(), MUTATION_FLAGS, true);
    checked(&output, 2);
    let (output, local) = capture_local(directory.path(), MUTATION_FLAGS, false);
    checked(&output, 2);
    no_compiler_work(&local);
    only_requested_capture(&local, true);
    for field in ["mutation_analysis", "mutation_effects"] {
        assert_eq!(local[field], combined[field], "{field}");
    }
    assert_eq!(local["mutation_analysis"]["coverage"], "incomplete");
    assert_eq!(local["mutation_analysis"]["unavailable_functions"], 1);
    assert_eq!(local["mutation_effects"]["coverage"], "complete");
    assert_eq!(local["mutation_analysis"]["findings"], 1);
    assert_eq!(local["mutation_effects"]["density_requested"], true);
    remove_sources(directory.path());
    let (output, replayed) = replay_without_tools(directory.path(), &local, MUTATION_FLAGS);
    checked(&output, 2);
    no_compiler_work(&replayed);
    for field in ["mutation_analysis", "mutation_effects"] {
        assert_eq!(replayed[field], local[field], "{field}");
    }
}

#[test]
fn local_only_replay_accepts_prior_observation_bearing_capture_without_running_numeric_rules() {
    let directory = fixture(SOURCE);
    let (output, combined) = capture_local(directory.path(), SOURCE_FLAGS, true);
    checked(&output, 0);
    assert!(
        raw(&combined)["facts"][0]["build"]["frontend_options"]
            .get(rstyle_core::advisory::KEY)
            .is_some()
    );
    remove_sources(directory.path());
    let (output, replayed) = replay_without_tools(directory.path(), &combined, SOURCE_FLAGS);
    checked(&output, 0);
    no_compiler_work(&replayed);
    assert_eq!(replayed["analysis"], combined["analysis"]);
}

#[test]
fn local_only_missing_and_malformed_source_ownership_stays_incomplete_even_with_live_sources() {
    let directory = fixture(SOURCE);
    let (output, original) = capture_local(directory.path(), SOURCE_FLAGS, false);
    checked(&output, 0);
    let wire = raw(&original);
    let ledger = only::ledger_path(&original);
    for missing in [true, false] {
        let mut changed = wire.clone();
        for fact in changed["facts"].as_array_mut().unwrap() {
            let options = fact["build"]["frontend_options"].as_object_mut().unwrap();
            if missing {
                assert!(options.remove("rstyle.nominal_source_census").is_some());
            } else {
                options.get_mut("rstyle.nominal_source_census").unwrap()["identity"] =
                    "independent-invalid-source-census".into();
            }
        }
        only::reseal_raw(&original, &ledger, &changed);
        let (output, report) = replay_without_tools(directory.path(), &original, SOURCE_FLAGS);
        checked(&output, 2);
        no_compiler_work(&report);
        assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
        assert_eq!(report["analysis"]["findings"], 0);
        assert!(directory.path().join("src/lib.rs").is_file());
    }
}
