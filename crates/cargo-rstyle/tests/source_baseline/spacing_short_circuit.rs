use super::*;

const RAW: &[&str] = &["--compiler-advisory-only"];
const SPACING_POLICY: &str =
    "[fn_length]\nmax_lines=10000\n[fn_params]\nmax_params=100\n[fn_spacing]\nmin_statements=5\n";
const MACRO: &str =
    "pub fn separated() {\n    println!(\"first\");\n\n    println!(\"second\");\n}\n";
const FIVE: &str = "pub fn five()->u8 {\nlet a=1;\nlet b=2;\nlet c=3;\nlet d=4;\na+b+c+d\n}\n";

fn fixture(source: &str) -> TempDir {
    let directory = workspace(source);
    std::fs::write(directory.path().join("analysis.toml"), SPACING_POLICY).unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut flags = RAW.to_vec();
    flags.extend_from_slice(extra);
    requested(directory, &flags)
}

fn spacing<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap()["assessments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|assessment| assessment["rule_id"] == "fn_spacing")
        .unwrap()
}

fn decisive_blank(report: &Value, name: &str, availability: &str) {
    let assessment = spacing(report, name);
    assert_eq!(assessment["coverage"], "complete");
    assert_eq!(assessment["outcome"], "evaluated_no_finding");
    assert_eq!(assessment["availability"], availability);
    assert!(assessment["actual"].is_null());
    assert!(!assessment["reasons"].as_array().unwrap().is_empty());
}

fn source_free(directory: &Path, original: &Value) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut flags = [RAW, BASELINE].concat();
        flags.extend(["--report-detail", detail]);
        let output = replay(directory, original, &flags);
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 0, 0);
        assert_eq!(report["verification_policy"], "raw-replay");
        assert_eq!(report["analysis"], original["analysis"]);
        assert!(report.get("compiler_advisory").is_none());
    }
}

#[test]
fn checked_blank_lines_complete_opaque_spacing_without_inventing_counts_and_replay_source_free() {
    let source = format!("{MACRO}{}", FIVE.replace("let c=3;", "\nlet c=3;"));
    let directory = fixture(&source);
    let (output, original) = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert_eq!(original["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(original["analysis"]["findings"], 0);
    decisive_blank(&original, "separated", "unsupported");
    assert_eq!(spacing(&original, "five")["actual"], 5);
    assert_eq!(spacing(&original, "five")["availability"], "known");
    assert!(
        stored(directory.path())["entries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    source_free(directory.path(), &original);
}

#[test]
fn blank_line_in_original_signature_keeps_legacy_spacing_meaning_with_opaque_body() {
    let directory = fixture(
        "pub fn signature(\n    value:u32,\n\n)->u32 {\n    println!(\"value\");\n    value\n}\n",
    );
    let (output, report) = capture(directory.path(), &["--fail-on-findings"]);
    checked(&output, 0);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    decisive_blank(&report, "signature", "unsupported");
}

fn oversized(blank: bool) -> String {
    format!(
        "pub fn large() {{\n{}{}}}\n",
        if blank { "\n" } else { "" },
        "let _value=0u32;\n".repeat(80)
    )
}

#[test]
fn blank_line_avoids_unnecessary_statement_budget_but_unspaced_large_and_macro_bodies_stay_incomplete()
 {
    let directory = fixture(&oversized(true));
    let (output, blank) = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert_eq!(blank["analysis"]["execution"], "completed");
    assert_eq!(blank["analysis"]["scope"]["coverage"], "complete");
    decisive_blank(&blank, "large", "not_applicable");
    for source in [
        oversized(false),
        "pub fn compact() {\nprintln!(\"first\");\n// phase\nprintln!(\"second\");\n}\n".to_owned(),
    ] {
        let directory = fixture(&source);
        let (output, report) = capture(directory.path(), &[]);
        checked(&output, 2);
        let name = report["analysis"]["functions"][0]["name"].as_str().unwrap();
        let assessment = spacing(&report, name);
        assert_eq!(assessment["outcome"], "not_evaluated");
        assert_eq!(assessment["coverage"], "incomplete");
        assert!(assessment["actual"].is_null());
        let output = replay(
            directory.path(),
            &report,
            &[
                "--compiler-advisory-only",
                "--write-source-baseline",
                "invalid.json",
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("invalid.json").exists());
    }
}

fn raw_wire(report: &Value) -> Value {
    let bundle = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

#[test]
fn blank_bytes_never_replace_missing_source_context_or_contradictory_source_identity() {
    let directory = fixture(MACRO);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    let wire = raw_wire(&original);
    let ledger = super::super::compiler_advisory::only::ledger_path(&original);
    for damage in ["context", "path", "bytes"] {
        let mut changed = wire.clone();
        match damage {
            "bytes" => changed["facts"][0]["source"]["digest"] = "xxh3-64:0000000000000000".into(),
            _ => {
                changed["facts"][0]["build"]["frontend_options"]
                    .as_object_mut()
                    .unwrap()
                    .remove(if damage == "context" {
                        rstyle_pdg::nominal_boundary::CONTEXT_KEY
                    } else {
                        "source_file"
                    });
            }
        }
        super::super::compiler_advisory::only::reseal_raw(&original, &ledger, &changed);
        let output = replay(
            directory.path(),
            &original,
            &[
                "--compiler-advisory-only",
                "--write-source-baseline",
                "invalid.json",
            ],
        );
        checked(&output, 2);
        assert!(!directory.path().join("invalid.json").exists(), "{damage}");
    }
    super::super::compiler_advisory::only::reseal_raw(&original, &ledger, &wire);
    checked(&replay(directory.path(), &original, RAW), 0);
}

fn dismiss_spacing(directory: &Path, original: &Value) {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-spacing",
            original["analysis"]["functions"][0]["key"]
                .as_str()
                .unwrap(),
            "--review-reason",
            "Reviewed original unseparated phase",
        ],
    );
    checked(&output, 0);
    assert_eq!(parsed(&output)["spacing_review"]["hidden_findings"], 1);
}

fn separated_capture(directory: &Path) -> Value {
    std::fs::write(
        directory.join("src/lib.rs"),
        "pub fn five()->u8 {\nprintln!(\"phase\");\n\n1\n}\n",
    )
    .unwrap();
    let (output, current) = capture(directory, &[]);
    checked(&output, 0);
    current
}

#[test]
fn decisive_blank_spacing_without_count_resolves_same_policy_baseline_and_review() {
    let directory = fixture(FIVE);
    let (output, original) = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert_eq!(spacing(&original, "five")["actual"], 5);
    dismiss_spacing(directory.path(), &original);
    let current = separated_capture(directory.path());
    let flags = [
        "--review-file",
        "reviews.json",
        "--review-status",
        "--review-prune",
        "--review-prune-dry-run",
        "--source-baseline",
        "baseline.json",
        "--fail-on-new-source",
    ];
    let flags = [RAW, &flags].concat();
    let output = replay(directory.path(), &current, &flags);
    checked(&output, 0);
    let resolved = parsed(&output);
    decisive_blank(&resolved, "five", "unsupported");
    counts(&resolved, 0, 0);
    assert_eq!(
        resolved["source_baseline"]["retained"][0]["state"],
        "resolved"
    );
    assert_eq!(
        resolved["review_status"]["records"][0]["state"],
        "no_finding"
    );
    assert_eq!(resolved["review_prune"]["eligible_entries"], 1);
    assert_eq!(resolved["review_prune"]["removed_entries"], 0);
    std::fs::write(
        directory.path().join("analysis.toml"),
        SPACING_POLICY.replace("min_statements=5", "min_statements=6"),
    )
    .unwrap();
    let output = replay(directory.path(), &current, &flags);
    checked(&output, 1);
    let stale = parsed(&output);
    assert_eq!(stale["review_status"]["records"][0]["state"], "stale");
    assert_eq!(stale["review_prune"]["eligible_entries"], 0);
    assert_eq!(stale["review_prune"]["removed_entries"], 0);
}
