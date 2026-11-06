//! `cargo rstyle cpg analyze`: every selected analysis in one pass over a run's documents.
use super::*;

fn with_documents(route: &str, documents: &[Value], extra: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let mut args = vec!["rstyle".to_string(), "cpg".into(), route.into()];
    args.extend(extra.iter().map(|arg| arg.to_string()));
    for (index, document) in documents.iter().enumerate() {
        let path = dir.path().join(format!("{index}.json"));
        std::fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
        args.push(path.display().to_string());
    }
    Command::new(cargo_bin("cargo-rstyle"))
        .args(&args)
        .output()
        .unwrap()
}

fn json_report(output: &Output) -> Value {
    assert!(
        output.status.code().is_some_and(|code| code < 2),
        "{output:?}"
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn analyzed(extra: &[&str]) -> Value {
    let args = [&["--format", "json"], extra].concat();
    json_report(&with_documents("analyze", &[example()], &args))
}

/// A config every example run violates, in a directory that outlives the call.
fn strict_config() -> (TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("rstyle.toml");
    std::fs::write(&config, "[fn_params]\nmax_params = 1\n").unwrap();
    let config = config.display().to_string();
    (dir, config)
}

/// A flow report's rows by function: a run's report lists them in canonical order.
fn by_function(flow: &Value) -> std::collections::BTreeMap<String, Value> {
    flow["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["function"].as_str().unwrap().to_string(), row.clone()))
        .collect()
}

#[test]
fn one_pass_reports_what_the_separate_commands_report() {
    let check = json_report(&with_documents(
        "check",
        &[example()],
        &["--format", "json"],
    ));
    let patterns = json_report(&with_documents(
        "patterns",
        &[example()],
        &["--format", "json", "--complexity"],
    ));
    let flow = json_report(&with_documents("flow", &[example()], &["--format", "json"]));
    let unified = analyzed(&[]);
    assert_eq!(unified["check"], check);
    assert_eq!(unified["patterns"], patterns["evaluation"]);
    assert_eq!(unified["runtime_bounds"], patterns["runtime_bounds"]);
    assert_eq!(by_function(&unified["flow"]), by_function(&flow));
}

#[test]
fn a_filter_leaves_the_other_analyses_absent() {
    let check = analyzed(&["--filter", "check"]);
    assert!(check["check"].is_object());
    assert!(check.get("patterns").is_none() && check.get("runtime_bounds").is_none());
    assert!(check.get("flow").is_none());

    let flow = analyzed(&["--filter", "flow"]);
    assert!(!flow["flow"]["functions"].as_array().unwrap().is_empty());
    assert!(flow.get("check").is_none() && flow.get("patterns").is_none());
    assert!(flow.get("runtime_bounds").is_none());

    let mined = analyzed(&["--filter", "patterns"]);
    assert!(mined["patterns"].is_object());
    assert!(mined.get("check").is_none() && mined.get("runtime_bounds").is_none());

    let bounds = analyzed(&["--filter", "bounds"]);
    assert!(bounds["runtime_bounds"].is_object());
    assert!(bounds.get("check").is_none() && bounds.get("patterns").is_none());

    let both = analyzed(&["--filter", "check,bounds"]);
    assert_eq!(both, analyzed(&["--filter", "bounds", "--filter", "check"]));
    assert!(both["check"].is_object() && both["runtime_bounds"].is_object());
}

#[test]
fn a_filtered_analysis_reports_what_it_reports_in_the_whole_run() {
    let whole = analyzed(&[]);
    assert_eq!(analyzed(&["--filter", "check"])["check"], whole["check"]);
    assert_eq!(
        analyzed(&["--filter", "patterns"])["patterns"],
        whole["patterns"]
    );
    assert_eq!(
        analyzed(&["--filter", "bounds"])["runtime_bounds"],
        whole["runtime_bounds"]
    );
    assert_eq!(analyzed(&["--filter", "flow"])["flow"], whole["flow"]);
}

#[test]
fn flags_that_cannot_be_honored_fail_before_anything_runs() {
    for (extra, expected) in [
        (vec!["--filter", "clones"], "Unknown analysis"),
        (vec!["--filter", "go:calls"], "use `rstyle go`"),
        (
            vec!["--filter", "check", "--fail-on-candidates"],
            "--fail-on-candidates needs the patterns analysis",
        ),
        (
            vec!["--filter", "bounds", "--sarif-dir", "unused"],
            "--sarif-dir needs check or patterns",
        ),
        (
            vec!["--filter", "flow", "--sarif-dir", "unused"],
            "--sarif-dir needs check or patterns",
        ),
    ] {
        let output = with_documents("analyze", &[example()], &extra);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{stderr}");
    }
}

#[test]
fn the_exit_code_is_the_worst_result_of_what_ran() {
    let (_dir, config) = strict_config();
    let output = with_documents("analyze", &[example()], &["--config", &config]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let output = with_documents(
        "analyze",
        &[example()],
        &["--config", &config, "--filter", "bounds"],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let output = with_documents("analyze", &[example(), example()], &[]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn sarif_is_one_file_per_selected_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let sarif = dir.path().join("sarif");
    let (_config_dir, config) = strict_config();
    let output = with_documents(
        "analyze",
        &[example()],
        &["--config", &config, "--sarif-dir", sarif.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let check: Value =
        serde_json::from_slice(&std::fs::read(sarif.join("check.sarif")).unwrap()).unwrap();
    assert_eq!(check["version"], "2.1.0");
    let run = &check["runs"][0];
    assert_eq!(run["automationDetails"]["id"], "rstyle/check/");
    assert!(
        run["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|result| result["ruleId"] == "fn_params"),
        "{run}"
    );
    let patterns: Value =
        serde_json::from_slice(&std::fs::read(sarif.join("patterns.sarif")).unwrap()).unwrap();
    assert_eq!(
        patterns["runs"][0]["automationDetails"]["id"],
        "rstyle/patterns/"
    );
    assert_eq!(patterns["runs"].as_array().unwrap().len(), 1);
    let report: Value =
        serde_json::from_slice(&std::fs::read(sarif.join("report.sarif")).unwrap()).unwrap();
    assert_eq!(report["runs"].as_array().unwrap().len(), 2);

    let only = dir.path().join("only");
    let output = with_documents(
        "analyze",
        &[example()],
        &["--filter", "check", "--sarif-dir", only.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(only.join("check.sarif").exists() && !only.join("patterns.sarif").exists());
}

#[test]
fn a_stream_on_the_pool_matches_files_and_keeps_its_receipt() {
    let mut bytes = vec![0x1e];
    serde_json::to_writer(&mut bytes, &example()).unwrap();
    bytes.extend_from_slice(b"\n\x1enull\n");
    let output = super::stdin::command("analyze", &bytes, &["--format", "json"]);
    let streamed = json_report(&output);
    let files = analyzed(&[]);
    assert_eq!(streamed["check"], files["check"]);
    assert_eq!(streamed["patterns"], files["patterns"]);
    assert_eq!(streamed["runtime_bounds"], files["runtime_bounds"]);
    assert_eq!(streamed["capture_receipt"]["status"], "accepted");
    assert_eq!(
        streamed["capture_receipt"]["transport"]["received_documents"],
        1
    );
}

#[test]
fn a_stream_with_no_terminal_record_is_rejected() {
    let mut bytes = vec![0x1e];
    serde_json::to_writer(&mut bytes, &example()).unwrap();
    bytes.push(b'\n');
    let output = super::stdin::command("analyze", &bytes, &["--format", "json"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert_eq!(json_receipt_status(&output), "rejected");
}

#[test]
fn a_stream_with_a_function_seen_twice_is_rejected_across_workers() {
    let mut bytes = Vec::new();
    for _ in 0..2 {
        bytes.push(0x1e);
        serde_json::to_writer(&mut bytes, &example()).unwrap();
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(b"\x1enull\n");
    let output = super::stdin::command("analyze", &bytes, &["--format", "json"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stdout(&output).contains("appears more than once"),
        "{output:?}"
    );
}

fn json_receipt_status(output: &Output) -> String {
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    receipt["status"].as_str().unwrap().to_string()
}
