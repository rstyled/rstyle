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

#[test]
fn cpg_check_joins_a_runs_units() {
    // Another crate's functions: a run merges one function compiled into several units.
    let text = example()
        .to_string()
        .replace("example::", "other::")
        .replace("src/lib.rs", "other/lib.rs");
    let mut other: Value = serde_json::from_str(&text).unwrap();
    other["unit"]["id"] = json!("unit:other");
    let output = check(&[example(), other], &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["complexity_measured"], 6);
}

#[test]
fn cpg_check_rejects_a_function_read_twice() {
    let output = check(&[example(), example()], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("appears more than once"));
}

/// `cpg patterns` over documents written to a temporary directory.
fn mine(documents: &[Value], extra: &[&str]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let mut args: Vec<String> = ["rstyle", "cpg", "patterns"].map(String::from).to_vec();
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

#[test]
fn cpg_patterns_runs_without_a_compiler() {
    let output = mine(&[example()], &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["evaluation"]["signature_groups"].is_array());
    assert!(report["evaluation"]["candidates"].is_array());
    assert_eq!(report["runtime_bounds"], Value::Null);
}

#[test]
fn cpg_patterns_reports_bounds_when_asked() {
    let output = mine(&[example()], &["--format", "json", "--complexity"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let bounds = &report["runtime_bounds"];
    assert_eq!(bounds["schema"], "rstyle.cpg-runtime-bounds-report/1");
    assert_eq!(bounds["rows"].as_array().unwrap().len(), 3);

    let output = mine(&[example()], &["--complexity"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout(&output).contains("Runtime upper bounds"));
}

#[test]
fn cpg_patterns_rejects_invalid_runs() {
    let output = mine(&[example(), example()], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("appears more than once"));
}

/// The documents a directory holds, in name order.
fn documents_in(directory: &Path) -> Vec<String> {
    let mut documents: Vec<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".cpg.json"))
        .map(|path| path.display().to_string())
        .collect();
    documents.sort();
    documents
}

/// The fixture's route, and the documents that same run wrote read with no compiler.
fn both_routes(fixture_name: &str, extra: &[&str]) -> (Value, Value) {
    let dir = fixture(fixture_name);
    let root = dir.path();
    let route = rstyle(
        root,
        &[&["patterns", "--format", "json", "--no-ignore"], extra].concat(),
    );
    assert_eq!(route.status.code(), Some(0), "{route:?}");
    let route: Value = serde_json::from_slice(&route.stdout).unwrap();
    let run = Path::new(route["run_directory"].as_str().unwrap());
    let mut documents = Vec::new();
    for entry in std::fs::read_dir(run).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() && path.join("documents.manifest.json").exists() {
            documents.extend(documents_in(&path));
        }
    }
    assert!(!documents.is_empty());
    let mut args = vec!["cpg", "patterns", "--format", "json"];
    args.extend_from_slice(extra);
    args.extend(documents.iter().map(String::as_str));
    let mined = rstyle(root, &args);
    assert_eq!(mined.status.code(), Some(0), "{mined:?}");

    (route, serde_json::from_slice(&mined.stdout).unwrap())
}

#[test]
fn cpg_patterns_finds_what_the_compiler_route_finds() {
    let (route, documents) = both_routes("animals", &[]);
    let units = route["compiler_advisory"]["units"].as_array().unwrap();
    let unit = units
        .iter()
        .find(|unit| unit["patterns"].is_object())
        .unwrap();
    let evaluation = &unit["patterns"]["evaluation"];
    assert!(!evaluation["candidates"].as_array().unwrap().is_empty());
    assert_eq!(
        evaluation["candidates"],
        documents["evaluation"]["candidates"]
    );
    assert_eq!(
        evaluation["signature_groups"],
        documents["evaluation"]["signature_groups"]
    );
}

#[test]
fn cpg_patterns_bounds_match_the_compiler_route() {
    let (route, documents) = both_routes("lowered", &["--complexity"]);
    let units = route["compiler_advisory"]["units"].as_array().unwrap();
    let unit = units
        .iter()
        .find(|unit| unit["runtime_bounds"].is_object())
        .unwrap();
    let rows = |report: &Value| -> std::collections::BTreeMap<String, Value> {
        report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let evidence = json!({"result": row["result"], "composed": row["composed"],
                                      "status": row["capture_status"]});
                (row["name"].as_str().unwrap().to_owned(), evidence)
            })
            .collect()
    };
    let compiled = rows(&unit["runtime_bounds"]);
    assert!(!compiled.is_empty());
    assert_eq!(compiled, rows(&documents["runtime_bounds"]));
}

/// A workspace whose effect reaches a function only through another crate.
fn crossing_workspace() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"io\", \"caller\"]\nresolver = \"3\"\n",
    );
    write(
        "io/Cargo.toml",
        "[package]\nname = \"io\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        "io/src/lib.rs",
        "pub fn load() -> String {\n    std::fs::read_to_string(\"x\").unwrap_or_default()\n}\n",
    );
    write(
        "caller/Cargo.toml",
        "[package]\nname = \"caller\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nio = { path = \"../io\" }\n",
    );
    write(
        "caller/src/lib.rs",
        "pub fn report() -> usize {\n    io::load().len()\n}\n",
    );
    dir
}

#[test]
fn a_joint_run_propagates_effects_across_units_as_the_compiler_route_does() {
    let dir = crossing_workspace();
    let root = dir.path();
    let route = rstyle(root, &["--format", "json"]);
    assert_eq!(route.status.code(), Some(0), "{route:?}");
    let route: Value = serde_json::from_slice(&route.stdout).unwrap();
    assert_eq!(route["summary_complete"], true);

    let out = root.join("cpg");
    let emit = rstyle(root, &["cpg", "emit", "--out", out.to_str().unwrap()]);
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");
    let documents = documents_in(&out);
    assert_eq!(documents.len(), 2);
    let mut args = vec!["cpg", "check", "--format", "json"];
    args.extend(documents.iter().map(String::as_str));
    let joint = rstyle(root, &args);
    assert_eq!(joint.status.code(), Some(0), "{joint:?}");
    let joint: Value = serde_json::from_slice(&joint.stdout).unwrap();
    assert_eq!(joint["summary"], route["summary"]);

    let counts = |document: &str| -> Value {
        let single = rstyle(root, &["cpg", "check", "--format", "json", document]);
        assert_eq!(single.status.code(), Some(0), "{single:?}");
        let single: Value = serde_json::from_slice(&single.stdout).unwrap();
        single["summary"]["effect_counts"].clone()
    };
    // Each unit read alone misses what crosses between them: the caller's inherited read.
    let alone: u64 = documents
        .iter()
        .filter_map(|document| counts(document)["io"].as_u64())
        .sum();
    assert!(joint["summary"]["effect_counts"]["io"].as_u64().unwrap() > alone);
}

/// `cpg emit` over the lowering fixture: its directory and output.
fn emitted() -> (TempDir, PathBuf) {
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
    (dir, out)
}

/// The rule facts core derives from the documents in `out`.
fn derived(out: &Path) -> Vec<rstyle_core::facts::FnFacts> {
    let documents = std::fs::read_dir(out)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(".cpg.json"))
        .map(|path| serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap())
        .collect();
    let bundled = rstyle_core::cpg::layout::bundle(documents).unwrap();
    rstyle_core::cpg::all_facts(&bundled).unwrap()
}

#[test]
fn statements_are_counted_by_source_shape() {
    let (_dir, out) = emitted();
    let facts = derived(&out);
    let statements = |name: &str| facts.iter().find(|f| f.name == name).unwrap().statements;
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

#[cfg(unix)]
#[test]
fn emit_selects_packages_and_targets() {
    let dir = fixture("lowered");
    let out = dir.path().join("cpg");
    // The manifest is reached through a symlink, as macOS temporary directories
    // are (`/var` -> `/private/var`); its sources still count as workspace code.
    let links = tempfile::tempdir().unwrap();
    let alias = links.path().join("lowered");
    std::os::unix::fs::symlink(dir.path(), &alias).unwrap();
    let manifest = alias.join("Cargo.toml");
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

mod bounds;
mod facts;
