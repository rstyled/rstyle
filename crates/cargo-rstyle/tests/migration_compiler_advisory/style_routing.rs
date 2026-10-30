use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn wide(a:u32,b:u32)->u32 { a+b }\n// rstyle-allow(fn_params): explicit API boundary\npub fn boundary(a:u32,b:u32)->u32 { a+b }\n";
const POLICY: &str = "[fn_params]\nmax_params=1\n";

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("rstyle.toml"), POLICY).unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--style-engine", "compiler", "--format", "json"];
    args.extend_from_slice(extra);
    let output = rstyle(directory, &args);
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output, report)
}

fn diagnostics(report: &Value) -> Vec<&Value> {
    report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["evaluation"]["diagnostics"].as_array().unwrap())
        .collect()
}

fn projected_rows(report: &Value, field: &str, contexts: &str) {
    let mut expected = Vec::new();
    for unit in report["compiler_advisory"]["units"].as_array().unwrap() {
        for row in unit["evaluation"][field].as_array().unwrap() {
            expected.push(serde_json::to_string(&(row, &unit["unit"])).unwrap());
        }
    }
    let rows = report[field].as_array().unwrap();
    let units = report[contexts].as_array().unwrap();
    assert_eq!(rows.len(), units.len());
    let mut actual: Vec<_> = rows
        .iter()
        .zip(units)
        .map(|pair| serde_json::to_string(&pair).unwrap())
        .collect();
    actual.sort();
    expected.sort();
    assert_eq!(
        actual, expected,
        "{field} must preserve evaluated rows and unit identities"
    );
}

fn root_projection(report: &Value) {
    projected_rows(report, "diagnostics", "diagnostic_units");
    projected_rows(report, "functions", "function_units");
    let planning = &report["compiler_advisory"]["planning"];
    let groups: Vec<_> = planning["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| &group["evaluation"])
        .collect();
    assert_eq!(report["fix_groups"], serde_json::json!(groups));
    assert_eq!(
        report["planning_input_complete"],
        planning["input_complete"]
    );
    assert_eq!(
        report["planning_required_edits_complete"],
        planning["required_edits_complete"]
    );
    assert_eq!(report["summary_complete"], true);
}

fn evidence_archive(report: &Value) {
    let path = Path::new(report["run_directory"].as_str().unwrap()).join("report.json");
    let archive: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(archive["compiler_advisory"], report["compiler_advisory"]);
    let facts = archive["functions"].as_array().unwrap();
    assert!(!facts.is_empty());
    assert!(facts.iter().all(|fact| fact.get("key").is_some()));
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row.get("complexity").is_some())
    );
    assert!(archive.get("diagnostic_units").is_none());
}

#[test]
fn root_style_route_uses_workspace_config_suppressions_and_finding_exit() {
    let directory = fixture(SOURCE);
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 1);
    root_projection(&report);
    evidence_archive(&report);
    assert_eq!(
        report["summary"],
        report["compiler_advisory"]["units"][0]["evaluation"]["summary"]
    );
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(diagnostics(&report).len(), 1);
    assert_eq!(diagnostics(&report)[0]["rule_id"], "fn_params");
    assert_eq!(diagnostics(&report)[0]["actual"], 2);
    assert_eq!(diagnostics(&report)[0]["limit"], 1);
    assert_eq!(
        report["compiler_advisory"]["planning"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let scope = &report["compiler_advisory"]["units"][0]["diagnostic_scope"];
    assert_eq!(scope["request"]["source_scope"], "authored");
    assert_eq!(scope["request"]["rules"], serde_json::json!([]));
    let legacy = rstyle(directory.path(), &["--format", "json"]);
    checked(&legacy, 1);
    let legacy: Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(
        legacy["diagnostics"],
        serde_json::json!(diagnostics(&report))
    );
}

#[test]
fn root_style_route_honors_explicit_config_and_ignores_pattern_ledger() {
    let directory = fixture(SOURCE);
    std::fs::write(
        directory.path().join("other.toml"),
        "[fn_params]\nmax_params=2\n",
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &["--config", "other.toml", "--ignore-file", "absent.toml"],
    );
    checked(&output, 0);
    assert!(diagnostics(&report).is_empty());
    assert_eq!(
        report["compiler_advisory"]["effective_config"]["fn_params"]["max_params"],
        2
    );
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn root_style_route_retains_invalid_suppression_and_compile_failure_exits() {
    let directory = fixture("// rstyle-allow(no_such_rule): control\npub fn clean() {}\n");
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 1);
    assert_eq!(diagnostics(&report)[0]["rule_id"], "invalid_suppression");
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn broken( {}\n").unwrap();
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(report["summary_complete"], false);
    assert!(report["summary"].is_null());
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["compiler_advisory"]["complete"], false);
}

fn build_environment_fixture(directory: &Path, value: char) {
    let names = [
        "RSTYLE_PROJECT_VALUE",
        "RSTYLE_PRODUCER_REVISION",
        "RSTYLE_RUSTC_VERSION",
        "RSTYLE_RUSTC_COMMIT",
    ];
    let mut build = String::from("fn main() {\n");
    let mut source = String::new();
    for name in names {
        build.push_str(&format!("println!(\"cargo:rustc-env={name}={value}\");\n"));
        source.push_str(&format!(
            "const _: () = assert!(env!(\"{name}\").as_bytes()[0] == b'{value}');\n"
        ));
    }
    build.push_str("}\n");
    source.push_str("pub fn clean() {}\n");
    std::fs::write(directory.join("build.rs"), build).unwrap();
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
}

#[test]
fn root_style_capture_preserves_cargo_build_script_environment_on_cold_and_warm_runs() {
    let directory = fixture("pub fn clean() {}\n");
    build_environment_fixture(directory.path(), 'A');
    let (output, cold) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(cold["capture_complete"], true);
    assert_eq!(cold["compiler_advisory"]["complete"], true);

    let (output, warm) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(warm["capture_complete"], true);
    fresh_selected_owner(&warm);

    build_environment_fixture(directory.path(), 'B');
    let (output, changed) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(changed["capture_complete"], true);
    assert_eq!(changed["compiler_advisory"]["complete"], true);
}

#[test]
fn root_style_route_preserves_test_compilation_contexts() {
    let directory =
        fixture("pub fn clean() {}\n#[cfg(test)] pub fn test_only(a:u32,b:u32)->u32 { a+b }\n");
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let (output, report) = capture(directory.path(), &["--all-targets"]);
    checked(&output, 1);
    assert_eq!(
        report["compiler_advisory"]["units"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(diagnostics(&report).len(), 1);
    root_projection(&report);
    let summaries: Vec<_> = report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| &unit["evaluation"]["summary"])
        .collect();
    for field in [
        "functions",
        "violations",
        "complexity_measured",
        "complexity_unavailable",
    ] {
        let total: u64 = summaries
            .iter()
            .map(|summary| summary[field].as_u64().unwrap())
            .sum();
        assert_eq!(report["summary"][field], total);
    }
    assert_eq!(report["summary"]["mean_density_milli"], 0);
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn root_style_summary_keeps_exact_density_totals_across_compilation_contexts() {
    let directory = fixture(
        "pub fn io() {\nlet _ = std::fs::read(\"unused\");\n\nlet _=1; let _=2; let _=3; let _=4; let _=5; let _=6; let _=7; let _=8;\n}\n#[cfg(test)] pub fn clean() {}\n",
    );
    let (output, report) = capture(directory.path(), &["--all-targets"]);
    checked(&output, 0);
    root_projection(&report);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    let mut means: Vec<_> = units
        .iter()
        .map(|unit| {
            unit["evaluation"]["summary"]["mean_density_milli"]
                .as_u64()
                .unwrap()
        })
        .collect();
    means.sort();
    assert_eq!(means, [166, 333]);
    assert_eq!(report["summary"]["functions"], 3);
    // Two IO owners each have floor(3000 / 9) = 333; the clean owner has zero.
    // Aggregating rounded unit means would incorrectly produce 221.
    assert_eq!(report["summary"]["mean_density_milli"], 222);
    assert_eq!(report["summary"]["max_density_milli"], 333);
    assert_eq!(report["summary"]["effect_counts"]["io"], 2);
}

#[test]
fn root_style_route_resolves_manifest_config_and_package_from_other_directory() {
    let directory = fixture(SOURCE);
    let elsewhere = tempfile::tempdir().unwrap();
    let manifest = directory.path().join("Cargo.toml");
    let (output, report) = capture(
        elsewhere.path(),
        &[
            "--manifest-path",
            manifest.to_str().unwrap(),
            "--package",
            "clone_fixture",
        ],
    );
    checked(&output, 1);
    assert_eq!(diagnostics(&report)[0]["limit"], 1);
    assert_eq!(report["compiler_advisory"]["complete"], true);
}

#[test]
fn style_backend_option_rejects_subcommand_misrouting() {
    let directory = fixture("pub fn clean() {}\n");
    let output = rstyle(
        directory.path(),
        &["--style-engine", "compiler", "patterns"],
    );
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("default style command only"));
}

const GENERATED_IO_CHAIN: &str = r#"
macro_rules! make {()=>{
    pub fn generated_leaf() {let _ = std::fs::read("/unused");}
    pub fn generated_middle() {generated_leaf();}
}}
make!();
pub fn authored_chain() {generated_middle();}
"#;

fn authored_inherited_finding(report: &Value) {
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert_eq!(report["matching_execution"], "not_requested");
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["complete"], true);
    assert_eq!(advisory["planning"]["input_complete"], false);
    let units = advisory["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert!(unit.get("patterns").is_none());
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 2);
    assert_eq!(unit["generated_effects"].as_array().unwrap().len(), 2);
    let scope = &unit["diagnostic_scope"];
    assert_eq!(scope["request"]["source_scope"], "authored");
    assert_eq!(scope["request"]["rules"], serde_json::json!([]));
    assert_eq!(scope["input_complete"], true);
    assert_eq!(scope["effect_inputs_complete"], true);
    assert_eq!(scope["unavailable_owners"], 0);
    let rows = diagnostics(report);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["rule_id"], "side_effect_density");
    assert_eq!(rows[0]["actual"], 3000);
    assert_eq!(rows[0]["limit"], 1);
    assert!(rows[0]["details"].as_array().unwrap().iter().any(|detail| {
        detail["count"] == 1 && detail["label"].as_str().unwrap().contains("std::fs::read")
    }));
    let observations = unit["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 1);
    assert!(
        observations[0]["name"]
            .as_str()
            .unwrap()
            .ends_with("authored_chain")
    );
    assert_eq!(observations[0]["statements"], 1);
}

fn fresh_selected_owner(report: &Value) {
    let run = Path::new(report["run_directory"].as_str().unwrap());
    let text = std::fs::read_to_string(run.join("cargo.jsonl")).unwrap();
    let owners: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| {
            row["reason"] == "compiler-artifact" && row["target"]["name"] == "clone_fixture"
        })
        .collect();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0]["fresh"], true);
}

fn replay_authored_style_without_tools(directory: &Path, original: &Value) -> (Output, Value) {
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
            "--compiler-advisory",
            "--compiler-diagnostic-scope",
            "authored",
            "--compiler-plans",
            "--fail-on-findings",
            "--report-detail",
            "compact",
            "--compiler-advisory-config",
            "rstyle.toml",
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
fn root_style_inherits_generated_io_into_authored_findings_and_replays_reused_owners() {
    let directory = fixture(GENERATED_IO_CHAIN);
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\ntransitive_depth=3\n",
    )
    .unwrap();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 1);
    authored_inherited_finding(&original);
    root_projection(&original);
    evidence_archive(&original);
    let (output, warm) = capture(directory.path(), &[]);
    checked(&output, 1);
    authored_inherited_finding(&warm);
    root_projection(&warm);
    let text = rstyle(directory.path(), &["--style-engine", "compiler"]);
    checked(&text, 1);
    let rendered = stdout(&text);
    assert!(rendered.contains("side_effect_density:"));
    assert!(rendered.contains("rstyle:"));
    assert!(rendered.contains("fix groups: 1 (0 stacked, 1 independent)"));
    assert!(!rendered.contains("land together"));
    assert!(rendered.contains("compiler style: summary complete=true, planning input complete=false, required edits complete=true"));
    fresh_selected_owner(&warm);
    assert_eq!(original["compiler_advisory"], warm["compiler_advisory"]);

    let old_run = Path::new(original["run_directory"].as_str().unwrap());
    std::fs::remove_dir_all(old_run).unwrap();
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    std::fs::remove_dir_all(directory.path().join("target/rstyle/clones/advisory-build")).unwrap();
    std::fs::remove_file(old_run.with_file_name("advisory-owners.json")).unwrap();
    let (output, replayed) = replay_authored_style_without_tools(directory.path(), &warm);
    checked(&output, 1);
    authored_inherited_finding(&replayed);
    assert_eq!(warm["compiler_advisory"], replayed["compiler_advisory"]);
}
