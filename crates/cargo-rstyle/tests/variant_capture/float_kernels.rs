use super::*;

// Whole original libm 0.2.16 files, frozen before measuring extraction. This is
// a diagnostic composition fixture; natural acceptance uses the complete crate.
const SOURCE: &str = concat!(
    include_str!("float_kernels/k_sinf.rs"),
    "\n",
    include_str!("float_kernels/k_cosf.rs"),
    r#"
pub fn sin_wrapper(x: f64) -> f32 { k_sinf(x) }
pub fn cos_wrapper(x: f64) -> f32 { k_cosf(x) }
pub fn sin_same(x: f64) -> f32 { k_sinf(x) }
pub fn sum(a: f64, b: f64) -> f64 { a + b }
pub fn multiply(a: f64, b: f64) -> f64 { a * b }
pub fn narrow(x: f64) -> f32 { x as f32 }
pub fn literal64() -> f64 { 1.25_f64 }
pub fn literal32() -> f32 { 0.5_f32 }
pub fn negative_zero() -> f64 { -0.0_f64 }
pub fn saved(mut a: f64, b: f64) -> f64 { let old = a; a = b; old * a }
pub fn subtract(a: f64, b: f64) -> f64 { a - b }
pub fn divide(a: f64, b: f64) -> f64 { a / b }
pub fn remainder(a: f64, b: f64) -> f64 { a % b }
pub fn comparison(a: f64, b: f64) -> bool { a < b }
pub fn negate(x: f64) -> f64 { -x }
pub fn to_int(x: f64) -> i64 { x as i64 }
pub fn from_int(x: i64) -> f64 { x as f64 }
pub fn widen(x: f32) -> f64 { x as f64 }
pub fn reference(x: &f64) -> f64 { *x }
pub fn optional(x: f64) -> Option<f64> { Some(x) }
pub fn array(x: f64) -> [f64; 1] { [x] }
pub fn tuple(x: f64) -> (f64, f64) { (x, x) }
const COMPUTED: f64 = 0.5 + 0.25;
pub fn computed_constant() -> f64 { COMPUTED }
mod foreign { pub const S1: f64 = -0.166666666416265235595; }
use foreign::S1 as IMPORTED;
pub fn qualified_constant() -> f64 { foreign::S1 }
pub fn imported_constant() -> f64 { IMPORTED }
pub fn unused(_x: f64) -> u8 { 7 }
"#,
);

const POSITIVES: &[&str] = &[
    "k_sinf",
    "k_cosf",
    "sin_wrapper",
    "cos_wrapper",
    "sin_same",
    "sum",
    "multiply",
    "subtract",
    "narrow",
    "literal64",
    "literal32",
    "negative_zero",
    "saved",
    "negate",
    "unused",
];

#[test]
fn float_kernels_compose_constants_arithmetic_and_narrowing_in_registered_contexts() {
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
            let directory = clone_workspace();
            let source = format!("{}{SOURCE}", if no_std { "#![no_std]\n" } else { "" });
            std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
            let (output, report) = clones(
                directory.path(),
                &["--extraction-profile", "hir-data-v3", "--target", target],
            );
            assert!(output.status.success(), "{}", report["capture_failures"]);
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 28);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if POSITIVES.contains(&name) {
                "admitted"
            } else {
                "excluded"
            },
            "{name}: {}",
            function["reasons"]
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
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 105);
    let left = &function(report, "sin_wrapper")["key"];
    let right = &function(report, "sin_same")["key"];
    let identical = pairs
        .iter()
        .find(|pair| {
            (&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left)
        })
        .unwrap();
    assert_eq!(identical["decision"], "Accepted");
}

#[test]
fn float_kernel_documents_reject_resealed_semantic_corruption_and_historical_policy() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_family(&report);
    for (name, mutation) in [
        ("literal64", "literal_bits"),
        ("literal64", "literal_width"),
        ("literal64", "literal_integer"),
        ("k_sinf", "constant_bits"),
        ("k_sinf", "constant_source"),
        ("k_sinf", "constant_same_value_source"),
        ("k_sinf", "constant_module"),
        ("narrow", "cast_width"),
        ("narrow", "cast_dispatch"),
    ] {
        let function = function(&report, name);
        let mut bundle = shared_evidence::wire(function);
        let index = shared_evidence::index(function);
        assert!(
            mutate_node(&mut bundle["functions"][index]["body"], mutation),
            "{mutation}"
        );
        reject_resealed(bundle, mutation);
    }
    reject_operand_order(function(&report, "saved"));
    reject_document_bits(function(&report, "literal64"));
    reject_old_policy(function(&report, "k_sinf"));
    reject_old_policy(function(&report, "unused"));
    let function = function(&report, "literal64");
    let mut bundle = shared_evidence::wire(function);
    bundle["functions"][shared_evidence::index(function)]["build"]["frontend_options"]["invocation"]
        .as_array_mut().unwrap().push("-Cllvm-args=-enable-unsafe-fp-math".into());
    reject_resealed(bundle, "unsafe floating backend override");
}

pub(super) fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

fn mutate_node(node: &mut Value, mutation: &str) -> bool {
    let kind = node["kind"].as_str().unwrap_or("");
    if kind == "expr_lit" && mutation.starts_with("literal_") {
        if mutation == "literal_bits" {
            let mut value: Value =
                serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
            value["bits"] = "0x4004000000000000".into();
            node["res"]["description"] = serde_json::to_string(&value).unwrap().into();
        } else if mutation == "literal_integer" {
            node["ty"]["normalized"] = "u64".into();
            node["ty"]["observed"] = "u64".into();
            node["res"] = Value::Null;
            node["lit"] = "1".into();
        } else {
            node["ty"]["normalized"] = "f32".into();
        }
        return true;
    }
    if kind == "expr_cast" && mutation.starts_with("cast_") {
        if mutation == "cast_width" {
            node["ty"]["normalized"] = "f64".into();
        } else {
            node["dispatch"]["kind"] = "overloaded".into();
        }
        return true;
    }
    if kind == "expr_path"
        && mutation.starts_with("constant_")
        && node["res"]["description"]
            .as_str()
            .is_some_and(|value| value.contains("S1"))
    {
        let mut record: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        mutate_constant(&mut record, mutation);
        node["res"]["description"] = serde_json::to_string(&record).unwrap().into();
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| mutate_node(child, mutation))
}

fn mutate_constant(record: &mut Value, mutation: &str) {
    let key = match mutation {
        "constant_bits" => "value",
        "constant_source" | "constant_same_value_source" => "declaration",
        "constant_module" => "definition",
        _ => unreachable!(),
    };
    let original = record[key]
        .as_str()
        .expect("float constant retains scalar resolution fields")
        .to_owned();
    record[key] = match mutation {
        "constant_bits" => {
            if original == "0x0000000000000000" {
                "0x3ff0000000000000".into()
            } else {
                "0x0000000000000000".into()
            }
        }
        "constant_same_value_source" => original
            .replace("-0.166666666416265235595", "-0.1666666664162652355950")
            .into(),
        "constant_source" => original.replace("-0.166666666416265235595", "-0.25").into(),
        "constant_module" => {
            format!("other_module::{}", original.rsplit("::").next().unwrap()).into()
        }
        _ => unreachable!(),
    };
    assert_ne!(record[key].as_str(), Some(original.as_str()));
}

pub(super) fn reject_resealed(bundle: Value, mutation: &str) {
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let seal = rstyle_pdg::unit_store::BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    let verified = seal.verify_owned(bytes).unwrap();
    assert!(
        rstyle_pdg::unit_store::UnitStore::from_bundle(verified.bytes()).is_err(),
        "{mutation}"
    );
}

fn reject_document_bits(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.literal.is_some())
        .unwrap();
    node.attributes.literal = Some(rstyle_pdg::StringFact::known("0x4004000000000000"));
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn reject_operand_order(function: &Value) {
    let mut bundle = shared_evidence::wire(function);
    let root = bundle["roots"][shared_evidence::index(function)]
        .as_u64()
        .unwrap() as usize;
    let target = bundle["graphs"][root]["pdg"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["attributes"]["operator"]["value"] == "*")
        .unwrap()["id"]
        .clone();
    let edges = bundle["graphs"][root]["pdg"]["edges"]
        .as_array_mut()
        .unwrap();
    let selected: Vec<_> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge["subkind"] == "value_operand" && edge["target"] == target)
        .filter(|(_, edge)| edge["operand_position"].is_u64())
        .map(|(index, _)| index)
        .collect();
    assert!(selected.len() >= 2);
    let first = selected[0];
    let second = *selected
        .iter()
        .skip(1)
        .find(|index| edges[**index]["target"] == edges[first]["target"])
        .unwrap();
    let left = edges[first]["source"].clone();
    edges[first]["source"] = edges[second]["source"].clone();
    edges[second]["source"] = left;
    reject_resealed(bundle, "saved operand order");
}

fn reject_old_policy(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.62.0".into();
    pair.cfg.profile.version = "3.62.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn float_kernel_runtime_preserves_exact_sign_toggles_and_directed_results() {
    let directory = clone_workspace();
    let oracle = r#"
#[test] fn independently_fixed_bits() {
    for bits in [0, 1 << 63, 1, 0x7ff0000000000000, 0xfff0000000000000,
                 0x7ff800000000002a, 0x7ff000000000002a, 0xfff0000020000000] {
        assert_eq!(negate(f64::from_bits(bits)).to_bits(), bits ^ (1 << 63));
    }
    for bits in [0, 1 << 63] {
        assert_eq!(k_sinf(f64::from_bits(bits)).to_bits(), 0);
        assert_eq!(k_cosf(f64::from_bits(bits)).to_bits(), 0x3f800000);
    }
    assert_eq!(k_sinf(f64::from_bits(1)).to_bits(), 0);
    assert_eq!(k_sinf(f64::from_bits((1 << 63) | 1)).to_bits(), 1 << 31);
    for (input, output) in [(0x7ff0000000000000, 0x7f800000),
                            (0xfff0000000000000, 0xff800000)] {
        assert_eq!(k_sinf(f64::from_bits(input)).to_bits(), output);
        assert!([0x7fc00000, 0xffc00000].contains(&k_cosf(f64::from_bits(input)).to_bits()));
    }
    assert_eq!(negative_zero().to_bits(), 1 << 63);
    assert_eq!(narrow(f64::from_bits(0x7fefffffffffffff)).to_bits(), 0x7f800000);
}
"#;
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, format!("{SOURCE}\n{oracle}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021", "-O"])
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
