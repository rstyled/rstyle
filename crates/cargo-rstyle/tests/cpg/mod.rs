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
