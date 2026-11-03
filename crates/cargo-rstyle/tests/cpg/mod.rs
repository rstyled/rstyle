//! `cargo rstyle cpg check` over compat.cpg documents; no compiler is involved.

use super::*;
use rstyle_core::cpg::layout::digest;
use serde_json::json;

fn example() -> Value {
    let path = workspace_root().join("crates/rstyle-core/src/cpg/testdata/example.cpg.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check(documents: &[Value], extra: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let mut args = vec!["rstyle".to_string(), "cpg".into(), "check".into()];
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

/// The split layout of a bundled run: one unit document and one document per function.
fn split(bundled: &Value) -> Vec<Value> {
    let mut unit = bundled.clone();
    unit["layout"] = json!("split");
    let functions = unit["unit"]
        .as_object_mut()
        .unwrap()
        .remove("functions")
        .unwrap();
    let unit_ref = json!({"id": unit["unit"]["id"], "digest": digest(&unit)});
    let mut documents = vec![unit.clone()];
    for function in functions.as_array().unwrap() {
        documents.push(json!({
            "schema_version": unit["schema_version"], "profile": unit["profile"], "layout": "split",
            "document": "function", "producer": unit["producer"], "coverage": unit["coverage"],
            "unit_ref": unit_ref, "function": function,
        }));
    }
    documents
}

#[test]
fn cpg_check_runs_the_style_rules_without_a_compiler() {
    let output = check(&[example()], &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        stdout(&output).contains("rstyle: 3 functions, 0 violations"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn cpg_check_reads_split_runs() {
    let output = check(&split(&example()), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["functions"], 3);
    assert_eq!(report["summary"]["complexity_measured"], 3);
}

#[test]
fn cpg_check_reports_rule_violations() {
    let mut document = example();
    let top = &mut document["unit"]["functions"][1]["signature"]["parameters"];
    for position in 2..6 {
        top.as_array_mut()
            .unwrap()
            .push(json!({"position": position, "symbol": "top.a", "type": "type:u32"}));
    }
    let output = check(&[document], &[]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout(&output).contains("fn_params"), "{}", stdout(&output));
}

#[test]
fn cpg_check_applies_an_explicit_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("rstyle.toml");
    std::fs::write(&config, "[fn_params]\nmax_params = 1\n").unwrap();
    let output = check(&[example()], &["--config", config.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout(&output).contains("fn_params"), "{}", stdout(&output));
}

#[test]
fn cpg_check_rejects_invalid_runs() {
    let mut newer = example();
    newer["profile"]["version"] = json!("0.2.0");
    let output = check(&[newer], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("this rstyle reads compat.cpg/0.1.0"));

    let mut documents = split(&example());
    documents.remove(0);
    let output = check(&documents, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("exactly one unit document, found 0"));
}

/// `cpg emit` over the lowering fixture: its directory, output and reference facts.
fn emitted() -> (TempDir, PathBuf, PathBuf) {
    let dir = fixture("lowered");
    let out = dir.path().join("cpg");
    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        stdout(&output).starts_with("1 compat.cpg documents written"),
        "{}",
        stdout(&output)
    );
    let facts = std::fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.to_string_lossy().ends_with(".facts.json"))
        .unwrap();
    (dir, out, facts)
}

#[test]
fn rustc_documents_derive_the_frontends_own_rule_facts() {
    let (dir, out, _) = emitted();
    let output = rstyle(
        dir.path(),
        &[
            "rstyle",
            "cpg",
            "compare",
            out.to_str().unwrap(),
            "--format",
            "json",
        ],
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["differences"], json!([]), "{report:#}");
    assert_eq!(report["diagnostic_differences"], json!([]));
    assert_eq!(report["functions"], 16);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn the_driver_counts_statements_by_source_shape() {
    let (_dir, _, facts) = emitted();
    let facts: Value = serde_json::from_str(&std::fs::read_to_string(facts).unwrap()).unwrap();
    let statements = |name: &str| {
        let record = facts
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name)
            .unwrap();
        record["statements"].clone()
    };
    assert_eq!(
        statements("Store::nested"),
        5,
        "a for loop ending another loop's body is a statement"
    );
    assert_eq!(
        statements("Store::countdown"),
        4,
        "a while loop's lowering adds no statement"
    );
    assert_eq!(
        statements("shapes::destructure"),
        10,
        "a destructuring assignment is one statement, also as an arm body"
    );
}

#[test]
fn compare_reports_differences_and_fails() {
    let (dir, out, facts) = emitted();
    let mut reference: Value =
        serde_json::from_str(&std::fs::read_to_string(&facts).unwrap()).unwrap();
    reference[0]["statements"] = json!(999);
    std::fs::write(&facts, serde_json::to_vec(&reference).unwrap()).unwrap();

    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "compare", out.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stdout(&output).contains("statements: 1 functions"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn emit_selects_packages_and_targets() {
    let dir = fixture("lowered");
    let out = dir.path().join("cpg");
    let manifest = dir.path().join("Cargo.toml");
    let args = [
        "rstyle",
        "cpg",
        "emit",
        "-p",
        "lowered",
        "--all-targets",
        "--manifest-path",
    ];
    let mut args: Vec<&str> = args.to_vec();
    args.extend([manifest.to_str().unwrap(), "--out", out.to_str().unwrap()]);
    let output = rstyle(dir.path(), &args);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    // The library and its test harness are two compilations.
    assert!(
        stdout(&output).starts_with("2 compat.cpg documents written"),
        "{}",
        stdout(&output)
    );

    // A second run replaces the first run's documents.
    let output = rstyle(dir.path(), &args);
    assert!(
        stdout(&output).starts_with("2 compat.cpg documents written"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn compare_rejects_directories_without_valid_documents() {
    let dir = tempfile::tempdir().unwrap();
    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "compare", dir.path().to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("has no compat.cpg documents"));

    std::fs::write(dir.path().join("unit.cpg.json"), b"{}").unwrap();
    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "compare", dir.path().to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unit.cpg.json: "));

    std::fs::write(
        dir.path().join("unit.cpg.json"),
        serde_json::to_vec(&example()).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("unit.facts.json"), b"{}").unwrap();
    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "compare", dir.path().to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unit.facts.json"));
}

#[test]
fn compare_reports_diagnostics_only_one_side_produces() {
    let (dir, out, facts) = emitted();
    let mut reference: Value =
        serde_json::from_str(&std::fs::read_to_string(&facts).unwrap()).unwrap();
    for function in reference.as_array_mut().unwrap() {
        function["params"] = json!(9);
    }
    std::fs::write(&facts, serde_json::to_vec(&reference).unwrap()).unwrap();
    let output = rstyle(
        dir.path(),
        &["rstyle", "cpg", "compare", out.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("rustc only: "),
        "{}",
        stdout(&output)
    );
}
