use super::propagation::checked;
use super::*;

const SOURCE: &str = "fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}\npub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){leaf(a,b,c,d,e);}";
const OPTIONS: &[&str] = &["--compiler-patterns", "--compiler-plans"];

fn not_requested(report: &Value) {
    assert!(report.get("matching").is_none());
    for function in report["functions"].as_array().unwrap() {
        assert!(function["unit_evidence"].get("namespace").is_none());
    }
}

#[test]
fn advisory_only_preserves_completed_collectors_and_plans_without_strict_admission() {
    let (directory, output, ordinary) =
        capture(SOURCE, &["--compiler-patterns", "--compiler-plans"]);
    checked(&output, 0);
    let (output, raw) = super::propagation::capture(directory.path(), OPTIONS);
    checked(&output, 0);
    not_requested(&raw);
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

fn replay(directory: &Path, original: &Value) -> (Output, Value) {
    super::reviews::replay(
        directory,
        original,
        &[
            "--compiler-patterns",
            "--compiler-plans",
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
    {
        let (output, report) = replay(directory.path(), &original);
        checked(&output, 1);
        not_requested(&report);
        assert_eq!(report["compiler_advisory"], original["compiler_advisory"]);
        assert_eq!(report["capture_complete"], true);
        assert_eq!(report["handoff_failures"], serde_json::json!([]));
    }
    let output = rstyle(
        directory.path(),
        &[
            "advisory",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&output, 0);
    assert!(stdout(&output).contains("Compiler reports: 1 units"));
}

#[test]
fn advisory_only_reviews_use_completed_diagnostics_without_changing_raw_findings() {
    let (directory, output, original) = capture(SOURCE, OPTIONS);
    checked(&output, 0);
    let (output, reviewed) = super::reviews::replay(
        directory.path(),
        &original,
        &[
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
    let (output, omitted) =
        super::propagation::capture(directory.path(), &["--compiler-plans", "-p", "caller"]);
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

fn rejected_raw(directory: &Path, original: &Value, expected: &str) {
    let output = rstyle(
        directory,
        &[
            "advisory",
            "--format",
            "json",
            "--compiler-advisory",
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
fn advisory_only_replay_rejects_saved_unit_mode_and_test_context_changes() {
    let (directory, output, original) = capture("pub fn original(){}", OPTIONS);
    checked(&output, 0);
    let path = ledger_path(&original);
    let bytes = std::fs::read(&path).unwrap();
    let unit: Value = serde_json::from_slice(&bytes).unwrap();
    let mut changed = unit.clone();
    changed["test"] = true.into();
    std::fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    rejected_raw(directory.path(), &original, "ledger");
    std::fs::write(path, bytes).unwrap();
    let (output, replayed) = replay(directory.path(), &original);
    checked(&output, 0);
    not_requested(&replayed);
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
    let (output, replayed) = replay(directory.path(), &original);
    checked(&output, 2);
    not_requested(&replayed);
    assert_eq!(replayed["observed_units"], original["observed_units"]);
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
}
