//! Expectations come from unchanged libm k_cos: six coefficient reads, four
//! ordered subtractions, and two distinct lexical bindings named w. The original
//! library oracle is maintained separately; this test exercises extraction proof.
use super::float_kernels::{function, reject_resealed};
use super::*;

const ROOT: &str = r#"
#[macro_use] mod macros;
cfg_if! {
    if #[cfg(any())] { mod never; }
    else if #[cfg(all())] { mod k_cos; }
    else { mod fallback; }
}
mod k_sinf;
mod k_cosf;
pub fn tail(x: f64, y: f64) -> f64 { k_cos::k_cos(x, y) }
pub fn same(x: f64, y: f64) -> f64 { k_cos::k_cos(x, y) }
pub fn subtract(a: f64, b: f64) -> f64 { a - b }
pub fn saved(a: f64, b: f64) -> f64 { let w = a; let old = w; let w = b; old - w }
pub fn integer(a: i64, b: i64) -> i64 { a - b }
pub fn divide(a: f64, b: f64) -> f64 { a / b }
pub fn compare(a: f64, b: f64) -> bool { a < b }
pub fn unsupported_call(a: f64) -> f64 { core::hint::black_box(a) }
pub fn subtract32(a: f32, b: f32) -> f32 { a - b }
"#;

fn capture() -> (TempDir, Value) {
    capture_context(None, false)
}

fn capture_context(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let root = format!("{}{ROOT}", if no_std { "#![no_std]\n" } else { "" });
    let directory = super::cfg_if_modules::workspace(&root, super::cfg_if_modules::CFG_IF);
    std::fs::write(
        directory.path().join("src/k_cos.rs"),
        include_str!("float_kernels/k_cos.rs"),
    )
    .unwrap();
    let mut arguments = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        arguments.extend(["--target", target]);
    }

    let (output, report) = clones(directory.path(), &arguments);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn original_cos_tail_composes_subtraction_constants_shadowing_and_strict_matching() {
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
            let (_directory, report) = capture_context(Some(target), no_std);
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let positive = [
        "k_cos::k_cos",
        "k_sinf::k_sinf",
        "k_cosf::k_cosf",
        "tail",
        "same",
        "subtract",
        "saved",
        "integer",
    ];
    for entry in report["functions"].as_array().unwrap() {
        let name = entry["facts"]["name"].as_str().unwrap();
        assert_eq!(
            entry["outcome"],
            if positive.contains(&name) {
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

    check_kernel(function(report, "k_cos::k_cos"));
    let left = &function(report, "tail")["key"];
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

fn check_kernel(entry: &Value) {
    let body = &entry["facts"]["body"];
    let mut constants = Vec::new();
    collect_constants(body, &mut constants);
    constants.sort();
    constants.dedup();
    assert_eq!(constants, ["C1", "C2", "C3", "C4", "C5", "C6"]);
    let bindings = entry["facts"]["bindings"].as_array().unwrap();
    let shadowed: Vec<_> = bindings
        .iter()
        .filter(|binding| binding["name"] == "w")
        .collect();
    assert_eq!(shadowed.len(), 2);
    assert_ne!(shadowed[0]["symbol"], shadowed[1]["symbol"]);

    let (unit, index) = data_unit(entry);
    let pair = unit.pair(index).unwrap();
    let subtractions: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.attributes
                .operator
                .as_ref()
                .and_then(|value| value.known_value())
                == Some("-")
        })
        .collect();
    assert_eq!(subtractions.len(), 4);
    for node in subtractions {
        assert_eq!(
            node.attributes.construct.as_deref(),
            Some("data.float-binary")
        );
        assert!(node.attributes.outcomes.is_empty());
        assert!(node.attributes.reads.is_empty());
        assert!(node.attributes.writes.is_empty());
        assert_eq!(
            node.annotations["rstyle.data:float-scalar"]["operation"],
            "-"
        );
        assert_eq!(
            node.annotations["rstyle.data:float-scalar"]["contraction"],
            "forbidden"
        );
    }
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
fn cos_tail_resealed_documents_reject_reordered_subtraction_and_changed_semantics() {
    let (_directory, report) = capture();
    for mutation in ["operator", "rounding", "contraction", "effects", "operands"] {
        let entry = function(&report, "subtract");
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        corrupt_graph(&mut bundle["graphs"][root]["pdg"], mutation);
        reject_resealed(bundle, mutation);
    }
    for mutation in ["source_operator", "constant", "saved_binding"] {
        let entry = function(
            &report,
            if mutation == "saved_binding" {
                "saved"
            } else {
                "k_cos::k_cos"
            },
        );
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        assert!(corrupt_source(
            &mut bundle["functions"][index]["body"],
            mutation
        ));
        reject_resealed(bundle, mutation);
    }
    reject_historical(function(&report, "k_cos::k_cos"));
    reject_historical(function(&report, "subtract"));
}

fn corrupt_graph(graph: &mut Value, mutation: &str) {
    let node = graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["attributes"]["operator"]["value"] == "-")
        .unwrap();
    let target = node["id"].clone();
    match mutation {
        "operator" => node["attributes"]["operator"]["value"] = "+".into(),
        "rounding" => {
            node["annotations"]["rstyle.data:float-scalar"]["rounding"] = "toward-zero".into()
        }
        "contraction" => {
            node["annotations"]["rstyle.data:float-scalar"]["contraction"] = "allowed".into()
        }
        "effects" => node["attributes"]["outcomes"] = serde_json::json!(["panic"]),
        "operands" => swap_operands(graph, &target),
        _ => unreachable!(),
    }
}

fn swap_operands(graph: &mut Value, target: &Value) {
    let edges = graph["edges"].as_array_mut().unwrap();
    let positions: Vec<_> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| {
            edge["subkind"] == "value_operand"
                && &edge["target"] == target
                && edge["operand_position"].is_u64()
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(positions.len(), 2);
    let first = edges[positions[0]]["source"].clone();
    edges[positions[0]]["source"] = edges[positions[1]]["source"].clone();
    edges[positions[1]]["source"] = first;
}

fn corrupt_source(node: &mut Value, mutation: &str) -> bool {
    if mutation == "source_operator" && node["dispatch"]["op"] == "-" {
        node["dispatch"]["op"] = "+".into();
        return true;
    }
    if mutation == "constant" && node["res"]["kind"] == "resolved_scalar_constant" {
        let mut record: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        record["value"] = "0x0000000000000000".into();
        node["res"]["description"] = serde_json::to_string(&record).unwrap().into();
        return true;
    }
    if mutation == "saved_binding" && node["snippet"] == "old - w" {
        let first = node["children"][0]["res"].clone();
        node["children"][0]["res"] = node["children"][1]["res"].clone();
        node["children"][1]["res"] = first;
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
    pair.pdg.profile.version = "3.64.0".into();
    pair.cfg.profile.version = "3.64.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn cos_tail_local_drop_effect_cannot_be_hidden_by_float_arithmetic() {
    let directory = clone_workspace();
    let source = "pub fn hidden(x: f64, y: f64) -> f64 { struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } let _guard = Hidden; x - y }";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    assert_eq!(function(&report, "hidden")["outcome"], "excluded");
}
