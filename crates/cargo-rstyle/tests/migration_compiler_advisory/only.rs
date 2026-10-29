use super::propagation::checked;
use super::*;

const SOURCE: &str = "fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}\npub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){leaf(a,b,c,d,e);}";
const OPTIONS: &[&str] = &[
    "--compiler-advisory-only",
    "--compiler-patterns",
    "--compiler-plans",
];

fn not_requested(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert!(report.get("matching").is_none());
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["pair_enumeration_complete"], false);
    assert_eq!(report["accepted_pairs"], serde_json::json!([]));
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "not_requested");
        assert!(function["unit_evidence"].get("namespace").is_none());
        assert_eq!(
            function["unit_evidence"]["artifact_kind"],
            "advisory-raw-facts"
        );
    }
}

fn strict_import_rejects(report: &Value) {
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let bytes = std::fs::read(path).unwrap();
    assert!(rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).is_err());
    assert!(
        rstyle_pdg::verification_policy::generated(|| {
            rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes)
        })
        .is_err()
    );
}

#[test]
fn advisory_only_preserves_completed_collectors_and_plans_without_strict_admission() {
    let (directory, output, ordinary) =
        capture(SOURCE, &["--compiler-patterns", "--compiler-plans"]);
    checked(&output, 0);
    let (output, raw) = super::propagation::capture(directory.path(), OPTIONS);
    checked(&output, 0);
    not_requested(&raw);
    strict_import_rejects(&raw);
    assert_eq!(raw["compiler_advisory"], ordinary["compiler_advisory"]);
    let evaluation = &raw["compiler_advisory"]["units"][0]["evaluation"];
    assert_eq!(evaluation["summary"]["effect_counts"]["io"], 2);
    assert_eq!(evaluation["diagnostics"].as_array().unwrap().len(), 2);
    assert!(
        !raw["compiler_advisory"]["planning"]["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn replay(directory: &Path, original: &Value, detail: &str) -> (Output, Value) {
    super::reviews::replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--compiler-patterns",
            "--compiler-plans",
            "--report-detail",
            detail,
            "--fail-on-findings",
        ],
    )
}

#[test]
fn advisory_only_source_free_replay_preserves_findings_in_full_compact_and_text_reports() {
    let (directory, output, original) = capture(SOURCE, OPTIONS);
    checked(&output, 0);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, detail);
        checked(&output, 1);
        not_requested(&report);
        assert_eq!(report["compiler_advisory"], original["compiler_advisory"]);
        assert_eq!(report["capture_complete"], true);
        assert_eq!(report["handoff_failures"], serde_json::json!([]));
    }
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&output, 0);
    assert!(stdout(&output).contains("Strict assessment and matching: not_requested"));
}

fn rejects_replay_mode(directory: &Path, report: &Value, extra: &[&str]) {
    let mut args = vec![
        "clones",
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--replay-run",
        report["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let output = rstyle(directory, &args);
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("mode"));
}

#[test]
fn advisory_only_and_strict_saved_runs_cannot_change_capture_mode_on_replay() {
    let (directory, output, ordinary) = capture("pub fn original(){}", &[]);
    checked(&output, 0);
    rejects_replay_mode(directory.path(), &ordinary, &["--compiler-advisory-only"]);
    let (output, raw) =
        super::propagation::capture(directory.path(), &["--compiler-advisory-only"]);
    checked(&output, 0);
    rejects_replay_mode(directory.path(), &raw, &[]);
}

#[test]
fn advisory_only_reviews_use_completed_diagnostics_without_changing_raw_findings() {
    let (directory, output, original) = capture(SOURCE, OPTIONS);
    checked(&output, 0);
    let (output, reviewed) = super::reviews::replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Reviewed original fixture parameter boundary",
        ],
    );
    checked(&output, 0);
    not_requested(&reviewed);
    let advisory = &reviewed["compiler_advisory"];
    assert_eq!(
        advisory["diagnostic_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        advisory["units"][0]["evaluation"],
        original["compiler_advisory"]["units"][0]["evaluation"]
    );
}

#[test]
fn advisory_only_missing_and_generated_dependency_owners_remain_incomplete() {
    let leaf = "pub fn leaf(){std::fs::write(\"/unused\",b\"x\").ok();}";
    let directory = super::propagation::workspace(leaf);
    let (output, omitted) = super::propagation::capture(
        directory.path(),
        &[
            "--compiler-advisory-only",
            "--compiler-plans",
            "-p",
            "caller",
        ],
    );
    checked(&output, 2);
    not_requested(&omitted);
    assert_eq!(omitted["compiler_advisory"]["complete"], false);
    let caller = super::propagation::unit(&omitted, "caller");
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        format!("macro_rules! make {{()=>{{{leaf}}}}} make!();"),
    )
    .unwrap();
    let (output, generated) = super::propagation::capture(directory.path(), OPTIONS);
    checked(&output, 2);
    not_requested(&generated);
    let leaf = super::propagation::unit(&generated, "leaf");
    assert_eq!(leaf["observations"], serde_json::json!([]));
    assert_eq!(leaf["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(
        generated["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn advisory_only_requires_compiler_inputs_and_rejects_graph_consumers() {
    let directory = clone_workspace();
    let output = rstyle(directory.path(), &["clones", "--compiler-advisory-only"]);
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("--compiler-advisory"));
    for flag in [
        "--fail-on-accepted-pairs",
        "--explain-pairs",
        "--helper-proposals",
        "--enum-dispatch",
        "--capability-proposals",
    ] {
        let output = rstyle(
            directory.path(),
            &[
                "clones",
                "--compiler-advisory",
                "--compiler-advisory-only",
                flag,
            ],
        );
        checked(&output, 2);
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(flag),
            "{flag}"
        );
    }
}

pub(in super::super) fn ledger_path(report: &Value) -> PathBuf {
    std::fs::read_dir(report["run_directory"].as_str().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("unit-t")
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
        })
        .unwrap()
}

pub(in super::super) fn reseal_raw(report: &Value, ledger: &Path, wire: &Value) {
    let bundle = Path::new(
        report["functions"][0]["unit_evidence"]["bundle"]
            .as_str()
            .unwrap(),
    );
    let bytes = serde_json::to_vec(wire).unwrap();
    let manifest = rstyle_pdg::unit_store::BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    let seal = serde_json::to_vec(&manifest).unwrap();
    std::fs::write(bundle, bytes).unwrap();
    std::fs::write(bundle.with_file_name("unit.evidence.manifest.json"), &seal).unwrap();
    let mut unit: Value = serde_json::from_slice(&std::fs::read(ledger).unwrap()).unwrap();
    for artifact in unit["artifacts"].as_array_mut().unwrap() {
        match artifact["file"].as_str().unwrap() {
            "unit.evidence.json" => artifact["checksum"] = manifest.checksum.clone().into(),
            "unit.evidence.manifest.json" => {
                artifact["checksum"] = rstyle_pdg::unit_store::transport_checksum(&seal).into();
            }
            _ => {}
        }
    }
    std::fs::write(ledger, serde_json::to_vec(&unit).unwrap()).unwrap();
}

fn rejected_raw(directory: &Path, original: &Value, expected: &str) {
    let output = rstyle(
        directory,
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&output, 2);
    let evidence = match serde_json::from_slice::<Value>(&output.stdout) {
        Ok(report) => report["handoff_failures"].to_string(),
        Err(_) => String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    assert!(evidence.contains(expected), "{evidence}");
}

#[test]
fn advisory_only_replay_rejects_resealed_wrong_phase_and_compiler_context() {
    let (directory, output, original) = capture("pub fn original(){}", OPTIONS);
    checked(&output, 0);
    let ledger = ledger_path(&original);
    let bundle = original["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let wire: Value = serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap();
    let mut changed = wire.clone();
    changed["phase"] = "pre-helper-certification".into();
    reseal_raw(&original, &ledger, &changed);
    rejected_raw(directory.path(), &original, "publication phase");
    let mut changed = wire.clone();
    changed["facts"][0]["build"]["frontend_options"]["invocation"]
        .as_array_mut()
        .unwrap()
        .push("--cfg=independent_changed_context".into());
    reseal_raw(&original, &ledger, &changed);
    rejected_raw(directory.path(), &original, "invocation");
    reseal_raw(&original, &ledger, &wire);
    let (output, replayed) = replay(directory.path(), &original, "compact");
    checked(&output, 0);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn advisory_only_replay_rejects_saved_unit_mode_and_test_context_changes() {
    let (directory, output, original) = capture("pub fn original(){}", OPTIONS);
    checked(&output, 0);
    let path = ledger_path(&original);
    let bytes = std::fs::read(&path).unwrap();
    let unit: Value = serde_json::from_slice(&bytes).unwrap();
    for (field, value) in [("compiler_advisory_only", false), ("test", true)] {
        let mut changed = unit.clone();
        changed[field] = value.into();
        std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        rejected_raw(directory.path(), &original, "ledger");
    }
    std::fs::write(path, bytes).unwrap();
    let (output, replayed) = replay(directory.path(), &original, "compact");
    checked(&output, 0);
    not_requested(&replayed);
}

#[test]
fn advisory_only_selected_dependency_replays_shared_effects_and_parameter_plans() {
    let leaf =
        "pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}";
    let directory = super::propagation::workspace(leaf);
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){renamed::leaf(a,b,c,d,e);}",
    )
    .unwrap();
    let mut options = OPTIONS.to_vec();
    options.extend(["-p", "caller", "--capture-dependency", "leaf"]);
    let (output, original) = super::propagation::capture(directory.path(), &options);
    checked(&output, 0);
    not_requested(&original);
    let caller = super::propagation::unit(&original, "caller");
    assert_eq!(caller["evaluation"]["summary"]["effect_counts"]["io"], 1);
    let groups = original["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["step_owners"].as_array().unwrap().len(), 2);
    let run = Path::new(original["run_directory"].as_str().unwrap());
    assert!(!run.join("dependency-closure.json").exists());
    for source in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.path().join(source)).unwrap();
    }
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, replayed) = replay(directory.path(), &original, "compact");
    checked(&output, 1);
    not_requested(&replayed);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}

#[test]
fn advisory_only_check_and_test_owners_remain_separate_on_source_free_replay() {
    let source = "pub fn original(){} #[cfg(test)] pub fn test_only(){}";
    let mut options = OPTIONS.to_vec();
    options.push("--all-targets");
    let (directory, output, original) = capture(source, &options);
    checked(&output, 2);
    not_requested(&original);
    assert_eq!(original["compiler_advisory"]["complete"], false);
    let observed = original["observed_units"].as_array().unwrap();
    assert_eq!(observed.len(), 2);
    assert!(observed.iter().any(|unit| unit["test"] == true));
    assert!(observed.iter().any(|unit| unit["test"] == false));
    let units = original["compiler_advisory"]["units"].as_array().unwrap();
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    assert_eq!(
        units
            .iter()
            .map(|unit| unit["observations"].as_array().unwrap().len())
            .sum::<usize>(),
        3
    );
    assert_eq!(
        units
            .iter()
            .map(|unit| unit["expected_functions"].as_u64().unwrap())
            .sum::<u64>(),
        4
    );
    assert_eq!(
        units
            .iter()
            .map(|unit| unit["unavailable"].as_array().unwrap().len())
            .sum::<usize>(),
        1
    );
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, replayed) = replay(directory.path(), &original, "compact");
    checked(&output, 2);
    not_requested(&replayed);
    assert_eq!(replayed["observed_units"], original["observed_units"]);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}
