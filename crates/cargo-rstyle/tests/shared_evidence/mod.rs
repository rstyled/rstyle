//! Strict shared-bundle mutation fixtures; never reconstruct nested proofs.
mod source_reuse;
use rstyle_pdg::{HirFnFacts, unit_store::UnitStore};
use serde_json::Value;

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
    assert_eq!(
        function["unit_evidence"]["namespace"],
        serde_json::to_value(store.namespace()).unwrap()
    );
    (store, index(function))
}

pub fn pair<'a>(store: &'a UnitStore, function: &Value) -> &'a rstyle_pdg::producer::Pair {
    assert_eq!(
        function["unit_evidence"]["namespace"],
        serde_json::to_value(store.namespace()).unwrap()
    );
    store.pair(index(function)).unwrap()
}

pub fn reject_edit(function: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = wire(function);
    UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
    edit(&mut bundle, index(function));
    let bytes = serde_json::to_vec(&bundle).unwrap();

    assert!(UnitStore::from_bundle(&bytes).is_err());
    assert!(rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err());
}

pub fn reject_facts(function: &Value, changed: &HirFnFacts) {
    reject_edit(function, |bundle, index| {
        let source = replace_source(bundle, index, &changed.source.bytes, &changed.source.digest);
        let mut frontend = serde_json::to_value(changed).unwrap();
        frontend["source"] = source;
        let options = frontend["build"]["frontend_options"]
            .as_object_mut()
            .unwrap();
        for key in [
            rstyle_pdg::nominal_boundary::census::KEY,
            rstyle_pdg::declaration_origins::KEY,
        ] {
            if let Some(value) = options.get_mut(key) {
                *value = serde_json::json!({"kind": "inline", "value": value.take()});
            }
        }
        bundle["functions"][index] = frontend;
        prune_source_table(bundle);
    });
}

pub fn replace_source(bundle: &mut Value, index: usize, bytes: &str, digest: &str) -> Value {
    let previous = bundle["functions"][index]["source"]["snapshot"]
        .as_u64()
        .unwrap();
    let reference = &bundle["functions"][index]["source"];
    let range = &reference["range"];
    let original = bundle["sources"][previous as usize].as_str().unwrap();
    if original
        .get(range["start"].as_u64().unwrap() as usize..range["end"].as_u64().unwrap() as usize)
        == Some(bytes)
    {
        let mut reference = reference.clone();
        reference["digest"] = digest.into();
        return reference;
    }
    let sources = bundle["sources"].as_array_mut().unwrap();
    sources.push(bytes.into());
    let snapshot = sources.len() - 1;
    serde_json::json!({"snapshot": snapshot, "range": {"start": 0, "end": bytes.len()}, "digest": digest})
}

fn source_references(bundle: &mut Value, mut visit: impl FnMut(&mut Value)) {
    for function in bundle["functions"].as_array_mut().unwrap() {
        visit(&mut function["source"]["snapshot"]);
        let options = function["build"]["frontend_options"]
            .as_object_mut()
            .unwrap();
        for (key, kind, records) in [
            (rstyle_pdg::nominal_boundary::census::KEY, "census", "files"),
            (
                rstyle_pdg::declaration_origins::KEY,
                "origins",
                "declarations",
            ),
        ] {
            if let Some(option) = options.get_mut(key)
                && option["kind"] == kind
            {
                for record in option["value"][records].as_array_mut().unwrap() {
                    visit(&mut record["snapshot"]);
                }
            }
        }
    }
}

fn prune_source_table(bundle: &mut Value) {
    let mut used = std::collections::BTreeSet::new();
    source_references(bundle, |id| {
        used.insert(id.as_u64().unwrap() as usize);
    });
    let mapping: std::collections::BTreeMap<_, _> = used
        .into_iter()
        .enumerate()
        .map(|(new, old)| (old, new))
        .collect();
    let previous = std::mem::take(bundle["sources"].as_array_mut().unwrap());
    bundle["sources"] = previous
        .into_iter()
        .enumerate()
        .filter(|(index, _)| mapping.contains_key(index))
        .map(|(_, value)| value)
        .collect();
    source_references(bundle, |id| {
        *id = mapping[&(id.as_u64().unwrap() as usize)].into();
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
        } else if corruption == "source" {
            let digest = bundle["functions"][callee]["source"]["digest"]
                .as_str()
                .unwrap()
                .to_owned();
            let source = replace_source(bundle, callee, "fn forged() {}", &digest);
            bundle["functions"][callee]["source"] = source;
            prune_source_table(bundle);
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

/// A checked normal-flow helper retains possible panic, never a totality claim.
pub fn check_partial_helper(function: &Value) {
    assert_eq!(
        function["outcome"], "admitted",
        "{}",
        function["facts"]["name"]
    );
    assert_eq!(function["extraction_evidence"]["cfg"]["status"], "partial");
    let bundle = wire(function);
    let (_, helper) = callee(&bundle, index(function));
    let summary = &bundle["helpers"][helper]["summary"];
    assert_eq!(summary["identity"], "rstyle.helper.checked-normal-body/1");
    assert_eq!(summary["may_panic"], true);
    assert_eq!(summary["always_returns"], false);
    assert_eq!(summary["mutates_arguments"], false);
    for field in ["may_panic", "always_returns"] {
        reject_edit(function, |bundle, caller| {
            let (_, helper) = callee(bundle, caller);
            bundle["helpers"][helper]["summary"][field] = (field == "always_returns").into();
        });
    }
}

/// Forge the document profile at the current wire boundary, so inherited record
/// metadata is coherent and strict verification must reject the actual claim.
pub fn with_profile_version(
    pair: &rstyle_pdg::producer::Pair,
    version: &str,
) -> rstyle_pdg::producer::Pair {
    let mut wire = serde_json::to_value(pair).unwrap();
    wire["pdg"]["profile"]["version"] = version.into();
    wire["cfg"]["profile"]["version"] = version.into();

    serde_json::from_value(wire).unwrap()
}
