//! Source-first scope expectations: two unchanged original kernels depend only on
//! their own primitive values/constants, despite unrelated mixed ancestor scopes.
use super::*;

const SIN: &str = include_str!("float_kernels/k_sinf.rs");
const COS: &str = include_str!("float_kernels/k_cosf.rs");
const ROOT: &str = r#"
use core::{f32, f64};
mod k_sinf;
mod k_cosf;
mod unrelated;
pub use unrelated::*;
mod types { pub type Marker = u64; }
"#;
const UNRELATED: &str = r#"
pub struct Noise;
impl Drop for Noise { fn drop(&mut self) { core::hint::black_box(()); } }
pub trait Trait { fn noise(&self) -> &str; }
impl Trait for Noise { fn noise(&self) -> &str { "unrelated" } }
macro_rules! hidden { () => { pub struct Generated; }; }
hidden!();
"#;

fn workspace(sine: &str) -> TempDir {
    let directory = clone_workspace();
    let source = directory.path().join("src");
    for (path, bytes) in [
        ("lib.rs", ROOT),
        ("k_sinf.rs", sine),
        ("k_cosf.rs", COS),
        ("unrelated.rs", UNRELATED),
    ] {
        std::fs::write(source.join(path), bytes).unwrap();
    }
    directory
}

#[test]
fn float_leaf_dependencies_survive_unrelated_parent_imports_macros_traits_and_drop() {
    let directory = workspace(SIN);
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_selected(&report);
    for name in ["k_sinf::k_sinf", "k_cosf::k_cosf"] {
        reject_historical_policy(function(&report, name));
    }
}

fn check_selected(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let admitted: Vec<_> = functions
        .iter()
        .filter(|function| function["outcome"] == "admitted")
        .collect();
    assert_eq!(admitted.len(), 2);

    for name in ["k_sinf::k_sinf", "k_cosf::k_cosf"] {
        let function = function(report, name);
        assert_eq!(
            function["outcome"], "admitted",
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
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 1);
}

#[test]
fn float_leaf_ownership_cannot_ignore_local_alias_glob_or_unknown_macro_output() {
    for prefix in [
        "type f64 = core::primitive::f64;",
        "use crate::types::*;",
        "macro_rules! inject { () => { const NOISE: u8 = 0; }; } inject!();",
    ] {
        let source = format!("{prefix}\n{SIN}");
        reject_sine(&source);
    }
}

#[test]
fn float_leaf_source_proof_cannot_certify_only_a_supported_prefix_or_suffix() {
    let prefix = SIN.replace("    let z = x * x;", "    helper(x);\n    let z = x * x;");
    let suffix = SIN.replace(
        "    ((x + s * (S1 + z * S2)) + s * w * r) as f32",
        "    helper(((x + s * (S1 + z * S2)) + s * w * r) as f32)",
    );
    let drop = SIN.replace(
        "    let z = x * x;",
        "    let _noise = crate::Noise;\n    let z = x * x;",
    );
    for (source, helper) in [
        (prefix, "fn helper(x: f64) -> f64 { x }"),
        (suffix, "fn helper(x: f32) -> f32 { x }"),
        (drop, ""),
    ] {
        assert_ne!(source, SIN);
        reject_sine(&format!("{source}\n{helper}"));
    }
}

fn reject_sine(source: &str) {
    let directory = workspace(source);
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);

    let sine = function(&report, "k_sinf::k_sinf");
    assert_eq!(sine["outcome"], "excluded", "{}", sine["reasons"]);
    let cosine = function(&report, "k_cosf::k_cosf");
    assert_eq!(cosine["outcome"], "admitted", "{}", cosine["reasons"]);

    super::word_classifiers::verify_admissions(&report);
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

fn reject_historical_policy(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();

    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.63.0".into();
    pair.cfg.profile.version = "3.63.0".into();

    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}
