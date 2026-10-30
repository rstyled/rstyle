use super::*;

const GUARDED: &str = "// π\r\npub fn guarded(value: u32) -> u32 {\r\n    if value > 0 {\r\n        let next = value + 1;\r\n        next + 1\r\n    } else { 0 }\r\n}\r\n";

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"clone_fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn requested(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--guard-clauses",
        "--guard-plans",
        "--locked",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn guard_plan<'a>(report: &'a Value, name: &str) -> &'a Value {
    let function = report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap();
    report["guard_planning"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|plan| plan["function"] == function["key"])
        .unwrap()
}

fn source_range<'a>(source: &'a str, region: &Value) -> &'a str {
    let span = &region["span"];
    &source[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
}

fn assert_planning(report: &Value, coverage: &str, findings: u64) {
    assert_eq!(
        report["guard_planning"]["schema"],
        "rstyle.guard-planning/1"
    );
    assert_eq!(report["guard_planning"]["coverage"], coverage);
    assert_eq!(report["guard_planning"]["findings"], findings);
    assert_eq!(
        report["edit_coordination"]["schema"],
        "rstyle.edit-coordination/1"
    );
}

#[test]
fn guard_planning_is_explicit_and_requires_guard_analysis() {
    let directory = workspace("pub fn clean() {}\n");
    let output = rstyle(directory.path(), &["clones", "--guard-plans"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--guard-clauses"), "{stderr}");
    let (output, report) = clones(directory.path(), &[]);
    assert!(output.status.success());
    assert!(report.get("guard_planning").is_none());
    assert!(report.get("edit_coordination").is_none());
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["guard_planning"]);
    assert_planning(&report, "complete", 0);
    assert_eq!(report["guard_planning"]["expected_plans"], 0);
    assert!(
        report["guard_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn terminal_guard_region_uses_original_utf8_crlf_file_bytes() {
    let directory = workspace(GUARDED);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["guard_planning"]);
    assert_planning(&report, "complete", 1);
    assert_eq!(report["guard_planning"]["expected_plans"], 1);
    let region = &guard_plan(&report, "guarded")["region"];
    assert_eq!(region["resource"], "src/lib.rs");
    assert_eq!(region["package_manifest"], "Cargo.toml");
    assert_eq!(region["span"]["start"], GUARDED.find("if value").unwrap());
    assert_eq!(
        source_range(GUARDED, region),
        "if value > 0 {\r\n        let next = value + 1;\r\n        next + 1\r\n    } else { 0 }"
    );
    let plans = report["edit_coordination"]["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0]["id"], 0);
    assert_eq!(plans[0]["family"], "guard");
    assert_eq!(plans[0]["local_id"], guard_plan(&report, "guarded")["id"]);
}

#[test]
fn opaque_and_budget_limited_guards_are_incomplete_instead_of_clean_empty_planning() {
    let directory = workspace("pub fn opaque(value: u32) -> u32 { let read = || value; read() }\n");
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_planning(&report, "incomplete", 0);
    assert!(report["guard_planning"]["expected_plans"].is_null());
    std::fs::write(directory.path().join("src/lib.rs"), GUARDED).unwrap();
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    assert_planning(&report, "incomplete", 0);
    assert_eq!(report["guard_planning"]["execution"], "budget_exhausted");
    assert!(report["guard_planning"]["expected_plans"].is_null());
    assert!(
        report["guard_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn combined(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--source-rules",
        "--analysis-config",
        "analysis.toml",
        "--parameter-plans",
    ];
    args.extend_from_slice(extra);
    requested(directory, &args)
}

fn global_id(report: &Value, family: &str, local: &Value) -> u64 {
    report["edit_coordination"]["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|plan| plan["family"] == family && plan["local_id"] == *local)
        .unwrap()["id"]
        .as_u64()
        .unwrap()
}

fn relation(report: &Value, first: u64, second: u64) -> &Value {
    report["edit_coordination"]["coordination"]["relations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|relation| {
            (relation["first"] == first && relation["second"] == second)
                || (relation["first"] == second && relation["second"] == first)
        })
        .unwrap()
}

#[test]
fn own_parameter_signature_and_terminal_guard_are_disjoint_known_source_regions() {
    let directory = workspace(
        "pub fn guarded(a: u32, b: u32) -> u32 { if a > 0 { let next = a + b; next + 1 } else { 0 } }\n",
    );
    let (_, report) = combined(directory.path(), &[]);
    assert_planning(&report, "complete", 1);
    let guard = guard_plan(&report, "guarded");
    let parameter = &report["parameter_planning"]["plans"][0];
    assert_eq!(parameter["function"], guard["function"]);
    let pair = relation(
        &report,
        global_id(&report, "guard", &guard["id"]),
        global_id(&report, "parameter", &parameter["id"]),
    );
    assert_eq!(pair["relation"], "disjoint_known_regions");
}

#[test]
fn witnessed_parameter_call_before_terminal_guard_is_a_disjoint_known_edit_region() {
    let source = "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn guarded(value: u32) -> u32 {\n\
            let read = target(value, 2);\n\
            if value > 0 { let next = read + 1; next + 1 } else { 0 }\n\
        }\n";
    let directory = workspace(source);
    let (_, report) = combined(directory.path(), &[]);
    let guard = guard_plan(&report, "guarded");
    let parameter = &report["parameter_planning"]["plans"][0];
    let calls = parameter["callers"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        source_range(source, &calls[0]["region"]),
        "target(value, 2)"
    );
    let pair = relation(
        &report,
        global_id(&report, "guard", &guard["id"]),
        global_id(&report, "parameter", &parameter["id"]),
    );
    assert_eq!(pair["relation"], "disjoint_known_regions");
}

#[test]
fn parameter_call_inside_terminal_guard_has_a_real_overlap_witness_in_incomplete_scope() {
    let source = "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn guarded(value: u32) -> u32 {\n\
            if value > 0 { let next = target(value, 2); next + 1 } else { 0 }\n\
        }\n";
    let directory = workspace(source);
    let (_, report) = combined(directory.path(), &[]);
    assert_planning(&report, "complete", 1);
    assert_eq!(report["parameter_planning"]["coverage"], "incomplete");
    assert_eq!(
        report["edit_coordination"]["coordination"]["coverage_complete"],
        false
    );
    let guard = guard_plan(&report, "guarded");
    let parameter = &report["parameter_planning"]["plans"][0];
    let calls = parameter["callers"].as_array().unwrap();
    assert_eq!(calls.len(), 1);
    let call = &calls[0]["region"];
    assert_eq!(source_range(source, call), "target(value, 2)");
    let pair = relation(
        &report,
        global_id(&report, "guard", &guard["id"]),
        global_id(&report, "parameter", &parameter["id"]),
    );
    assert_eq!(pair["relation"], "overlap_witness");
    assert!(
        pair["witnesses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|witness| witness["intersection"] == call["span"])
    );
}

#[test]
fn source_suppression_and_path_exclusion_do_not_erase_guard_intents() {
    let directory = workspace(
        "// rstyle-allow(fn_params): original adapter\n\
        pub fn allowed(a: u32, b: u32) -> u32 { if a > 0 { let next = a + b; next + 1 } else { 0 } }\nmod excluded;\n",
    );
    std::fs::write(directory.path().join("src/excluded.rs"),
        "pub fn excluded(a: u32, b: u32) -> u32 { if a > 0 { let next = a + b; next + 1 } else { 0 } }\n").unwrap();
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[\"src/excluded.rs\"]\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = combined(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["guard_planning"]);
    assert_planning(&report, "complete", 2);
    assert_eq!(report["parameter_planning"]["findings"], 0);
    assert!(
        report["parameter_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!guard_plan(&report, "allowed")["region"].is_null());
    assert!(!guard_plan(&report, "excluded::excluded")["region"].is_null());
}

fn replay(directory: &Path, original: &Value, detail: &str) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--guard-clauses",
            "--guard-plans",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--parameter-plans",
            "--helper-proposals",
            "--edit-plans",
            "--report-detail",
            detail,
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
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
fn combined_planning_replays_original_regions_without_sources_or_tools() {
    let directory = workspace(
        "pub fn guarded(a: u32, b: u32) -> u32 { if a > 0 { let next = a + b; next + 1 } else { 0 } }\n",
    );
    let (output, original) = combined(directory.path(), &["--helper-proposals", "--edit-plans"]);
    assert!(matches!(output.status.code(), Some(0 | 2)));
    assert_planning(&original, "complete", 1);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (replayed, report) = replay(directory.path(), &original, detail);
        assert_eq!(replayed.status.code(), output.status.code());
        assert_eq!(report["verification_policy"], "strict-replay");
        for field in [
            "guard_planning",
            "parameter_planning",
            "edit_planning",
            "edit_coordination",
        ] {
            assert_eq!(report[field], original[field], "{field}");
        }
    }
}
