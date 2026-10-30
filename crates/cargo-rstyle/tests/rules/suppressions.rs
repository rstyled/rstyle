use super::*;

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=0\n[fn_params]\nmax_params=0\ncount_self=false\n",
    )
    .unwrap();
    directory
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

fn counts(report: &Value, raw: u64, visible: u64, suppressed: u64) {
    assert_eq!(report["analysis"]["findings"], raw);
    assert_eq!(report["analysis"]["visible_findings"], visible);
    assert_eq!(report["analysis"]["suppressed_findings"], suppressed);
    assert_eq!(raw, visible + suppressed);
}

#[test]
fn captured_allow_comments_filter_only_named_source_rules() {
    let directory = workspace(
        "// rstyle-allow(fn_params): generated signature\n\
         pub fn named(a: u8, b: u8) -> u8 { a + b }\n\
         // rstyle-allow(fn_params, fn_length): generated adapter\n\
         pub fn both(a: u8, b: u8) -> u8 { a + b }\n",
    );
    let full = capture(directory.path());
    counts(&full, 4, 1, 3);
    let named = assessed(&full, "named");
    assert_eq!(named["source_suppression"]["status"], "allowed");
    assert_eq!(
        named["source_suppression"]["rules"],
        serde_json::json!(["fn_params"])
    );
    assert_eq!(named["assessments"][0]["outcome"], "finding");
    assert_eq!(named["assessments"][1]["outcome"], "finding");
    let (output, compact) = replay(directory.path(), &full, &["--report-detail", "compact"]);
    assert!(output.status.success());
    assert_eq!(full["analysis"], compact["analysis"]);
    let (output, _) = replay(directory.path(), &full, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn invalid_allow_comments_remain_visible_diagnostics() {
    let directory = workspace(
        "// rstyle-allow(fn_params)\n\
         pub fn no_reason(a: u8) -> u8 { a }\n\
         // rstyle-allow(fn_length, unknown): invalid rule\n\
         pub fn unknown_rule(a: u8) -> u8 { a }\n",
    );
    let full = capture(directory.path());
    counts(&full, 6, 6, 0);
    for name in ["no_reason", "unknown_rule"] {
        let function = assessed(&full, name);
        assert_eq!(function["source_suppression"]["status"], "invalid");
        assert!(
            !function["source_suppression"]["reason"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        let diagnostic = &function["assessments"][2];
        assert_eq!(diagnostic["rule_id"], "invalid_suppression");
        assert_eq!(diagnostic["outcome"], "finding");
        assert_eq!(diagnostic["actual"], 1);
        assert_eq!(diagnostic["limit"], 0);
    }
    let (output, _) = replay(directory.path(), &full, &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn comment_association_skips_docs_and_attributes_but_stops_at_blank_lines() {
    let directory = workspace(
        "// rstyle-allow(fn_length, fn_params): attributed source\n\
         /// Original documentation.\n\
         #[inline]\n\
         pub fn attributed(a: u8) -> u8 { a }\n\
         // rstyle-allow(fn_length, fn_params): separated comment\n\n\
         pub fn separated(a: u8) -> u8 { a }\n",
    );
    let report = capture(directory.path());
    counts(&report, 4, 2, 2);
    assert_eq!(
        assessed(&report, "attributed")["source_suppression"]["status"],
        "allowed"
    );
    assert_eq!(
        assessed(&report, "separated")["source_suppression"]["status"],
        "none"
    );
}

#[test]
fn same_named_function_and_method_owners_keep_distinct_allow_comments() {
    let directory = workspace(
        "pub mod first {\n\
         // rstyle-allow(fn_length, fn_params): first owner only\n\
         pub fn same(a: u8) -> u8 { a }\n}\n\
         pub mod second {\n\
         pub fn same(a: u8) -> u8 { a }\n}\n\
         pub struct Owner;\nimpl Owner {\n\
         // rstyle-allow(fn_params): method owner only\n\
         pub fn same(&self, a: u8) -> u8 { a }\n}\n",
    );
    let report = capture(directory.path());
    counts(&report, 6, 3, 3);
    let mut states: Vec<_> = report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| function["source_suppression"]["status"].as_str().unwrap())
        .collect();
    states.sort_unstable();
    assert_eq!(states, ["allowed", "allowed", "none"]);
}

#[test]
fn source_free_replay_uses_original_comments_and_budget_failures_stay_incomplete() {
    let directory = workspace(
        "// rstyle-allow(fn_length, fn_params): original captured adapter\n\
         pub fn covered(a: u8) -> u8 { a }\n",
    );
    let original = capture(directory.path());
    counts(&original, 2, 0, 2);
    for delete in [false, true] {
        replace_original(directory.path(), delete);
        let (output, report) = replay(directory.path(), &original, &["--fail-on-findings"]);
        assert!(output.status.success(), "{report}");
        assert_eq!(original["analysis"], report["analysis"]);
        assert_eq!(report["verification_policy"], "strict-replay");
    }
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--max-analysis-functions", "0"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    counts(&report, 0, 0, 0);
    let function = assessed(&report, "covered");
    assert_eq!(function["source_suppression"]["status"], "unavailable");
    assert_eq!(function["assessments"][0]["outcome"], "not_evaluated");
    assert_eq!(function["assessments"][1]["outcome"], "not_evaluated");
}

fn replace_original(directory: &Path, delete: bool) {
    for file in ["Cargo.toml", "src/lib.rs"] {
        let path = directory.join(file);
        if delete {
            std::fs::remove_file(path).unwrap();
        } else {
            std::fs::write(path, "changed and deliberately invalid Rust/Cargo source").unwrap();
        }
    }
}

#[test]
fn legacy_non_source_rule_names_do_not_suppress_guard_opportunities() {
    let directory = workspace(
        "// rstyle-allow(abstraction, side_effect_density, fn_length, fn_params): source rules only\n\
         pub fn nested(value: u32) -> u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }\n",
    );
    let (output, report) =
        source_report(directory.path(), &["--guard-clauses", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{report}");
    counts(&report, 3, 1, 2);
    assert_eq!(
        assessed(&report, "nested")["guard_clause"]["outcome"],
        "finding"
    );
}
