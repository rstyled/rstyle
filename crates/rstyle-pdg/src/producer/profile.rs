//! Local policy transport, preserving the pinned wire schema and base vocabulary.

use super::Pair;
use crate::{HirFnFacts, ProfileId, extraction_profile::*};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

pub fn apply(pair: Pair, facts: &HirFnFacts) -> Result<Pair, String> {
    let profile = ExtractionProfile::from_facts(facts)?;
    if profile == ExtractionProfile::R0 {
        return Ok(pair);
    }
    let policy = profile.policy();
    let mut pdg: crate::PdgDocument = convert(&pair.pdg, &policy)?;
    let mut cfg: crate::CfgDocument = convert(&pair.cfg, &policy)?;
    configure_cfg(&mut cfg, &policy, profile);
    let evidence = selection_evidence(facts, profile)?;
    if profile.supports("rust.scalar-loops") {
        let cyclic = cyclic_policy()?;
        pdg.extensions.insert(
            "rstyle.hir-coverage:cyclic-flow-policy".into(),
            cyclic.clone(),
        );
        cfg.extensions
            .insert("rstyle.hir-coverage:cyclic-flow-policy".into(), cyclic);
    }
    pdg.extensions.insert(EVIDENCE_KEY.into(), evidence.clone());
    cfg.extensions.insert(EVIDENCE_KEY.into(), evidence);

    Ok(Pair { pdg, cfg })
}

fn configure_cfg(cfg: &mut crate::CfgDocument, policy: &ProfileId, profile: ExtractionProfile) {
    cfg.profile.name = policy.id.clone();
    cfg.profile.version = policy.version.clone();
    if profile.supports("rust.scalar-loops") {
        cfg.profile.exit_policy =
            "one normal exit; actual returns/fallthrough only; divergence remains cyclic".into();
    }
}

fn cyclic_policy() -> Result<Value, String> {
    serde_json::from_str(r#"{"reaching_definitions":"finite-monotone-kill-gen/1","postdominance":"all-maximal-paths-least-fixed-point/1","execution":"nearest-strict-entry-dominator/1","value_availability":"strict-dominance-without-conditional-value-merges/1","virtual_executable_exits":false}"#).map_err(|e| e.to_string())
}

fn selection_evidence(facts: &HirFnFacts, profile: ExtractionProfile) -> Result<Value, String> {
    let selection = SelectionEvidence {
        profile: profile.policy(),
        selection: profile.token(),
        capabilities: profile.capabilities(),
        contract: if profile == ExtractionProfile::HelpersV1 {
            "docs/extraction-profiles/helpers-v1.md"
        } else {
            "docs/extraction-profiles/coverage-v1.md"
        },
        signature: &facts.signature,
        frontend: facts,
        semantics: "local expansion conformance; bounded normal flow; exceptional flow omitted",
    };

    serde_json::to_value(selection).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct SelectionEvidence<'a> {
    profile: ProfileId,
    selection: &'static str,
    capabilities: &'static [&'static str],
    contract: &'static str,
    signature: &'a crate::SignatureFacts,
    frontend: &'a HirFnFacts,
    semantics: &'static str,
}

fn convert<T: Serialize + DeserializeOwned>(value: &T, policy: &ProfileId) -> Result<T, String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let value = replace_policies(value, policy);

    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn replace_policies(value: Value, policy: &ProfileId) -> Value {
    match value {
        Value::Object(object) => replace_object(object, policy),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|child| replace_policies(child, policy))
                .collect(),
        ),
        _ => value,
    }
}

fn replace_object(object: serde_json::Map<String, Value>, policy: &ProfileId) -> Value {
    let identity = object.get("id").and_then(Value::as_str) == Some(crate::R0_POLICY_ID)
        && object.get("version").and_then(Value::as_str) == Some(crate::R0_POLICY_VERSION);

    Value::Object(
        object
            .into_iter()
            .map(|(key, child)| {
                let child = match key.as_str() {
                    "id" if identity => json!(policy.id),
                    "version" if identity => json!(policy.version),
                    _ => replace_child(&key, child, policy),
                };
                (key, child)
            })
            .collect(),
    )
}

fn replace_child(key: &str, child: Value, policy: &ProfileId) -> Value {
    // Dependencies are already verified documents/frontend snapshots. Their
    // identities and policies must survive caller-policy transport unchanged.
    if key == "dependency" {
        return child;
    }
    if matches!(key, "description" | "normal_flow_scope")
        && let Some(text) = child.as_str()
    {
        json!(text.replace("R0", "local scalar").replace(
            "compat.rust.hir-r0/0.1.0",
            &format!("{}/{}", policy.id, policy.version)
        ))
    } else {
        replace_policies(child, policy)
    }
}
