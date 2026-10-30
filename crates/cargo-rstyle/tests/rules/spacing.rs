use super::*;

const FIVE: &str =
    "pub fn five() -> u8 {\nlet a = 1;\nlet b = 2;\nlet c = 3;\nlet d = 4;\na + b + c + d\n}\n";

fn workspace(source: &str, spacing: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    policy(directory.path(), spacing);
    directory
}

fn policy(directory: &Path, spacing: &str) {
    std::fs::write(
        directory.join("analysis.toml"),
        format!("[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=100\n{spacing}"),
    )
    .unwrap();
}

fn capture(directory: &Path) -> Value {
    let (output, report) = source_report(directory, &[]);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        report
    );
    report
}

fn spacing<'a>(report: &'a Value, name: &str) -> &'a Value {
    assessed(report, name)["assessments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|assessment| assessment["rule_id"] == "fn_spacing")
        .unwrap()
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
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)));
    (output, report)
}

#[test]
fn spacing_is_opt_in_and_counts_the_tail_at_the_five_statement_boundary() {
    let source = format!(
        "{FIVE}pub fn four() -> u8 {{\nlet a = 1;\nlet b = 2;\nlet c = 3;\na + b + c\n}}\n"
    );
    let directory = workspace(&source, "");
    let original = capture(directory.path());
    assert!(original["analysis"]["effective_config"]["min_statements"].is_null());
    assert_eq!(
        assessed(&original, "five")["assessments"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    policy(directory.path(), "[fn_spacing]\n");
    let (output, report) = replay(directory.path(), &original, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{report}");
    assert_eq!(report["analysis"]["effective_config"]["min_statements"], 5);
    assert_eq!(spacing(&report, "five")["actual"], 5);
    assert_eq!(spacing(&report, "five")["limit"], 4);
    assert_eq!(spacing(&report, "five")["outcome"], "finding");
    assert_eq!(spacing(&report, "four")["actual"], 4);
    assert_eq!(spacing(&report, "four")["outcome"], "evaluated_no_finding");
}

#[test]
fn blank_lines_and_direct_statement_boundaries_keep_original_source_meaning() {
    let source = format!(
        "{}pub fn nested() -> u8 {{\nlet a = {{ let x = 1; let y = 2; x + y }};\nlet read = || {{ let x = 3; let y = 4; x + y }};\nlet b =\n    5 +\n    6;\na + read() + b\n}}\n",
        FIVE.replacen("let c", "\nlet c", 1)
    );
    let directory = workspace(&source, "[fn_spacing]\n");
    let report = capture(directory.path());
    assert_eq!(spacing(&report, "five")["actual"], 5);
    assert_eq!(spacing(&report, "five")["outcome"], "evaluated_no_finding");
    assert_eq!(spacing(&report, "nested")["actual"], 4);
    assert_eq!(
        spacing(&report, "nested")["outcome"],
        "evaluated_no_finding"
    );
    let owner = assessed(&report, "nested");
    let graph = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == owner["key"])
        .unwrap();
    assert_eq!(graph["outcome"], "excluded");
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
}

#[test]
fn unsupported_original_statement_forms_do_not_erase_length_or_arity_results() {
    let directory = workspace(
        "pub fn direct_macro() { println!(\"hello\"); }\npub fn conditional() { #[cfg(any())] let value = 1; }\npub async fn asynchronous(value: u8) -> u8 { value }\n",
        "[fn_spacing]\n",
    );
    let (output, report) = source_report(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    for name in ["direct_macro", "conditional", "asynchronous"] {
        let function = assessed(&report, name);
        for rule in function["assessments"].as_array().unwrap().iter().take(2) {
            assert_eq!(rule["availability"], "known", "{function}");
            assert_eq!(rule["outcome"], "evaluated_no_finding");
        }
        let rule = spacing(&report, name);
        assert_eq!(rule["outcome"], "not_evaluated", "{rule}");
        assert!(rule["actual"].is_null());
        assert_ne!(rule["coverage"], "complete");
    }
}

#[test]
fn disabling_spacing_needs_no_statement_count_and_saved_policy_can_enable_it() {
    let directory = workspace(
        "pub fn direct_macro() { println!(\"hello\"); }\n",
        "[fn_spacing]\nmin_statements=0\n",
    );
    let original = capture(directory.path());
    assert_eq!(
        spacing(&original, "direct_macro")["outcome"],
        "evaluated_no_finding"
    );
    assert_eq!(spacing(&original, "direct_macro")["limit"], 0);
    assert_eq!(original["analysis"]["scope"]["coverage"], "complete");
    for file in ["Cargo.toml", "src/lib.rs"] {
        std::fs::remove_file(directory.path().join(file)).unwrap();
    }
    policy(directory.path(), "[fn_spacing]\n");
    let (output, report) = replay(directory.path(), &original, &[]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(spacing(&report, "direct_macro")["outcome"], "not_evaluated");
    assert_eq!(report["verification_policy"], "strict-replay");
}

#[test]
fn spacing_suppression_hides_only_its_finding_and_preserves_raw_measurements() {
    let directory = workspace(
        &format!("// rstyle-allow(fn_spacing): documented grouping\n{FIVE}"),
        "[fn_spacing]\n",
    );
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=0\n[fn_params]\nmax_params=100\n[fn_spacing]\n",
    )
    .unwrap();
    let (output, report) = source_report(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{report}");
    assert_eq!(spacing(&report, "five")["outcome"], "finding");
    assert_eq!(spacing(&report, "five")["actual"], 5);
    assert_eq!(report["analysis"]["findings"], 2);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    assert_eq!(report["analysis"]["suppressed_findings"], 1);
    assert_eq!(
        assessed(&report, "five")["source_suppression"]["rules"],
        serde_json::json!(["fn_spacing"])
    );
}

#[test]
fn path_exclusion_skips_all_requested_source_rules_but_budget_never_becomes_clean() {
    let directory = workspace(FIVE, "[fn_spacing]\n");
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[\"src/**\"]\n[fn_spacing]\n",
    )
    .unwrap();
    let original = capture(directory.path());
    let function = assessed(&original, "five");
    assert_eq!(function["assessments"].as_array().unwrap().len(), 3);
    for rule in function["assessments"].as_array().unwrap() {
        assert_eq!(rule["outcome"], "not_evaluated");
        assert_eq!(rule["coverage"], "complete");
        assert_eq!(rule["availability"], "not_applicable");
    }
    assert_eq!(original["analysis"]["scope"]["excluded_rules"], 3);
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--max-analysis-functions", "0"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(report["analysis"]["scope"]["excluded_rules"], 0);
    assert_eq!(spacing(&report, "five")["outcome"], "not_evaluated");
    assert_ne!(spacing(&report, "five")["coverage"], "complete");
}

#[test]
fn source_free_compact_replay_retains_spacing_and_reevaluates_its_threshold() {
    let directory = workspace(FIVE, "[fn_spacing]\n");
    let original = capture(directory.path());
    for file in ["Cargo.toml", "src/lib.rs"] {
        std::fs::remove_file(directory.path().join(file)).unwrap();
    }
    let (output, compact) = replay(
        directory.path(),
        &original,
        &["--report-detail", "compact", "--fail-on-findings"],
    );
    assert_eq!(output.status.code(), Some(1), "{compact}");
    assert_eq!(original["analysis"], compact["analysis"]);
    policy(directory.path(), "[fn_spacing]\nmin_statements=6\n");
    let (output, clean) = replay(directory.path(), &original, &["--fail-on-findings"]);
    assert!(output.status.success(), "{clean}");
    assert_eq!(spacing(&clean, "five")["actual"], 5);
    assert_eq!(spacing(&clean, "five")["outcome"], "evaluated_no_finding");
    assert_eq!(clean["analysis"]["scope"]["coverage"], "complete");
}
