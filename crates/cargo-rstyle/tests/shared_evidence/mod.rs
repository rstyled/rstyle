//! Strict shared-bundle mutation fixtures; never reconstruct nested proofs.
use rstyle_pdg::{HirFnFacts, unit_store::UnitStore};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn wire(function: &Value) -> Value {
    let path = function["unit_evidence"]["bundle"].as_str().unwrap();
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

pub fn index(function: &Value) -> usize {
    usize::try_from(function["unit_evidence"]["function"].as_u64().unwrap()).unwrap()
}

pub fn load(function: &Value) -> (UnitStore, usize) {
    let bytes = std::fs::read(function["unit_evidence"]["bundle"].as_str().unwrap()).unwrap();
    let store = UnitStore::from_bundle(&bytes).unwrap();
    assert_eq!(function["unit_evidence"]["identity"], store.identity());
    (store, index(function))
}

pub fn reject_edit(function: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = wire(function);
    UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
    edit(&mut bundle, index(function));
    refresh_identity(&mut bundle);
    let bytes = serde_json::to_vec(&bundle).unwrap();

    assert!(UnitStore::from_bundle(&bytes).is_err());
    assert!(rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err());
}

fn refresh_identity(bundle: &mut Value) {
    let facts: Vec<HirFnFacts> = serde_json::from_value(bundle["functions"].clone()).unwrap();
    let identity = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&facts).unwrap())
    );
    bundle["identity"] = identity.clone().into();
    for graph in bundle["graphs"].as_array_mut().unwrap() {
        for kind in ["pdg", "cfg"] {
            graph[kind]["extensions"][rstyle_pdg::extraction_profile::EVIDENCE_KEY]["frontend_ref"]
                ["unit"] = identity.clone().into();
        }
    }
}

pub fn reject_facts(function: &Value, changed: &HirFnFacts) {
    reject_edit(function, |bundle, index| {
        bundle["functions"][index] = serde_json::to_value(changed).unwrap();
    });
}

fn callee(bundle: &Value, caller: usize) -> (usize, usize) {
    let function =
        usize::try_from(bundle["links"][caller][0]["function"].as_u64().unwrap()).unwrap();
    let helper = usize::try_from(bundle["certified"][function].as_u64().unwrap()).unwrap();
    (function, helper)
}

pub fn reject_helper(function: &Value, corruption: &str) {
    if corruption == "manifest" {
        reject_seal(function);
        return;
    }
    reject_edit(function, |bundle, caller| {
        let (callee, helper) = callee(bundle, caller);
        if corruption.starts_with("output_") || corruption == "effects" {
            corrupt_summary(&mut bundle["helpers"][helper]["summary"], corruption);
        } else {
            corrupt_frontend(&mut bundle["functions"][callee], corruption);
        }
    });
}

fn reject_seal(function: &Value) {
    let path = std::path::Path::new(function["unit_evidence"]["bundle"].as_str().unwrap());
    let bytes = std::fs::read(path).unwrap();
    let manifest: rstyle_pdg::unit_store::BundleManifest = serde_json::from_slice(
        &std::fs::read(path.with_file_name("unit.evidence.manifest.json")).unwrap(),
    )
    .unwrap();
    manifest.verify(&bytes).unwrap();
    let mut changed = bytes;
    changed.push(b' ');
    assert!(manifest.verify(&changed).is_err());
}

fn corrupt_frontend(frontend: &mut Value, corruption: &str) {
    match corruption {
        "source" => frontend["source"]["bytes"] = "fn forged() {}".into(),
        "signature" => frontend["signature"]["return_ty"]["normalized"] = "i32".into(),
        "unit_invocation" => {
            frontend["build"]["frontend_options"]["invocation"] = serde_json::json!(["forged"])
        }
        "unit_target" => frontend["build"]["target_triple"]["value"] = "another-target".into(),
        _ => panic!("unknown frontend corruption {corruption}"),
    }
}

fn corrupt_summary(summary: &mut Value, corruption: &str) {
    match corruption {
        "effects" => summary["always_returns"] = false.into(),
        "output_missing" => {
            summary.as_object_mut().unwrap().remove("reference_outputs");
        }
        "output_parameter" => summary["reference_outputs"][0]["parameter"] = 0.into(),
        "output_empty" => summary["reference_outputs"] = serde_json::json!([]),
        "output_path" => summary["reference_outputs"][0]["path"] = serde_json::json!([]),
        "output_identity" => summary["identity"] = "rstyle.helper.source-body/1".into(),
        _ => panic!("unknown summary corruption {corruption}"),
    }
}

pub fn reject_read_summary(function: &Value, change: &str) {
    reject_edit(function, |bundle, caller| {
        let (_, helper) = callee(bundle, caller);
        tamper_read_summary(&mut bundle["helpers"][helper]["summary"], change);
    });
}

fn tamper_read_summary(summary: &mut Value, change: &str) {
    match change {
        "missing" => {
            summary.as_object_mut().unwrap().remove("memory_reads");
        }
        "empty" => summary["memory_reads"] = serde_json::json!([]),
        "extra" => {
            let read = summary["memory_reads"][0].clone();
            summary["memory_reads"].as_array_mut().unwrap().push(read);
        }
        "operation" => summary["memory_reads"][0]["operation"] = "forged".into(),
        "access" => summary["memory_reads"][0]["access"] = 1.into(),
        "parameter" => summary["memory_reads"][0]["parameter"] = 1.into(),
        "path" => summary["memory_reads"][0]["path"] = serde_json::json!([]),
        "type" => summary["memory_reads"][0]["value"]["value"] = "u16".into(),
        "effect" => summary["memory_effects"] = false.into(),
        "identity" => summary["identity"] = "rstyle.helper.source-body/1".into(),
        _ => panic!("unknown read corruption {change}"),
    }
}
