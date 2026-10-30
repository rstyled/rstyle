use super::*;

const FIRST_GUARD: &str = "if first > 0 { let next = first + 1; next + 1 } else { 0 }";
const SECOND_GUARD: &str = "if second > 0 { let next = second + 2; next + 2 } else { 0 }";
const FIRST: &str = "|first:u32| { if first > 0 { let next = first + 1; next + 1 } else { 0 } }";
const SECOND: &str =
    "|second:u32| { if second > 0 { let next = second + 2; next + 2 } else { 0 } }";
const OPTIONS: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--guard-clauses",
    "--guard-plans",
];

fn source() -> String {
    format!(
        "// π\r\npub fn outer(value:u32)->u32 {{\r\n    // λ inside original function\r\n    let first = {FIRST};\r\n    let rejected = |early:u32| {{ if early == 0 {{ return 0; }} if early > 1 {{ let next = early + 1; next + 1 }} else {{ 0 }} }};\r\n    let second = {SECOND};\r\n    if value > 0 {{ let next = first(value) + second(value) + rejected(value); next + 1 }} else {{ 0 }}\r\n}}\r\n"
    )
}

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn span(start: usize, len: usize) -> Value {
    serde_json::json!({"start":start,"end":start+len})
}

fn assert_scoped_regions(report: &Value, source: &str) {
    let function = report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().ends_with("outer"))
        .unwrap();
    let offset = function["fn_span"]["start"].as_u64().unwrap() as usize;
    let opportunities = guard(report, "outer")["closure_opportunities"]
        .as_array()
        .unwrap();
    let plans = report["guard_planning"]["plans"].as_array().unwrap();
    assert_eq!(opportunities.len(), 2);
    assert_eq!(plans.len(), 2);
    for (closure, tail) in [(FIRST, FIRST_GUARD), (SECOND, SECOND_GUARD)] {
        let relative = span(source.find(closure).unwrap() - offset, closure.len());
        let opportunity = opportunities
            .iter()
            .find(|row| row["closure_span"] == relative)
            .unwrap();
        let absolute = source.find(tail).unwrap();
        assert_eq!(opportunity["span"], span(absolute - offset, tail.len()));
        assert_eq!(opportunity["fallback_branch"], "else");
        assert!(
            opportunity["advisory"]
                .as_str()
                .unwrap()
                .contains("within this closure")
        );
        let plan = plans
            .iter()
            .find(|plan| plan["closure_span"] == relative)
            .unwrap();
        assert_eq!(plan["function"], function["key"]);
        assert_eq!(plan["region"]["resource"], "src/lib.rs");
        assert_eq!(plan["region"]["span"], span(absolute, tail.len()));
    }
    assert_ne!(plans[0]["id"], plans[1]["id"]);
}

#[test]
fn synchronous_closures_keep_separate_guard_regions_and_original_utf8_crlf_offsets() {
    let source = source();
    let directory = workspace(&source);
    let mut args = OPTIONS.to_vec();
    args.push("--fail-on-findings");
    let (output, report) = clones(directory.path(), &args);
    checked(&output, 2);
    let assessment = guard(&report, "outer");
    assert!(assessment["opportunity"].is_null());
    assert_eq!(assessment["availability"], "unsupported");
    assert_eq!(assessment["coverage"], "incomplete");
    assert_eq!(report["analysis"]["scope"]["coverage"], "incomplete");
    assert_eq!(report["guard_planning"]["coverage"], "incomplete");
    assert_scoped_regions(&report, &source);
}

#[test]
fn unsupported_closure_control_flow_does_not_produce_guard_opportunities() {
    let bodies = [
        "async |value:u32| { if value > 0 { let next = value + 1; next + 1 } else { 0 } }",
        "|value:u32| if value > 0 { let next = value + 1; next + 1 } else { 0 }",
        "|value:u32| { let _ = std::stringify!(hidden); if value > 0 { let next = value + 1; next + 1 } else { 0 } }",
        "|value:u32| { for _ in 0..value {} if value > 0 { let next = value + 1; next + 1 } else { 0 } }",
        "|value:u32| { if value == 0 { return 0; } if value > 0 { let next = value + 1; next + 1 } else { 0 } }",
    ];
    let source = bodies
        .iter()
        .enumerate()
        .map(|(index, body)| format!("pub fn rejected_{index}() {{ let _closure = {body}; }}\n"))
        .collect::<String>();
    let directory = workspace(&source);
    let (output, report) = clones(directory.path(), OPTIONS);
    checked(&output, 2);
    for index in 0..bodies.len() {
        let assessment = guard(&report, &format!("rejected_{index}"));
        assert!(assessment["opportunity"].is_null());
        assert_eq!(assessment["availability"], "unsupported");
        assert!(
            assessment
                .get("closure_opportunities")
                .is_none_or(|rows| rows.as_array().unwrap().is_empty())
        );
    }
    assert_eq!(report["guard_planning"]["plans"], serde_json::json!([]));
}

#[test]
fn scoped_closure_guard_replay_preserves_strict_results_without_sources_or_tools() {
    let directory = workspace(&source());
    let (output, original) = clones(directory.path(), OPTIONS);
    checked(&output, 2);
    let (output, plain) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    checked(&output, 0);
    for field in ["matching", "accepted_pairs", "functions"] {
        assert_eq!(original[field], plain[field], "{field}");
    }
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory.path())
        .args(["clones", "--format", "json"])
        .args(OPTIONS)
        .args(["--replay-run", original["run_directory"].as_str().unwrap()])
        .env("PATH", directory.path().join("no-tools"))
        .env("CARGO", directory.path().join("no-cargo"))
        .env("RUSTC", directory.path().join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.path().join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 2);
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in [
        "analysis",
        "guard_planning",
        "edit_coordination",
        "matching",
        "accepted_pairs",
        "functions",
    ] {
        assert_eq!(original[field], replayed[field], "{field}");
    }
}
