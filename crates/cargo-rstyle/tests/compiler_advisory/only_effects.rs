use super::propagation::checked;
use super::*;

const POLICY: &str = "[side_effects]\nmax_density_milli=500\nmin_statements=0\nmin_lines=0\n";
const FLAGS: &[&str] = &[
    "--compiler-advisory-config",
    "compiler.toml",
    "--effect-density",
    "--effect-config",
    "effects.toml",
];
const CHAIN: &str = "fn leaf(target:&mut u32){*target=1;*target=2;}\nfn wrapper(target:&mut u32){leaf(target);leaf(target);}\npub fn caller(target:&mut u32){wrapper(target);wrapper(target);}\npub fn counted(target:&mut u32){for _ in 0u32..3u32 {*target+=1;}}\n";

fn fixture(source: &str, policy: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("effects.toml"), policy).unwrap();
    std::fs::write(
        directory.path().join("compiler.toml"),
        "[fn_spacing]\nmin_statements=100\n[mutation_budget]\nmax_mutations_per_target=100\nmax_mutated_targets=100\n",
    )
    .unwrap();
    directory
}

fn capture(directory: &Path, raw: bool, extra: &[&str]) -> (Output, Value) {
    let mut flags = FLAGS.to_vec();
    if raw {
        flags.push("--compiler-advisory-only");
    }
    flags.extend_from_slice(extra);
    super::propagation::capture(directory, &flags)
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["mutation_effects"]["functions"]
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

fn measured(report: &Value, name: &str, events: usize, actual: usize) {
    let density = &function(report, name)["density"];
    assert_eq!(density["complete"], true, "{name}: {density}");
    assert_eq!(density["event_inventory"]["coverage"], "complete");
    assert_eq!(
        density["event_inventory"]["events"]
            .as_array()
            .unwrap()
            .len(),
        events
    );
    assert_eq!(density["measurement"]["actual"], actual);
    assert_eq!(density["measurement"]["exempt"], false);
}

fn parity(directory: &Path, extra: &[&str], code: i32) -> Value {
    let (output, ordinary) = capture(directory, false, extra);
    checked(&output, code);
    let (output, raw) = capture(directory, true, extra);
    checked(&output, code);
    raw_contract(&raw);
    assert_eq!(raw["mutation_effects"], ordinary["mutation_effects"]);
    raw
}

#[test]
fn raw_effects_reuse_scalar_summaries_repeated_occurrences_and_original_loop_sites() {
    let directory = fixture(CHAIN, POLICY);
    let raw = parity(directory.path(), &["--mutation-effects"], 0);
    assert_eq!(raw["mutation_effects"]["coverage"], "complete");
    for (name, direct, calls) in [
        ("leaf", 2, 0),
        ("wrapper", 0, 2),
        ("caller", 0, 2),
        ("counted", 1, 0),
    ] {
        let evidence = &function(&raw, name)["evidence"];
        assert_eq!(evidence["coverage"], "complete", "{name}: {evidence}");
        assert_eq!(evidence["caller_visible_mutation"], "writes_formal");
        assert_eq!(evidence["direct_site_count"], direct);
        assert_eq!(evidence["calls"].as_array().unwrap().len(), calls);
        assert_eq!(evidence["roots"].as_array().unwrap().len(), 1);
        assert_eq!(evidence["roots"][0]["formal"], 0);
    }
    measured(&raw, "leaf", 1, 500);
    measured(&raw, "wrapper", 2, 1000);
    measured(&raw, "caller", 2, 1000);
    let calls = function(&raw, "caller")["evidence"]["calls"]
        .as_array()
        .unwrap();
    assert_ne!(calls[0]["span"], calls[1]["span"]);
    assert_eq!(calls[0]["summary_index"], calls[1]["summary_index"]);
    for call in calls {
        assert_eq!(call["mappings"].as_array().unwrap().len(), 4);
    }
}

#[test]
fn raw_scalar_effects_run_without_requesting_density_or_direct_rules() {
    let directory = fixture(CHAIN, POLICY);
    let flags = [
        "--compiler-advisory-config",
        "compiler.toml",
        "--mutation-effects",
    ];
    let (output, ordinary) = super::propagation::capture(directory.path(), &flags);
    checked(&output, 0);
    let mut flags = flags.to_vec();
    flags.push("--compiler-advisory-only");
    let (output, raw) = super::propagation::capture(directory.path(), &flags);
    checked(&output, 0);
    raw_contract(&raw);
    assert_eq!(raw["mutation_effects"], ordinary["mutation_effects"]);
    assert_eq!(raw["mutation_effects"]["coverage"], "complete");
    assert_eq!(raw["mutation_effects"]["density_requested"], false);
    assert!(function(&raw, "caller")["density"].is_null());
    let plan = Path::new(raw["run_directory"].as_str().unwrap()).join("plan.json");
    let plan: Value = serde_json::from_slice(&std::fs::read(plan).unwrap()).unwrap();
    assert_eq!(plan["local_source_facts"], true);
    assert_eq!(plan["local_mutation_facts"], true);
}

#[test]
fn raw_density_classifies_external_calls_without_claiming_complete_scalar_mutations() {
    let policy = format!("{POLICY}\n[side_effects.paths]\ntime=['std::thread::yield_now']\n");
    let directory = fixture(
        "pub fn yielding(){std::thread::yield_now();}\npub fn aborting(){std::process::abort();}\n",
        &policy,
    );
    let raw = parity(directory.path(), &[], 0);
    assert_eq!(raw["mutation_effects"]["scalar_mutations_requested"], false);
    assert_eq!(raw["mutation_effects"]["coverage"], "complete");
    for (name, kind, actual) in [("yielding", "time", 1000), ("aborting", "io", 3000)] {
        measured(&raw, name, 1, actual);
        let row = function(&raw, name);
        assert_eq!(row["density"]["event_inventory"]["counts"][kind], 1);
        assert_eq!(row["evidence"]["coverage"], "incomplete");
    }
    let combined = parity(directory.path(), &["--mutation-effects"], 2);
    assert_eq!(combined["mutation_effects"]["coverage"], "incomplete");
    assert_eq!(
        function(&combined, "yielding")["density"],
        function(&raw, "yielding")["density"]
    );
}

#[test]
fn raw_unknown_effects_never_gain_ratios_or_exemptions_from_zero_unknown_weight() {
    let policy =
        "[side_effects]\nmin_statements=100\nmin_lines=100\n[side_effects.weights]\nunknown=0\n";
    for source in [
        "pub fn caller(callback:fn()){callback();}",
        "macro_rules! opaque {()=>{()}} pub fn caller(){opaque!();}",
        "fn leaf(t:&mut u32){*t=1;} fn first(t:&mut u32){leaf(t);} fn second(t:&mut u32){first(t);} pub fn caller(t:&mut u32){second(t);}",
    ] {
        let directory = fixture(source, policy);
        let raw = parity(
            directory.path(),
            &["--mutation-effects", "--fail-on-findings"],
            2,
        );
        assert_eq!(raw["mutation_effects"]["coverage"], "incomplete");
        let density = &function(&raw, "caller")["density"];
        assert_eq!(density["complete"], false);
        assert!(density["measurement"]["actual"].is_null());
        assert_eq!(density["measurement"]["outcome"], "not_evaluated");
        assert_eq!(density["measurement"]["exempt"], false);
        assert_eq!(density["visible_findings"], 0);
    }
}

fn remove_sources(directory: &Path) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let mut command = Command::new(executable);
    command.current_dir(directory).args([
        "clones",
        "--format",
        "json",
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-advisory-only",
        "--mutation-effects",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ]);
    command
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

#[test]
fn raw_effects_replay_full_and_compact_numeric_baselines_without_sources_or_tools() {
    let directory = fixture("pub fn caller(target:&mut u32){*target=1;}\n", POLICY);
    let (output, original) = capture(
        directory.path(),
        true,
        &[
            "--mutation-effects",
            "--write-density-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    remove_sources(directory.path());
    replay_baselines(directory.path(), &original);
    std::fs::write(
        directory.path().join("effects.toml"),
        POLICY.replace("max_density_milli=500", "max_density_milli=1000"),
    )
    .unwrap();
    let (output, relaxed) = replay(directory.path(), &original, &["--fail-on-findings"]);
    checked(&output, 0);
    measured(&relaxed, "caller", 1, 1000);
    assert_eq!(relaxed["mutation_effects"]["findings"], 0);
    assert_eq!(
        function(&relaxed, "caller")["evidence"],
        function(&original, "caller")["evidence"]
    );
}

#[test]
fn old_source_only_raw_capture_cannot_invent_optional_effect_type_facts() {
    let directory = fixture(
        "pub fn caller(target:&mut u32){while *target<3 {*target+=1;}}\n",
        POLICY,
    );
    let (output, original) = super::propagation::capture(
        directory.path(),
        &[
            "--compiler-advisory-only",
            "--source-rules",
            "--compiler-advisory-config",
            "compiler.toml",
        ],
    );
    checked(&output, 0);
    remove_sources(directory.path());
    let (output, report) = replay(directory.path(), &original, &[]);
    checked(&output, 2);
    raw_contract(&report);
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
    let row = function(&report, "caller");
    assert_eq!(row["evidence"]["coverage"], "incomplete");
    assert_eq!(row["density"]["complete"], false);
    assert!(row["density"]["measurement"]["actual"].is_null());
}

fn replay_baselines(directory: &Path, original: &Value) {
    for detail in ["full", "compact"] {
        let (output, report) = replay(
            directory,
            original,
            &[
                "--report-detail",
                detail,
                "--density-baseline",
                "baseline.json",
                "--fail-on-new-density",
            ],
        );
        checked(&output, 0);
        raw_contract(&report);
        assert_eq!(report["mutation_effects"], original["mutation_effects"]);
        assert_eq!(report["density_review"]["baseline"]["coverage"], "complete");
        assert_eq!(report["density_review"]["baseline"]["existing_findings"], 1);
        assert_eq!(report["density_review"]["baseline"]["new_findings"], 0);
    }
}
