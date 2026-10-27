//! Exact original base64 0.23.1 encoded_len; expectations are mathematical
//! length/overflow behavior, independent of the extraction implementation.
use super::float_kernels::{function, reject_resealed};
use super::*;

const ORIGINAL: &str = r#"#[must_use]
pub const fn encoded_len(bytes_len: usize, padding: bool) -> Option<usize> {
    let rem = bytes_len % 3;

    let complete_input_chunks = bytes_len / 3;
    // `?` is disallowed in const, and `let Some(_) = _ else` requires 1.65.0, whereas this
    // messier syntax works on 1.48
    let complete_chunk_output =
        if let Some(complete_chunk_output) = complete_input_chunks.checked_mul(4) {
            complete_chunk_output
        } else {
            return None;
        };

    if rem > 0 {
        if padding {
            complete_chunk_output.checked_add(4)
        } else {
            let encoded_rem = match rem {
                1 => 2,
                // only other possible remainder is 2
                // can't use a separate _ => unreachable!() in const fns in ancient rust versions
                _ => 3,
            };
            complete_chunk_output.checked_add(encoded_rem)
        }
    } else {
        Some(complete_chunk_output)
    }
}"#;

const CASES: &str = r#"
pub fn checked_add(a: usize, b: usize) -> Option<usize> { a.checked_add(b) }
pub fn checked_mul(a: usize, b: usize) -> Option<usize> { a.checked_mul(b) }
pub fn bind(a: usize, b: usize) -> Option<usize> {
    if let Some(value) = a.checked_add(b) { Some(value) } else { None }
}
pub fn saved(mut a: usize, b: usize) -> Option<usize> {
    let result = a.checked_add(b); a = 0;
    if let Some(value) = result { Some(value) } else { None }
}
pub fn two(a: usize, b: usize) -> Option<usize> {
    let first = a.checked_add(1); let second = b.checked_mul(2);
    if let Some(value) = first { Some(value) } else { second }
}
pub fn caller(n: usize, padding: bool) -> Option<usize> { encoded_len(n, padding) }
pub fn same(n: usize, padding: bool) -> Option<usize> { encoded_len(n, padding) }
pub fn mutable(n: usize) -> Option<usize> {
    if let Some(mut value) = n.checked_add(1) { value = 2; Some(value) } else { None }
}
pub fn by_ref(n: usize) -> Option<usize> {
    if let Some(ref value) = n.checked_add(1) { Some(*value) } else { None }
}
pub fn at(n: usize) -> Option<usize> {
    if let Some(value @ 0..=8) = n.checked_add(1) { Some(value) } else { None }
}
pub fn none_pattern(n: usize) -> Option<usize> {
    if let None = n.checked_add(1) { None } else { Some(n) }
}
pub fn nested(n: usize) -> Option<usize> {
    if let Some(Some(value)) = Some(n.checked_add(1)) { Some(value) } else { None }
}
pub fn missing_else(n: usize) -> Option<usize> {
    if let Some(value) = n.checked_add(1) { let _saved = value; } None
}
pub fn hidden_effect(n: usize) -> Option<usize> {
    if let Some(value) = n.checked_add(1) { Some(value) } else { panic!("overflow effect") }
}
"#;

fn captured(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{ORIGINAL}\n{CASES}"),
    )
    .unwrap();
    let mut flags = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        flags.extend(["--target", target]);
    }

    let (output, report) = clones(directory.path(), &flags);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn original_checked_length_composes_if_let_early_return_and_option_branches() {
    assert_eq!(
        rstyle_pdg::concrete_types::digest(ORIGINAL.as_bytes()),
        "sha256:3bd1a0a0a073b862443aedd744a26d71d86bba3be8e70074dfdbf23de3ba5e6d"
    );
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let (_directory, report) = captured(Some(target), no_std);
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let positives = [
        "encoded_len",
        "checked_add",
        "checked_mul",
        "bind",
        "saved",
        "two",
        "caller",
        "same",
    ];
    for entry in report["functions"].as_array().unwrap() {
        let name = entry["facts"]["name"].as_str().unwrap();
        assert_eq!(
            entry["outcome"],
            if positives.contains(&name) {
                "admitted"
            } else {
                "excluded"
            },
            "{name}: {}",
            entry["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }

    let left = &function(report, "caller")["key"];
    let right = &function(report, "same")["key"];
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 28);
    assert!(
        pairs
            .iter()
            .any(|pair| ((&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left))
                && pair["decision"] == "Accepted")
    );
}

#[test]
fn checked_length_let_chains_remain_excluded() {
    let directory = clone_workspace();
    let manifest = directory.path().join("Cargo.toml");
    let bytes = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("2021", "2024");
    std::fs::write(manifest, bytes).unwrap();
    let source = "pub fn chained(n: usize) -> Option<usize> { if let Some(value) = n.checked_add(1) && value > 0 { Some(value) } else { None } }";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    assert_eq!(function(&report, "chained")["outcome"], "excluded");
}

#[test]
fn checked_lengths_match_independent_integer_length_formula_at_overflow_boundaries() {
    let directory = clone_workspace();
    let oracle = r#"
#[test] fn mathematical_lengths() {
    let limit = usize::MAX as u128;
    let mut inputs: Vec<usize> = (0..=4096).collect();
    for center in [usize::MAX, (usize::MAX / 4) * 3, usize::MAX / 3] {
        for delta in 0..=8 {
            if let Some(n) = center.checked_sub(delta) { inputs.push(n); }
            if let Some(n) = center.checked_add(delta) { inputs.push(n); }
        }
    }
    for n in inputs {
        for padding in [false, true] {
            let wide = n as u128;
            let rem = wide % 3;
            let extra = if rem == 0 { 0 } else if padding { 4 } else { rem + 1 };
            let expected = (wide / 3) * 4 + extra;
            let expected = if expected > limit { None } else { Some(expected as usize) };
            assert_eq!(encoded_len(n, padding), expected, "{n} {padding}");
        }
    }
}
"#;
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, format!("{ORIGINAL}\n{oracle}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021", "-C", "overflow-checks=on"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn checked_length_if_let_rejects_resealed_initializer_guard_binding_and_return_changes() {
    let (_directory, report) = captured(None, false);
    for mutation in [
        "initializer",
        "selector_annotation",
        "payload_guard",
        "false_edge",
        "orphan_selector",
        "alternative",
        "return",
    ] {
        let entry = function(&report, "two");
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        corrupt_graph(&mut bundle["graphs"][root], mutation);
        reject_resealed(bundle, mutation);
    }
    for mutation in [
        "authority",
        "operand",
        "mutable_binding",
        "binding_identity",
        "missing_initializer",
    ] {
        let entry = function(&report, "bind");
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        assert!(corrupt_source(
            &mut bundle["functions"][index]["body"],
            mutation
        ));
        reject_resealed(bundle, mutation);
    }
    reject_historical(function(&report, "encoded_len"));
    reject_historical(function(&report, "bind"));
    reject_historical(function(&report, "checked_add"));
}

#[test]
fn checked_length_arithmetic_semantics_require_exact_overflow_and_evaluation_order() {
    let (_directory, report) = captured(None, false);
    for mutation in ["overflow", "arithmetic", "evaluation_order"] {
        let entry = function(&report, "checked_add");
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        let node = bundle["graphs"][root]["pdg"]["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["attributes"]["construct"] == "data.length.checked-add-usize")
            .unwrap();
        let semantics = &mut node["annotations"]["rstyle.data:checked-length"];
        match mutation {
            "overflow" => semantics["overflow"] = "wrap".into(),
            "arithmetic" => semantics["arithmetic"] = "multiplication".into(),
            "evaluation_order" => semantics["evaluation_order"] = serde_json::json!([1, 0]),
            _ => unreachable!(),
        }
        reject_resealed(bundle, mutation);
    }
}

fn node_id(graph: &Value, construct: &str) -> Value {
    graph["pdg"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["attributes"]["construct"] == construct)
        .unwrap()["id"]
        .clone()
}

fn corrupt_graph(graph: &mut Value, mutation: &str) {
    let selector = node_id(graph, "if.option-some");
    let payload = node_id(graph, "data.option-value-payload");
    if mutation == "false_edge" || mutation == "return" {
        let kind = if mutation == "return" {
            "return"
        } else {
            "false"
        };
        let edge = graph["cfg"]["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["kind"] == kind)
            .unwrap();
        edge["kind"] = "normal".into();
        return;
    }
    if mutation == "initializer" {
        let second = node_id(graph, "data.length.checked-mul-usize");
        let edge = graph["pdg"]["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["target"] == payload && edge["subkind"] == "value_operand")
            .unwrap();
        edge["source"] = second;
        return;
    }
    corrupt_control(graph, mutation, &selector, &payload);
}

fn corrupt_control(graph: &mut Value, mutation: &str, selector: &Value, payload: &Value) {
    let nodes = graph["pdg"]["nodes"].as_array_mut().unwrap();
    match mutation {
        "selector_annotation" => {
            nodes
                .iter_mut()
                .find(|node| &node["id"] == selector)
                .unwrap()["annotations"]
                .as_object_mut()
                .unwrap()
                .remove("rstyle.data:checked-option");
        }
        "payload_guard" => {
            nodes
                .iter_mut()
                .find(|node| &node["id"] == payload)
                .unwrap()["annotations"]["compat.rust.hir-r0:guards"] = serde_json::json!([[]])
        }
        "orphan_selector" => {
            let mut extra = nodes
                .iter()
                .find(|node| &node["id"] == selector)
                .unwrap()
                .clone();
            extra["id"] = "orphan-if-let".into();
            nodes.push(extra);
        }
        "alternative" => {
            let site = &mut graph["pdg"]["extensions"]["compat.rust.hir-r0:value-sites"][0];
            site["alternatives"][0]["value_pdg_node_id"] =
                site["alternatives"][1]["value_pdg_node_id"].clone();
        }
        _ => unreachable!(),
    }
}

fn corrupt_source(node: &mut Value, mutation: &str) -> bool {
    if mutation == "missing_initializer" && node["kind"] == "expr_let" {
        node["children"].as_array_mut().unwrap().pop();
        return true;
    }
    if mutation == "authority" && node["kind"] == "expr_method_call" {
        let mut callee: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        callee["definition_hash"] = "unregistered".into();
        node["res"]["description"] = serde_json::to_string(&callee).unwrap().into();
        return true;
    }
    if mutation == "operand" && node["kind"] == "expr_method_call" {
        node["children"].as_array_mut().unwrap().swap(0, 1);
        return true;
    }
    if node["kind"] == "pat_binding" && node["snippet"] == "value" {
        if mutation == "mutable_binding" {
            node["binding_mut"] = true.into();
        } else if mutation == "binding_identity" {
            node["binding"] = "forged-binding".into();
        } else {
            return false;
        }
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| corrupt_source(child, mutation))
}

fn reject_historical(entry: &Value) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.69.0".into();
    pair.cfg.profile.version = "3.69.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn cfg_selected_inherent_span_excludes_attributes_and_preserves_strict_ownership() {
    let directory = clone_workspace();
    let source = r#"
#[derive(Clone, Copy)] pub struct Config { enabled: bool }
#[cfg(target_pointer_width = "64")]
impl Config { pub fn enabled(&self) -> bool { self.enabled } }
#[cfg(target_pointer_width = "32")]
impl Config { pub fn enabled(&self) -> bool { !self.enabled } }
"#;
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let entry = function(&report, "Config::enabled");
    assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    super::word_classifiers::verify_admissions(&report);

    let mut bundle = shared_evidence::wire(entry);
    let index = shared_evidence::index(entry);
    let wrong = source.find("#[cfg(target_pointer_width").unwrap();
    bundle["functions"][index]["build"]["frontend_options"]["inherent_definition"]["implementation_span"]
        ["start"] = wrong.into();
    reject_resealed(bundle, "implementation span includes attributes");
}
