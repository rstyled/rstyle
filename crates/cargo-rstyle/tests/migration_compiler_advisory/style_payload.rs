use super::propagation::checked;
use super::*;

const SOURCE: &str = "pub fn wide(a:u32,b:u32)->u32 { let total=a+b; total }\npub fn caller(value:u32)->u32 { wide(value,1) }\n";
const OPTIONS: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory-only",
    "--compiler-advisory",
    "--compiler-diagnostic-scope",
    "authored",
    "--compiler-plans",
    "--fail-on-findings",
    "--compiler-advisory-config",
    "rstyle.toml",
    "--report-detail",
    "compact",
];

fn fixture() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(
        directory.path().join("rstyle.toml"),
        "[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    directory
}

fn root_capture(directory: &Path) -> Value {
    let output = rstyle(
        directory,
        &["--style-engine", "compiler", "--format", "json"],
    );
    checked(&output, 1);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary_complete"], true);
    assert_eq!(report["summary"]["functions"], 2);
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["rule_id"], "fn_params");
    assert_eq!(diagnostics[0]["actual"], 2);
    assert_eq!(diagnostics[0]["limit"], 1);
    report
}

fn snapshot(report: &Value) -> Value {
    let archive = Path::new(report["run_directory"].as_str().unwrap()).join("report.json");
    let archive: Value = serde_json::from_slice(&std::fs::read(archive).unwrap()).unwrap();
    let bundle = archive["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

fn style_snapshot(wire: &Value) {
    assert_eq!(wire["schema"], "rstyle.diagnostic.style-facts/2");
    assert_eq!(wire["phase"], "compiler-style-inputs");
    let facts = wire["facts"].as_array().unwrap();
    assert_eq!(facts.len(), 2);
    for function in facts {
        let body: rstyle_pdg::HirRef = serde_json::from_value(function["body"].clone()).unwrap();
        assert_eq!(body.krate, 0);
        assert!(body.local > 0);
        assert!(function["body"].get("children").is_none());
        assert_eq!(function["bindings"], "not_requested");
    }
}

fn complete_snapshot(style: &Value, full: &Value) {
    shared_unit_options(style, full);
    assert_eq!(full["schema"], "rstyle.diagnostic.raw-facts/4");
    assert_eq!(full["phase"], "compiler-advisory-only");
    assert_eq!(style["sources"], full["sources"]);
    for function in full["facts"].as_array().unwrap() {
        let header = style["facts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|header| header["name"] == function["name"])
            .unwrap();
        assert!(!function["body"]["children"].as_array().unwrap().is_empty());
        assert!(!function["bindings"].as_array().unwrap().is_empty());
        assert_eq!(header["body"], function["body"]["hir"]);
        for field in ["name", "source", "fn_span"] {
            assert_eq!(header[field], function[field], "{field}");
        }
        signature_identity(&header["signature"], &function["signature"]);
        for (field, value) in header["build"].as_object().unwrap() {
            if field != "frontend_options" {
                assert_eq!(value, &function["build"][field], "{field}");
            }
        }
    }
}

fn shared_unit_options(style: &Value, full: &Value) {
    let bytes = serde_json::to_vec(style).unwrap();
    let restored =
        rstyle_pdg::unit_store::read_style_snapshot_phase(&bytes, "compiler-style-inputs").unwrap();
    for key in ["invocation", rstyle_pdg::nominal_boundary::CONTEXT_KEY] {
        let first = restored[0].build.frontend_options.get(key).unwrap();
        for fact in &restored {
            assert!(std::ptr::eq(
                first,
                fact.build.frontend_options.get(key).unwrap()
            ));
            let original = full["facts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|original| original["name"] == fact.name)
                .unwrap();
            assert_eq!(first, &original["build"]["frontend_options"][key]);
        }
    }
    assert!(full["facts"][0]["build"]["frontend_options"]["invocation"].is_array());
    assert_eq!(
        style["facts"][0]["build"]["frontend_options"]["invocation"]["kind"],
        "inline"
    );
    assert_eq!(
        style["facts"][1]["build"]["frontend_options"]["invocation"]["kind"],
        "shared_reference"
    );
}

fn signature_identity(style: &Value, full: &Value) {
    let mut enriched = style.clone();
    let raw_params = enriched["params"].as_array_mut().unwrap();
    let full_params = full["params"].as_array().unwrap();
    assert_eq!(raw_params.len(), full_params.len());
    for (raw, full) in raw_params.iter_mut().zip(full_params) {
        // This fixture explicitly annotates every parameter as u32. Numeric
        // capture leaves pattern spelling empty; concrete capture copies it.
        assert_eq!(raw["ty"]["spelling"], "u32");
        assert_eq!(raw["pat"]["ty"]["spelling"], "");
        assert_eq!(full["pat"]["ty"]["spelling"], full["ty"]["spelling"]);
        raw["pat"]["ty"]["spelling"] = full["pat"]["ty"]["spelling"].clone();
    }
    assert_eq!(&enriched, full);
}

fn offline(directory: &Path, report: &Value, extra: &[&str]) -> Output {
    let mut options = vec!["clones", "--format", "json"];
    options.extend_from_slice(OPTIONS);
    if extra.contains(&"--report-detail") {
        options.truncate(options.len() - 2);
    }
    replay(directory, report, &options, extra)
}

fn replay(directory: &Path, report: &Value, options: &[&str], extra: &[&str]) -> Output {
    diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
            .current_dir(directory)
            .args(options)
            .args(["--replay-run", report["run_directory"].as_str().unwrap()])
            .args(extra)
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RUSTC", directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}

fn root_replay(directory: &Path, original: &Value) {
    const ROOT: &[&str] = &["--style-engine", "compiler", "--format", "json"];
    let output = replay(directory, original, ROOT, &[]);
    checked(&output, 1);
    let restored: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in [
        "diagnostics",
        "summary",
        "functions",
        "fix_groups",
        "compiler_advisory",
        "unit_contexts",
        "observed_units",
        "build_contexts",
        "summary_complete",
        "planning_input_complete",
        "planning_required_edits_complete",
        "matching_execution",
        "strict_assessment_execution",
    ] {
        assert_eq!(original[field], restored[field], "{field}");
    }
    for extra in [
        &["-p", "fixture"][..],
        &["--all-targets"][..],
        &["--manifest-path", "Cargo.toml"][..],
    ] {
        let output = replay(directory, original, ROOT, extra);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("original selection"));
    }
    let output = replay(directory, original, &["--style-engine", "legacy"], &[]);
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires --style-engine compiler"));
    let missing = serde_json::json!({"run_directory":directory.join("missing-run")});
    checked(&replay(directory, &missing, ROOT, &[]), 2);

    std::fs::write(
        directory.join("relaxed.toml"),
        "[fn_params]\nmax_params=2\n",
    )
    .unwrap();
    let output = replay(directory, original, ROOT, &["--config", "relaxed.toml"]);
    checked(&output, 0);
    let relaxed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(relaxed["summary"]["functions"], 2);
    assert_eq!(relaxed["summary"]["violations"], 0);
    assert_eq!(relaxed["diagnostics"], serde_json::json!([]));
    assert_eq!(relaxed["summary_complete"], true);
}

fn projected_numeric(root: &Value, replay: &Value) {
    let evaluated = &replay["compiler_advisory"]["units"][0]["evaluation"];
    for field in ["diagnostics", "summary", "functions"] {
        assert_eq!(root[field], evaluated[field], "{field}");
    }
    let groups: Vec<_> = replay["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|group| &group["evaluation"])
        .collect();
    assert_eq!(root["fix_groups"], serde_json::json!(groups));
    assert_eq!(root["compiler_advisory"], replay["compiler_advisory"]);
}

fn unavailable_body_consumers(directory: &Path, original: &Value) {
    for extra in [
        &["--report-detail", "full"][..],
        &["--source-rules"][..],
        &["--compiler-patterns"][..],
    ] {
        let output = offline(directory, original, extra);
        checked(&output, 2);
        let text = format!(
            "{}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains("style-facts")
                || text.contains("compiler-style-inputs")
                || text.contains("style inputs")
                || text.contains("style-only")
                || text.contains("full body"),
            "missing unavailable-payload explanation: {text}"
        );
        if let Ok(report) = serde_json::from_slice::<Value>(&output.stdout) {
            assert!(
                report["handoff_failures"]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
                    || report["capture_complete"] == false,
                "missing explicit failure: {report}"
            );
        }
    }
}

#[test]
fn root_numeric_payload_is_distinct_from_full_clones_and_replays_without_sources_or_tools() {
    let directory = fixture();
    let first = root_capture(directory.path());
    let first_wire = snapshot(&first);
    style_snapshot(&first_wire);
    let (output, full) = clones(directory.path(), OPTIONS);
    checked(&output, 1);
    let full_wire = snapshot(&full);
    complete_snapshot(&first_wire, &full_wire);
    projected_numeric(&first, &full);
    let repeated = root_capture(directory.path());
    style_snapshot(&snapshot(&repeated));
    assert_ne!(first["run_directory"], repeated["run_directory"]);
    for field in ["diagnostics", "summary", "functions", "fix_groups"] {
        assert_eq!(first[field], repeated[field], "{field}");
    }
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let output = offline(directory.path(), &repeated, &[]);
    checked(&output, 1);
    let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
    projected_numeric(&repeated, &replay);
    root_replay(directory.path(), &repeated);
    unavailable_body_consumers(directory.path(), &repeated);
}
