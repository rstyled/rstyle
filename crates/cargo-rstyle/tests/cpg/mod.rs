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
        .filter(|path| path.to_string_lossy().ends_with(".cpg.json.zst"))
        .map(|path| path.display().to_string())
        .collect();
    documents.sort();
    documents
}

/// The fixture's route, and the documents that same run wrote read with no compiler.
fn both_routes(fixture_name: &str, extra: &[&str]) -> (Value, Value) {
    routes_over(fixture(fixture_name).path(), extra)
}

/// A workspace's pattern route, and the documents that same run wrote read with no compiler.
fn routes_over(root: &Path, extra: &[&str]) -> (Value, Value) {
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

/// Each function's bounds in a report's rows, by name.
fn bound_rows(report: &Value) -> std::collections::BTreeMap<String, Value> {
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
}

/// The bounds of every unit of a compiler route, by function name.
fn route_bounds(route: &Value) -> std::collections::BTreeMap<String, Value> {
    route["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|unit| unit["runtime_bounds"].is_object())
        .flat_map(|unit| bound_rows(&unit["runtime_bounds"]))
        .collect()
}

#[test]
fn cpg_patterns_bounds_match_the_compiler_route() {
    let (route, documents) = both_routes("lowered", &["--complexity"]);
    let compiled = route_bounds(&route);
    assert!(!compiled.is_empty());
    assert_eq!(compiled, bound_rows(&documents["runtime_bounds"]));
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
        "pub fn load() -> String {\n    std::fs::read_to_string(\"x\").unwrap_or_default()\n}\n\npub fn spin(items: &[u32]) -> u32 {\n    let mut total = 0;\n    for item in items {\n        total += *item;\n    }\n    total\n}\n",
    );
    write(
        "caller/Cargo.toml",
        "[package]\nname = \"caller\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nio = { path = \"../io\" }\n",
    );
    write(
        "caller/src/lib.rs",
        "pub fn report() -> usize {\n    io::load().len()\n}\n\npub fn twice(items: &[u32]) -> u32 {\n    io::spin(items) + io::spin(items)\n}\n",
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

#[test]
fn bounds_compose_through_a_callee_in_another_unit() {
    let dir = crossing_workspace();
    let (route, documents) = routes_over(dir.path(), &["--complexity"]);
    let compiled = route_bounds(&route);
    assert_eq!(compiled, bound_rows(&documents["runtime_bounds"]));
    let composed = &compiled["twice"]["composed"]["Proven"];
    assert_eq!(
        composed["resolved_callees"].as_array().unwrap().len(),
        1,
        "{composed}"
    );
    assert!(
        composed["resolved_callees"][0]
            .as_str()
            .unwrap()
            .ends_with("/io::spin"),
        "{composed}"
    );
    // Two calls of a loop over the input: its entries count, though no loop is written here.
    assert_ne!(
        composed["loop_body_entries"],
        json!({"Const": 0}),
        "{composed}"
    );
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
        stdout(&output).contains(" compat.cpg documents written"),
        "{}",
        stdout(&output)
    );
    (dir, out)
}

/// The rule facts core derives from the documents in `out`, graphs included.
fn derived(out: &Path) -> Vec<rstyle_core::facts::FnFacts> {
    let documents: Vec<Value> = documents_in(out)
        .iter()
        .map(|path| rstyle_core::cpg::transport::read_json(Path::new(path)).unwrap())
        .collect();
    let demand = rstyle_core::cpg::records::Demand {
        graphs: true,
        bounds: false,
    };
    rstyle_core::cpg::run_records(documents, demand)
        .unwrap()
        .into_iter()
        .filter_map(rstyle_core::cpg::authored)
        .collect()
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
    let units = || {
        let documents = parsed(&documents_in(&out));
        let units: std::collections::BTreeSet<String> = documents
            .iter()
            .map(|document| document["unit"]["id"].as_str().unwrap().to_owned())
            .collect();
        (units.len(), documents.len())
    };
    let first = units();
    assert_eq!(first.0, 2, "{}", stdout(&output));

    // A second run also clears stale seals and documents in either encoding.
    std::fs::write(out.join("documents.manifest.json"), b"stale manifest").unwrap();
    std::fs::write(out.join("stale.cpg.json"), b"stale JSON").unwrap();
    std::fs::write(out.join("stale.cpg.json.zst"), b"stale Zstd").unwrap();
    let output = rstyle(dir.path(), &args);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(units(), first);
    assert!(!out.join("documents.manifest.json").exists());
    assert!(!out.join("stale.cpg.json").exists());
    assert!(!out.join("stale.cpg.json.zst").exists());
}

mod bounds;
mod build_outputs;
mod capture;
mod facts;
mod transport;

fn parsed(documents: &[String]) -> Vec<Value> {
    documents
        .iter()
        .map(|path| rstyle_core::cpg::transport::read_json(Path::new(path)).unwrap())
        .collect()
}

/// The documents a capture of `root` wrote, and those `cpg emit` writes.
fn captured_and_emitted(root: &Path) -> (Value, Vec<String>, Vec<String>) {
    let route = rstyle(root, &["--format", "json"]);
    assert_eq!(route.status.code(), Some(0), "{route:?}");
    let route: Value = serde_json::from_slice(&route.stdout).unwrap();
    let mut captured = Vec::new();
    for entry in std::fs::read_dir(route["run_directory"].as_str().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path.join("documents.manifest.json").exists() {
            captured.extend(documents_in(&path));
        }
    }
    let out = root.join("cpg");
    let emit = rstyle(root, &["cpg", "emit", "--out", out.to_str().unwrap()]);
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");

    (route, captured, documents_in(&out))
}

fn function_names(documents: &[String]) -> std::collections::BTreeSet<String> {
    parsed(documents)
        .iter()
        .flat_map(|document| document["unit"]["functions"].as_array().unwrap().clone())
        .map(|function| function["name"].as_str().unwrap().to_owned())
        .collect()
}

fn check_summary(root: &Path, documents: &[String]) -> Value {
    let mut args = vec!["cpg", "check", "--format", "json"];
    args.extend(documents.iter().map(String::as_str));
    let check = rstyle(root, &args);
    assert_eq!(check.status.code(), Some(0), "{check:?}");
    serde_json::from_slice::<Value>(&check.stdout).unwrap()["summary"].clone()
}

#[test]
fn an_emit_records_every_function_a_capture_does() {
    let dir = fixture("generated");
    let (_, captured, emitted) = captured_and_emitted(dir.path());

    let names = function_names(&emitted);
    assert_eq!(names, function_names(&captured));
    for generated in [
        "macro_writes_a_file",
        "Written::generated",
        "<Written as std::clone::Clone>::clone",
    ] {
        assert!(names.contains(generated), "{generated}: {names:?}");
    }
}

#[test]
fn an_emit_records_a_function_a_build_script_writes() {
    let dir = fixture("generated");
    let app = dir.path().join("app");
    std::fs::write(
        app.join("build.rs"),
        "fn main() {\n    let out = std::env::var(\"OUT_DIR\").unwrap();\n    \
         std::fs::write(format!(\"{out}/included.rs\"), \"pub fn included() {}\").unwrap();\n}\n",
    )
    .unwrap();
    let source = std::fs::read_to_string(app.join("src/lib.rs")).unwrap();
    std::fs::write(
        app.join("src/lib.rs"),
        format!("include!(concat!(env!(\"OUT_DIR\"), \"/included.rs\"));\n{source}"),
    )
    .unwrap();
    let out = dir.path().join("cpg");

    let emit = rstyle(dir.path(), &["cpg", "emit", "--out", out.to_str().unwrap()]);

    assert_eq!(emit.status.code(), Some(0), "{emit:?}");
    assert!(function_names(&documents_in(&out)).contains("included"));
}

#[test]
fn a_macro_writes_no_finding_but_its_effects_reach_its_callers_on_every_route() {
    let dir = fixture("generated");
    let (route, captured, emitted) = captured_and_emitted(dir.path());

    assert_eq!(route["summary_complete"], true);
    assert_eq!(route["summary"]["effect_counts"]["io"], 1, "{route}");
    assert_eq!(check_summary(dir.path(), &captured), route["summary"]);
    assert_eq!(check_summary(dir.path(), &emitted), route["summary"]);
}

#[test]
fn bounds_compose_through_what_a_derive_writes_over_emitted_documents() {
    let dir = fixture("generated");
    let (_, _, emitted) = captured_and_emitted(dir.path());
    let mut args = vec!["cpg", "patterns", "--complexity", "--format", "json"];
    args.extend(emitted.iter().map(String::as_str));

    let mined = rstyle(dir.path(), &args);

    assert_eq!(mined.status.code(), Some(0), "{mined:?}");
    let mined: Value = serde_json::from_slice(&mined.stdout).unwrap();
    let rows = mined["runtime_bounds"]["rows"].as_array().unwrap();
    let caller = rows
        .iter()
        .find(|row| row["name"] == "calls_what_a_derive_wrote")
        .unwrap();
    let resolved = caller["composed"]["Proven"]["resolved_callees"]
        .as_array()
        .unwrap();
    assert_eq!(resolved.len(), 1, "{caller}");
    assert!(
        resolved[0]
            .as_str()
            .unwrap()
            .ends_with("app::{impl#0}::clone"),
        "{caller}"
    );
}

/// A function's nodes, and the name of the symbol each node refers to.
fn named_nodes(function: &Value) -> Vec<(&Value, Option<&str>)> {
    let symbols = function["symbols"].as_array().unwrap();
    let refs = function["refs"].as_array().unwrap();
    function["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            let symbol = refs
                .iter()
                .find(|reference| {
                    reference["node"] == node["id"] && reference["target"]["kind"] == "symbol"
                })
                .and_then(|reference| {
                    symbols
                        .iter()
                        .find(|symbol| symbol["id"] == reference["target"]["id"])
                });
            (node, symbol.and_then(|symbol| symbol["name"].as_str()))
        })
        .collect()
}

fn function<'a>(documents: &'a [Value], name: &str) -> &'a Value {
    documents
        .iter()
        .flat_map(|document| document["unit"]["functions"].as_array().unwrap())
        .find(|function| function["name"] == name)
        .unwrap_or_else(|| panic!("no function {name}"))
}

#[test]
fn a_macro_writing_at_its_inputs_tokens_is_recorded_as_generated() {
    let dir = fixture("generated");
    let (_, _, emitted) = captured_and_emitted(dir.path());
    let documents = parsed(&emitted);
    let written = function(&documents, "Written::generated");

    let identifier = named_nodes(written)
        .into_iter()
        .find(|(node, name)| node["kind"] == "identifier" && *name == Some("spanned"))
        .unwrap()
        .0;
    assert_eq!(identifier["provenance"]["origin"], "macro_expansion");
    assert_eq!(identifier["provenance"]["macro"], "Spanning");
    let binding = written["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"] == "pattern_binding")
        .unwrap();
    assert_eq!(binding["provenance"]["origin"], "macro_expansion");
    let operator = written["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"] == "operator")
        .unwrap();
    assert!(operator.get("token").is_none(), "{operator}");
}

#[test]
fn a_function_an_attribute_wraps_is_measured_and_located_as_written() {
    let dir = fixture("generated");
    let (route, captured, _) = captured_and_emitted(dir.path());

    assert_eq!(
        route["summary_complete"], true,
        "{}",
        route["handoff_failures"]
    );
    let units = route["compiler_advisory"]["units"].as_array().unwrap();
    assert!(
        units
            .iter()
            .flat_map(|unit| unit["observations"].as_array().unwrap())
            .any(|observation| observation["name"] == "wrapped_by_an_attribute"),
        "the wrapped function is not measured"
    );

    let documents = parsed(&captured);
    let wrapped = function(&documents, "wrapped_by_an_attribute");
    let root = wrapped["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == wrapped["root"])
        .unwrap();
    assert_eq!(root["provenance"]["origin"], "macro_expansion");
    assert!(
        wrapped["signature"].get("provenance").is_none(),
        "{wrapped}"
    );
    let content = documents
        .iter()
        .flat_map(|document| document["unit"]["files"].as_array().unwrap())
        .find(|file| file["id"] == wrapped["span"]["file"])
        .unwrap()["content"]
        .as_str()
        .unwrap();
    let written = |field: &str| wrapped["span"][field].as_u64().unwrap() as usize;
    assert_eq!(
        &content[written("start")..written("end")],
        "pub fn wrapped_by_an_attribute(value: u32) -> u32 {\n    let doubled = value * 2;\n    doubled + 1\n}"
    );
}

#[test]
fn a_function_whose_signature_a_macro_rewrites_stays_its_authors() {
    let dir = fixture("generated");
    let (_, captured, _) = captured_and_emitted(dir.path());
    let documents = parsed(&captured);
    let rewritten = function(&documents, "rewritten_by_an_attribute");
    assert_eq!(
        rewritten["signature"]["provenance"]["origin"],
        "macro_expansion"
    );

    let (route, _) = routes_over(dir.path(), &["--complexity"]);

    assert!(
        route_bounds(&route).contains_key("rewritten_by_an_attribute"),
        "{:?}",
        route_bounds(&route).keys()
    );
}

#[test]
fn complexity_counts_what_the_author_writes_and_not_what_a_macro_does() {
    let dir = fixture("generated");
    let (_, _, emitted) = captured_and_emitted(dir.path());
    let facts = derived(&dir.path().join("cpg"));
    assert!(!emitted.is_empty());
    let scores = |name: &str| {
        let measured = facts
            .iter()
            .find(|facts| facts.name == name)
            .unwrap_or_else(|| panic!("no facts for {name}"))
            .complexity
            .as_ref()
            .unwrap();
        (measured.cyclomatic, measured.cognitive)
    };

    let plain = scores("branches_written_plainly");
    assert!(plain.0 > 1);
    assert_eq!(scores("branches_an_attribute_wraps"), plain);
    assert_eq!(scores("branches_passed_to_a_macro"), plain);
    assert_eq!(scores("branches_a_macro_writes"), (1, 0));
}

#[test]
fn a_raw_name_is_recorded_as_written() {
    let dir = fixture("generated");
    let (_, _, emitted) = captured_and_emitted(dir.path());
    let documents = parsed(&emitted);
    let raw = function(&documents, "raw");
    let content = documents
        .iter()
        .flat_map(|document| document["unit"]["files"].as_array().unwrap())
        .find(|file| file["id"] == raw["span"]["file"])
        .unwrap()["content"]
        .as_str()
        .unwrap();
    let text = |span: &Value| {
        let (start, end) = (
            span["start"].as_u64().unwrap(),
            span["end"].as_u64().unwrap(),
        );
        content[start as usize..end as usize].to_owned()
    };

    for (node, name) in named_nodes(raw) {
        if let Some(name) = name.filter(|_| node["kind"] == "identifier") {
            assert_eq!(text(&node["span"]), name, "{node}");
        }
    }
    let names: Vec<&str> = raw["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|symbol| symbol["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"r#type") && names.contains(&"r#match"),
        "{names:?}"
    );
    for symbol in raw["symbols"].as_array().unwrap() {
        if let Some(declared) = symbol.get("declared") {
            assert_eq!(text(declared), symbol["name"].as_str().unwrap(), "{symbol}");
        }
    }
}

#[test]
fn what_a_macro_passes_through_and_a_raw_name_stay_source() {
    let dir = fixture("generated");
    let (_, _, emitted) = captured_and_emitted(dir.path());
    let documents = parsed(&emitted);

    for (function_name, name) in [("passed_through", "given"), ("raw", "r#match")] {
        let found = named_nodes(function(&documents, function_name))
            .into_iter()
            .find(|(node, symbol)| node["kind"] == "identifier" && *symbol == Some(name))
            .unwrap()
            .0;
        assert_eq!(found["provenance"]["origin"], "source", "{found}");
    }
}
