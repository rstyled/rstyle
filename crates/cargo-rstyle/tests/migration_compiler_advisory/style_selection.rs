use super::propagation::{checked, workspace};
use super::*;

const WIDE: &str = "pub fn wide(a:u32,b:u32)->u32 {a+b}\n";

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut arguments = vec!["--style-engine", "compiler", "--format", "json"];
    arguments.extend_from_slice(extra);
    let output = rstyle(directory, &arguments);
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output, report)
}

fn fixture() -> TempDir {
    let directory = workspace("pub fn leaf() {}\n");
    std::fs::write(directory.path().join("caller/src/lib.rs"), WIDE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

#[test]
fn root_compiler_style_selects_members_without_leaking_other_workspace_findings() {
    let directory = fixture();
    for (packages, code, functions, findings) in [
        (vec!["-p", "leaf"], 0, 1, 0),
        (vec!["-p", "caller"], 1, 1, 1),
        (vec![], 1, 2, 1),
    ] {
        let (output, report) = capture(directory.path(), &packages);
        checked(&output, code);
        assert_eq!(report["capture_complete"], true);
        assert_eq!(report["summary_complete"], true);
        assert_eq!(report["summary"]["functions"], functions);
        assert_eq!(report["summary"]["violations"], findings);
        assert_eq!(report["diagnostics"].as_array().unwrap().len(), findings);
        assert_eq!(
            report["compiler_advisory"]["units"]
                .as_array()
                .unwrap()
                .len(),
            functions
        );
        if findings == 1 {
            let diagnostic = &report["diagnostics"][0];
            assert_eq!(diagnostic["rule_id"], "fn_params");
            assert_eq!(diagnostic["actual"], 2);
            assert_eq!(diagnostic["limit"], 1);
            assert!(
                diagnostic["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("caller/src/lib.rs")
            );
        }
    }
}

#[test]
fn root_compiler_style_rejects_missing_and_malformed_explicit_configuration() {
    let directory = fixture();
    let path = directory.path().join("bad.toml");
    std::fs::write(&path, "[fn_params]\nmax_params='not a number'\n").unwrap();
    for name in ["missing.toml", "bad.toml"] {
        let output = rstyle(
            directory.path(),
            &[
                "--style-engine",
                "compiler",
                "--format",
                "json",
                "--config",
                name,
            ],
        );
        checked(&output, 2);
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        if name == "bad.toml" {
            assert!(String::from_utf8_lossy(&output.stderr).contains("invalid type"));
        }
    }
    assert!(!directory.path().join("missing.toml").exists());
}

#[test]
fn root_compiler_style_source_exclusions_remove_findings_metrics_and_edit_groups() {
    let directory = fixture();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "exclude=['caller/**']\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["summary_complete"], true);
    assert_eq!(report["summary"]["functions"], 1);
    assert_eq!(report["summary"]["violations"], 0);
    for field in ["diagnostics", "functions", "fix_groups"] {
        assert_eq!(report[field], serde_json::json!([]));
    }
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    let caller = units
        .iter()
        .find(|unit| unit["package_root"].as_str().unwrap().ends_with("caller"))
        .unwrap();
    assert_eq!(caller["evaluation"]["summary"]["functions"], 0);
}

#[test]
fn root_compiler_style_keeps_uncaptured_requested_effect_inputs_incomplete() {
    let directory = workspace("pub fn leaf() {let _=std::fs::read(\"/unused\");}\n");
    let (output, report) = capture(directory.path(), &["-p", "caller"]);
    checked(&output, 2);
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["summary_complete"], false);
    assert_eq!(report["summary"]["functions"], 1);
    assert_eq!(report["compiler_advisory"]["complete"], false);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 1);
    assert_eq!(
        units[0]["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    assert_eq!(units[0]["diagnostic_scope"]["input_complete"], false);
    assert_eq!(
        units[0]["diagnostic_scope"]["effect_inputs_complete"],
        false
    );
    assert_eq!(report["planning_input_complete"], false);
}
