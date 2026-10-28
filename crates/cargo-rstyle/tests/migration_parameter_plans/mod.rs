use super::*;

const CALLS: &str = "pub fn many(left: u32, right: u32) -> u32 { left + right }\n\
pub fn reversed(right: u32, left: u32) -> u32 { left + right }\n\
pub mod namesake { pub fn many(value: u32) -> u32 { value } }\n\
pub fn caller(value: u32) -> u32 {\n\
    let first = many(value, 22);\n\
    let second = many(33, value);\n\
    let third = reversed(value, 66);\n\
    let other = namesake::many(77);\n\
    first + second + third + other\n}\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\ncount_self=false\n",
    )
    .unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--parameter-plans",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn plan<'a>(report: &'a Value, name: &str) -> &'a Value {
    let key = &report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            let candidate = function["name"].as_str().unwrap();
            (candidate == name.trim_start_matches("::") || candidate.ends_with(name))
                && !candidate.contains("namesake::")
        })
        .unwrap()["key"];
    report["parameter_planning"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|plan| plan["function"] == *key)
        .unwrap()
}

fn original_text<'a>(source: &'a str, span: &Value) -> &'a str {
    &source[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
}

fn assert_scope(planning: &Value, coverage: &str) {
    assert_eq!(planning["schema"], "rstyle.parameter-planning/1");
    assert_eq!(planning["coverage"], coverage);
    assert_eq!(planning["caller_scope"], "captured_selected_units");
    assert_eq!(planning["whole_plan_independence"], "unknown");
    assert_eq!(planning["stack_order"], "not_proven");
}

#[test]
fn original_parameter_signatures_and_calls_keep_namesakes_and_argument_order_distinct() {
    let directory = workspace(CALLS);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    let planning = &report["parameter_planning"];
    assert_scope(planning, "complete");
    assert_eq!(planning["findings"], 2);
    assert_eq!(planning["expected_plans"], 2);
    assert_eq!(planning["plans"].as_array().unwrap().len(), 2);
    assert_original_plan(
        &report,
        "::many",
        "pub fn many(left: u32, right: u32) -> u32 ",
        2,
    );
    assert_original_plan(
        &report,
        "::reversed",
        "pub fn reversed(right: u32, left: u32) -> u32 ",
        1,
    );
    let calls = plan(&report, "::many")["callers"].as_array().unwrap();
    assert_eq!(
        original_text(CALLS, &calls[0]["region"]["span"]),
        "many(value, 22)"
    );
    assert_eq!(
        original_text(CALLS, &calls[1]["region"]["span"]),
        "many(33, value)"
    );
    assert_formals(&calls[0], &[Some(0), None]);
    assert_formals(&calls[1], &[None, Some(0)]);
    assert_formals(&plan(&report, "::reversed")["callers"][0], &[Some(0), None]);
}

fn assert_original_plan(report: &Value, name: &str, signature: &str, calls: usize) {
    let plan = plan(report, name);
    assert_eq!(original_text(CALLS, &plan["signature"]["span"]), signature);
    assert_eq!(plan["signature"]["resource"], "src/lib.rs");
    assert_eq!(plan["signature"]["package_manifest"], "Cargo.toml");
    assert_eq!(plan["signature"]["function"], plan["function"]);
    assert_eq!(plan["callers"].as_array().unwrap().len(), calls);
    for caller in plan["callers"].as_array().unwrap() {
        assert_eq!(caller["region"]["function"], caller["function"]);
        assert_eq!(caller["region"]["resource"], "src/lib.rs");
    }
}

fn assert_formals(call: &Value, expected: &[Option<u32>]) {
    let arguments = call["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), expected.len());
    for (argument, expected) in arguments.iter().zip(expected) {
        assert_eq!(argument["formal"]["status"], "known");
        assert_eq!(argument["formal"]["value"], serde_json::json!(expected));
    }
}

#[test]
fn parameter_plans_require_source_rules_and_remain_opt_in() {
    let directory = workspace("pub fn clean(value: u32) -> u32 { value }\n");
    let output = rstyle(directory.path(), &["clones", "--parameter-plans"]);
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("--source-rules"), "{error}");
    let (output, ordinary) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(ordinary.get("parameter_planning").is_none());
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    assert_scope(&report["parameter_planning"], "complete");
    assert_eq!(report["parameter_planning"]["findings"], 0);
    assert_eq!(report["parameter_planning"]["expected_plans"], 0);
    assert!(
        report["parameter_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn suppressed_and_path_excluded_parameter_findings_create_no_edit_intents() {
    let directory = workspace(
        "// rstyle-allow(fn_params): generated API\n\
         pub fn allowed(a: u32, b: u32) -> u32 { a + b }\n\
         mod excluded;\n",
    );
    std::fs::write(
        directory.path().join("src/excluded.rs"),
        "pub fn excluded(a: u32, b: u32) -> u32 { a + b }\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[\"src/excluded.rs\"]\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    assert_eq!(report["analysis"]["suppressed_findings"], 1);
    assert_scope(&report["parameter_planning"], "complete");
    assert_eq!(report["parameter_planning"]["findings"], 0);
    assert_eq!(report["parameter_planning"]["expected_plans"], 0);
    assert!(
        report["parameter_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn opaque_macro_and_indirect_callers_do_not_claim_complete_edit_coverage() {
    let directory = workspace(
        "pub fn many(a: u32, b: u32) -> u32 { a + b }\n\
         macro_rules! invoke { ($value:expr) => { many($value, 2) }; }\n\
         pub fn caller(value: u32) -> u32 {\n\
             let selected = many;\n\
             let indirect = selected(value, 1);\n\
             let expanded = invoke!(value);\n\
             if value > 0 { many(indirect, expanded) } else { many(0, value) }\n\
         }\n",
    );
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_scope(&report["parameter_planning"], "incomplete");
    assert_eq!(report["parameter_planning"]["findings"], 1);
    assert!(
        !report["parameter_planning"]["caller_census"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        report["parameter_planning"]["caller_census"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unit| unit["coverage"] == "incomplete")
    );
}

#[test]
fn useful_direct_call_witness_survives_an_opaque_following_block() {
    let source = "pub fn many(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn caller(value: u32) -> u32 {\n\
            let known = many(value, 2);\n\
            { let selected = many; let _ = selected(value, 3); }\n\
            known\n\
        }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_scope(&report["parameter_planning"], "incomplete");
    let plan = plan(&report, "::many");
    assert!(!plan["signature"].is_null());
    assert!(!plan["reasons"].as_array().unwrap().is_empty());
    let calls = plan["callers"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        original_text(source, &calls[0]["region"]["span"]),
        "many(value, 2)"
    );
    assert_formals(&calls[0], &[Some(0), None]);
}

#[test]
fn excluded_caller_still_contributes_a_required_callsite_for_visible_target() {
    let directory = workspace("pub fn many(a: u32, b: u32) -> u32 { a + b }\nmod excluded;\n");
    let caller = "pub fn caller(value: u32) -> u32 { crate::many(value, 2) }\n";
    std::fs::write(directory.path().join("src/excluded.rs"), caller).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[\"src/excluded.rs\"]\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    assert_scope(&report["parameter_planning"], "complete");
    assert_eq!(report["parameter_planning"]["findings"], 1);
    let calls = plan(&report, "::many")["callers"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["region"]["resource"], "src/excluded.rs");
    assert_eq!(
        original_text(caller, &calls[0]["region"]["span"]),
        "crate::many(value, 2)"
    );
    assert_formals(&calls[0], &[Some(0), None]);
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
            "--parameter-plans",
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
fn parameter_planning_replay_preserves_original_regions_and_budget_failures() {
    let directory = workspace(CALLS);
    let (output, original) = requested(directory.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        original["parameter_planning"]
    );
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, &["--report-detail", detail]);
        assert!(output.status.success(), "{}", report["parameter_planning"]);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["parameter_planning"], original["parameter_planning"]);
        assert_eq!(report["matching"], original["matching"]);
    }
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--max-analysis-functions", "0"],
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_scope(&report["parameter_planning"], "incomplete");
    assert!(report["parameter_planning"]["expected_plans"].is_null());
    assert!(
        report["parameter_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn missing_original_ownership_cannot_become_a_clean_empty_parameter_plan() {
    let directory = workspace(CALLS);
    let (output, report) = clones(directory.path(), &["--source-rules", "--parameter-plans"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_scope(&report["parameter_planning"], "incomplete");
    assert!(report["parameter_planning"]["expected_plans"].is_null());
    assert!(
        report["parameter_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
