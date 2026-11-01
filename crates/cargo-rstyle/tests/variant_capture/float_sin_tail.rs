//! Original k_sin uses six exact coefficients and an i32 == 0 selector. Both
//! complete alternatives contribute f64 values; neither branch may hide effects.
use super::float_kernels::{function, reject_resealed};
use super::*;

const ROOT: &str = r#"
#[macro_use] mod macros;
cfg_if! { if #[cfg(any())] { mod never; } else { mod k_sin; mod cases; mod shadow; } }
mod k_sinf;
mod k_cosf;
pub fn tail(x: f64, y: f64, iy: i32) -> f64 { k_sin::k_sin(x, y, iy) }
pub fn same(x: f64, y: f64, iy: i32) -> f64 { k_sin::k_sin(x, y, iy) }
"#;

const CASES: &str = r#"
pub fn branch(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { x - y } else { y - x } }
pub fn reversed(x: f64, y: f64, iy: i32) -> f64 { if 0 == iy { x - y } else { y - x } }
pub fn locals(x: f64, y: f64, iy: i32) -> f64 {
    let w = x; if iy == 0 { let old = w; let w = y; old - w } else { let w = y; w - x }
}
pub fn nested(x: f64, y: f64, iy: i32, other: i32) -> f64 {
    if iy == 0 { if other == 1 { x - y } else { x + y } } else { y - x }
}
pub fn divide_then(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { x / y } else { x } }
pub fn divide_else(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { x } else { x / y } }
pub fn dead_then(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { if false { x / y } else { x } } else { y } }
pub fn dead_else(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { x } else { if true { y } else { x / y } } }
pub fn missing_else(x: f64, iy: i32) -> f64 { if iy == 0 { let _x = x; } x }
pub fn comparison(x: f64, y: f64, iy: i32) -> f64 { if x < y { x } else { y } }
pub fn borrowed(x: &f64, y: f64, iy: i32) -> f64 { if iy == 0 { *x } else { y } }
pub fn call(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { core::hint::black_box(x) } else { y } }
pub fn expanded(x: f64, y: f64, iy: i32) -> f64 { if iy == 0 { assert!(iy == 0); x } else { y } }
pub fn hidden(x: f64, y: f64, iy: i32) -> f64 {
    if iy == 0 { struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } let _guard = Hidden; x } else { y }
}
"#;

const POSITIVE: &[&str] = &[
    "k_sin::k_sin",
    "k_sinf::k_sinf",
    "k_cosf::k_cosf",
    "tail",
    "same",
    "cases::branch",
    "cases::reversed",
    "cases::locals",
    "cases::nested",
];

fn capture(boundary: bool) -> (TempDir, Value) {
    capture_context(boundary, None, false)
}

fn capture_context(boundary: bool, target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let root = format!(
        "{}{ROOT}{}",
        if no_std { "#![no_std]\n" } else { "" },
        if boundary {
            "\nuse core::{f32, f64}; mod unrelated; pub use unrelated::*;\n"
        } else {
            ""
        }
    );
    let directory = super::cfg_if_modules::workspace(&root, super::cfg_if_modules::CFG_IF);
    let cases = format!(
        "const C: f64 = 1.0;\n{}",
        CASES.replace("-> f64 {", "-> f64 { let coefficient = C;")
    );
    for (path, source) in [
        ("k_sin.rs", include_str!("float_kernels/k_sin.rs")),
        ("cases.rs", cases.as_str()),
        (
            "shadow.rs",
            "const C: f64 = 1.0; type i32 = core::primitive::i32; pub fn shadow_guard(x: f64, y: f64, iy: i32) -> f64 { let coefficient = C; if iy == 0 { x - y } else { y - x } }",
        ),
        (
            "unrelated.rs",
            r#"pub struct Noise; impl Drop for Noise { fn drop(&mut self) { core::hint::black_box(()); } } pub trait Trait { fn noise(&self) -> &str; } impl Trait for Noise { fn noise(&self) -> &str { "unrelated" } } macro_rules! hidden { () => { pub struct Generated; }; } hidden!();"#,
        ),
    ] {
        std::fs::write(directory.path().join("src").join(path), source).unwrap();
    }

    let mut arguments = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        arguments.extend(["--target", target]);
    }

    let (output, report) = clones(directory.path(), &arguments);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn original_sin_tail_has_complete_alternatives_and_identical_callers() {
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
            let (_directory, report) = capture_context(false, Some(target), no_std);
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    for entry in report["functions"].as_array().unwrap() {
        let name = entry["facts"]["name"].as_str().unwrap();
        if POSITIVE.contains(&name) {
            assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        }
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

    let left = &function(report, "tail")["key"];
    let right = &function(report, "same")["key"];
    assert!(
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pair| ((&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left))
                && pair["decision"] == "Accepted")
    );
    check_original(function(report, "k_sin::k_sin"));
}

fn check_original(entry: &Value) {
    let mut constants = Vec::new();
    collect_constants(&entry["facts"]["body"], &mut constants);
    constants.sort();
    constants.dedup();
    assert_eq!(constants, ["S1", "S2", "S3", "S4", "S5", "S6"]);
    let (unit, index) = data_unit(entry);
    let pair = unit.pair(index).unwrap();
    let sites = pair.pdg.extensions["compat.rust.hir-r0:value-sites"]
        .as_array()
        .unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0]["alternatives"].as_array().unwrap().len(), 2);
}

fn collect_constants(node: &Value, names: &mut Vec<String>) {
    if node["res"]["kind"] == "resolved_scalar_constant" {
        let record: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        names.push(
            record["definition"]
                .as_str()
                .unwrap()
                .rsplit("::")
                .next()
                .unwrap()
                .to_owned(),
        );
    }
    for child in node["children"].as_array().unwrap() {
        collect_constants(child, names);
    }
}

#[test]
fn sin_tail_dependency_boundary_checks_both_branches_and_historical_policy() {
    let (_directory, report) = capture(true);
    for name in [
        "k_sin::k_sin",
        "cases::branch",
        "cases::reversed",
        "cases::locals",
        "cases::nested",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in [
        "cases::divide_then",
        "cases::divide_else",
        "cases::dead_then",
        "cases::dead_else",
        "cases::missing_else",
        "cases::comparison",
        "cases::borrowed",
        "cases::call",
        "cases::expanded",
        "cases::hidden",
    ] {
        assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(
        function(&report, "shadow::shadow_guard")["outcome"],
        "excluded"
    );
    reject_historical(function(&report, "k_sin::k_sin"));
    sin_tail_resealed_guards_alternatives_and_saved_values_are_not_authority(&report);
}

fn sin_tail_resealed_guards_alternatives_and_saved_values_are_not_authority(report: &Value) {
    for mutation in [
        "condition_op",
        "condition_literal",
        "condition_type",
        "condition_binding",
        "binding_inventory",
        "condition_order",
        "source_clause",
        "source_body",
        "saved_local",
    ] {
        reject_changed_facts(report, mutation);
    }
    for mutation in [
        "swap_alternatives",
        "drop_alternative",
        "guard",
        "merge",
        "return",
    ] {
        let entry = function(report, "k_sin::k_sin");
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        corrupt_graph(&mut bundle["graphs"][root], mutation);
        reject_resealed(bundle, mutation);
    }
}

fn reject_changed_facts(report: &Value, mutation: &str) {
    let name = if mutation == "saved_local" {
        "cases::locals"
    } else {
        "cases::nested"
    };
    let entry = function(report, name);
    let mut bundle = shared_evidence::wire(entry);
    let index = shared_evidence::index(entry);
    if mutation == "source_clause" {
        change_source_clause(&mut bundle, index);
    } else {
        let facts = &mut bundle["functions"][index];
        match mutation {
            "binding_inventory" => swap_binding_inventory(facts),
            "condition_binding" => {
                let replacement = path_resolution(&facts["body"], "other").unwrap();
                assert!(replace_guard_formal(&mut facts["body"], &replacement));
            }
            _ => assert!(corrupt_source(&mut facts["body"], mutation)),
        }
    }
    reject_resealed(bundle, mutation);
}

fn change_source_clause(bundle: &mut Value, index: usize) {
    let reference = &bundle["functions"][index]["source"];
    let snapshot = reference["snapshot"].as_u64().unwrap() as usize;
    let start = reference["range"]["start"].as_u64().unwrap() as usize;
    let end = reference["range"]["end"].as_u64().unwrap() as usize;
    let original = &bundle["sources"][snapshot].as_str().unwrap()[start..end];
    let changed = original.replacen("iy == 0", "iy == 1", 1);
    assert_ne!(changed, original);
    let digest = rstyle_pdg::concrete_types::digest(changed.as_bytes());
    let source = shared_evidence::replace_source(bundle, index, &changed, &digest);
    bundle["functions"][index]["source"] = source;
    bundle["functions"][index]["build"]["frontend_options"]["source_correspondence"]["source_digest"] =
        digest.into();
    change_guard_literal(&mut bundle["functions"][index]["body"]);
}

fn change_guard_literal(node: &mut Value) {
    if let Some(snippet) = node["snippet"].as_str() {
        node["snippet"] = snippet.replacen("iy == 0", "iy == 1", 1).into();
    }
    if node["dispatch"]["op"] == "==" && node["children"][0]["snippet"] == "iy" {
        node["children"][1]["lit"] = "1".into();
        node["children"][1]["snippet"] = "1".into();
    }
    for child in node["children"].as_array_mut().unwrap() {
        change_guard_literal(child);
    }
}

fn swap_binding_inventory(facts: &mut Value) {
    let bindings = facts["bindings"].as_array_mut().unwrap();
    let first = bindings
        .iter()
        .position(|binding| binding["name"] == "iy")
        .unwrap();
    let second = bindings
        .iter()
        .position(|binding| binding["name"] == "other")
        .unwrap();
    let left = bindings[first]["hir"].clone();
    let right = bindings[second]["hir"].clone();
    bindings[first]["hir"] = right.clone();
    bindings[second]["hir"] = left.clone();
    swap_local_resolutions(&mut facts["body"], &left, &right);
}

fn swap_local_resolutions(node: &mut Value, left: &Value, right: &Value) {
    if node["res"]["kind"] == "local" {
        if &node["res"]["target"] == left {
            node["res"]["target"] = right.clone();
        } else if &node["res"]["target"] == right {
            node["res"]["target"] = left.clone();
        }
    }
    for child in node["children"].as_array_mut().unwrap() {
        swap_local_resolutions(child, left, right);
    }
}

fn path_resolution(node: &Value, name: &str) -> Option<Value> {
    if node["kind"] == "expr_path" && node["snippet"] == name {
        return Some(node["res"].clone());
    }
    node["children"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|child| path_resolution(child, name))
}

fn replace_guard_formal(node: &mut Value, replacement: &Value) -> bool {
    if node["dispatch"]["op"] == "==" {
        node["children"][0]["res"] = replacement.clone();
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| replace_guard_formal(child, replacement))
}

fn corrupt_source(node: &mut Value, mutation: &str) -> bool {
    if node["kind"] == "expr_if" && mutation == "source_body" {
        node["children"].as_array_mut().unwrap().swap(1, 2);
        return true;
    }
    if node["dispatch"]["op"] == "==" && mutation.starts_with("condition_") {
        corrupt_condition(node, mutation);
        return true;
    }
    if mutation == "saved_local" && node["snippet"] == "old - w" {
        node["children"][0]["res"] = node["children"][1]["res"].clone();
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| corrupt_source(child, mutation))
}

fn corrupt_condition(node: &mut Value, mutation: &str) {
    match mutation {
        "condition_op" => node["dispatch"]["op"] = "!=".into(),
        "condition_literal" => node["children"][1]["lit"] = "1".into(),
        "condition_type" => node["children"][0]["ty"]["normalized"] = "u32".into(),
        "condition_order" => node["children"].as_array_mut().unwrap().swap(0, 1),
        _ => unreachable!(),
    }
}

fn corrupt_graph(graph: &mut Value, mutation: &str) {
    if mutation == "return" {
        let edges = graph["cfg"]["edges"].as_array_mut().unwrap();
        let edge = edges
            .iter_mut()
            .find(|edge| edge["kind"] == "return")
            .unwrap();
        edge["kind"] = "normal".into();
        return;
    }
    let site = &mut graph["pdg"]["extensions"]["compat.rust.hir-r0:value-sites"][0];
    match mutation {
        "swap_alternatives" => {
            let first = site["alternatives"][0]["value_pdg_node_id"].clone();
            site["alternatives"][0]["value_pdg_node_id"] =
                site["alternatives"][1]["value_pdg_node_id"].clone();
            site["alternatives"][1]["value_pdg_node_id"] = first;
        }
        "drop_alternative" => {
            site["alternatives"].as_array_mut().unwrap().pop();
        }
        "guard" => {
            site["alternatives"][0]["annotations"]["compat.rust.hir-r0:guards"] =
                serde_json::json!([[]])
        }
        "merge" => site["pdg_node_id"] = site["controller_pdg_node_id"].clone(),
        _ => unreachable!(),
    }
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
    pair.pdg.profile.version = "3.65.0".into();
    pair.cfg.profile.version = "3.65.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}
