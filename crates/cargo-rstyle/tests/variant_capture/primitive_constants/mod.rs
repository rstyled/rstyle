//! Exact primitive authority composed with the unchanged original arithmetic.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::unit_store::{BundleManifest, UnitStore, transport_checksum};

const SELECTION: &str =
    include_str!("../../../../../eval/coverage-development/primitive-constant-selection.json");
const RESOLUTION: &str = "resolved_primitive_constant";

fn original() -> String {
    let selection: Value = serde_json::from_str(SELECTION).unwrap();
    selection["functions"][0]["source"].as_str().unwrap().into()
}

fn source() -> String {
    let original = original();
    let alpha = original
        .replace("mul_shift_32", "alpha_mul_shift")
        .replace("factor_lo", "low_word")
        .replace("factor_hi", "high_word")
        .replace("bits0", "low_product")
        .replace("bits1", "high_product")
        .replace("shifted_sum", "answer");
    format!(
        "{original}\n{alpha}\n\
         fn bridge(m:u32,f:u64,s:i32)->u32 {{ mul_shift_32(m,f,s) }}\n\
         pub fn wrapper(m:u32,f:u64,s:i32)->u32 {{ bridge(m,f,s) }}\n\
         fn alpha_bridge(m:u32,f:u64,s:i32)->u32 {{ alpha_mul_shift(m,f,s) }}\n\
         pub fn alpha_wrapper(m:u32,f:u64,s:i32)->u32 {{ alpha_bridge(m,f,s) }}\n"
    )
}

fn capture(source: &str) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn original_mul_shift_constant_composes_with_alpha_peers_and_wrapper_chain() {
    let (_directory, report) = capture(&source());
    for name in [
        "mul_shift_32",
        "alpha_mul_shift",
        "bridge",
        "wrapper",
        "alpha_bridge",
        "alpha_wrapper",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    super::word_classifiers::verify_admissions(&report);
    for (first, second) in [
        ("mul_shift_32", "alpha_mul_shift"),
        ("bridge", "alpha_bridge"),
        ("wrapper", "alpha_wrapper"),
    ] {
        accepted_pair(&report, first, second);
    }
    let entry = function(&report, "mul_shift_32");
    let node = primitive(&entry["facts"]["body"]).expect("captured primitive constant");
    let record: Value = serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    assert_eq!(record["primitive"], "u32");
    assert_eq!(record["member"], "MAX");
    assert_eq!(record["value"], "4294967295");
    reject_attacks(entry);
    reject_resealed(function(&report, "wrapper"), downgrade);
}

fn accepted_pair(report: &Value, first: &str, second: &str) {
    let first = &function(report, first)["key"];
    let second = &function(report, second)["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == first && &pair["second"] == second)
                || (&pair["first"] == second && &pair["second"] == first)
        })
        .expect("complete alpha peer comparison");
    assert_eq!(pair["decision"], "Accepted", "{pair}");
}

fn primitive(node: &Value) -> Option<&Value> {
    if node["res"]["kind"] == RESOLUTION {
        return Some(node);
    }
    node["children"].as_array()?.iter().find_map(primitive)
}

fn constant_mut(node: &mut Value) -> Option<&mut Value> {
    if matches!(
        node["res"]["kind"].as_str(),
        Some("resolved_primitive_constant" | "resolved_scalar_constant")
    ) {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(constant_mut)
}

fn reject_attacks(entry: &Value) {
    for attack in [
        "value",
        "type",
        "member",
        "crate",
        "path",
        "crate_hash",
        "hash",
        "observed",
        "normalized",
    ] {
        reject_resealed(entry, |bundle, index| {
            let node = constant_mut(&mut bundle["functions"][index]["body"]).unwrap();
            corrupt_constant(node, attack);
        });
    }
    for field in [
        "target_triple",
        "pointer_width",
        "rustc_commit",
        "rustc_version",
    ] {
        reject_resealed(entry, |bundle, index| {
            let value = &mut bundle["functions"][index]["build"][field]["value"];
            *value = if field == "pointer_width" {
                32.into()
            } else {
                "untrusted".into()
            };
        });
    }
    reject_resealed(entry, downgrade);
}

fn corrupt_constant(node: &mut Value, attack: &str) {
    if matches!(attack, "observed" | "normalized") {
        node["ty"][attack] = "u64".into();
        return;
    }
    let mut record: Value =
        serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
    match attack {
        "value" => record["value"] = "0".into(),
        "type" => record["primitive"] = "u64".into(),
        "member" => record["member"] = "MIN".into(),
        "crate" => record["definition"]["crate"] = "std".into(),
        "path" => record["definition"]["path"] = "core::u64::MAX".into(),
        "crate_hash" => record["definition"]["crate_hash"] = "forged".into(),
        "hash" => record["definition"]["definition_hash"] = "forged".into(),
        _ => unreachable!(),
    }
    node["res"]["description"] = serde_json::to_string(&record).unwrap().into();
}

fn downgrade(bundle: &mut Value, index: usize) {
    let graph = bundle["roots"][index].as_u64().unwrap() as usize;
    bundle["graphs"][graph]["pdg"]["profile"]["version"] = "3.76.0".into();
    bundle["graphs"][graph]["cfg"]["profile"]["version"] = "3.76.0".into();
}

fn reject_resealed(entry: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = shared_evidence::wire(entry);
    UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
    edit(&mut bundle, shared_evidence::index(entry));
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let manifest = BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    manifest.verify(&bytes).unwrap();
    assert!(UnitStore::from_bundle(&bytes).is_err());
    assert!(rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err());
}

#[test]
fn unsupported_primitive_spellings_and_neighbors_remain_inventoried() {
    let source = r#"
pub fn supported() -> u32 { u32::MAX }
pub fn wider() -> u64 { u64::MAX }
pub fn minimum() -> u32 { u32::MIN }
type Word = u32;
pub fn alias() -> u32 { Word::MAX }
pub unsafe fn unsafe_neighbor(p: *const u32) -> u32 { unsafe { *p } }
mod shadowed {
    #[allow(non_camel_case_types)]
    struct u32;
    impl u32 { const MAX: core::primitive::u32 = 7; }
    pub fn shadow() -> core::primitive::u32 { u32::MAX }
}
"#;
    let (_directory, report) = capture(source);
    assert_eq!(function(&report, "supported")["outcome"], "admitted");
    for name in ["wider", "minimum", "alias", "unsafe_neighbor"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    let shadow = function(&report, "shadowed::shadow");
    assert!(primitive(&shadow["facts"]["body"]).is_none());
    reject_shadow_authority(function(&report, "supported"), shadow);
}

fn reject_shadow_authority(supported: &Value, shadow: &Value) {
    let supported_facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(supported["facts"].clone()).unwrap();
    let genuine: rstyle_pdg::HirNode =
        serde_json::from_value(primitive(&supported["facts"]["body"]).unwrap().clone()).unwrap();
    rstyle_pdg::primitive_constants::verify(&genuine, &supported_facts).unwrap();

    let shadow_facts: rstyle_pdg::HirFnFacts =
        serde_json::from_value(shadow["facts"].clone()).unwrap();
    let mut body = shadow["facts"]["body"].clone();
    let mut forged: rstyle_pdg::HirNode =
        serde_json::from_value(constant_mut(&mut body).unwrap().clone()).unwrap();
    assert_eq!(
        forged.ty.as_ref().unwrap().normalized.as_deref(),
        Some("u32")
    );
    forged.res = genuine.res;
    // Qualified declaration spelling may exclude the honest local constant.
    // Invoke the leaf directly so rejection must reach lexical owner evidence,
    // after passing the genuine compiler, constant, type and source-path checks.
    let error = rstyle_pdg::primitive_constants::verify(&forged, &shadow_facts).unwrap_err();
    assert!(error.contains("shadowed"), "{error}");
}
