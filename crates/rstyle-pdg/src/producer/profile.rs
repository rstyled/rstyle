//! Local policy transport, preserving the pinned wire schema and base vocabulary.

mod shape;
#[cfg(test)]
mod tests;
mod transport;

use super::Pair;
use crate::{ProfileId, extraction_profile::*};
use serde::Serialize;
#[cfg(test)]
use serde::de::DeserializeOwned;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;

pub fn apply(
    pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &super::Graph,
) -> Result<Pair, String> {
    crate::diagnostics::Stage::new("producer.profile", &facts.name, None)
        .observe(|| apply_profile(pair, facts, graph))
}

fn apply_profile(
    pair: Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &super::Graph,
) -> Result<Pair, String> {
    let profile = ExtractionProfile::from_facts(facts)?;
    if profile == ExtractionProfile::R0 {
        return Ok(pair);
    }
    let policy = profile.policy();
    let Pair { mut pdg, mut cfg } = transport::apply(pair, &policy)?;
    configure_cfg(&mut cfg, &policy, profile);
    let evidence = selection_evidence(facts, profile)?;
    if profile.supports("rust.scalar-loops") {
        let cyclic = cyclic_policy(profile)?;
        pdg.extensions.insert(
            "rstyle.hir-coverage:cyclic-flow-policy".into(),
            cyclic.clone(),
        );
        cfg.extensions
            .insert("rstyle.hir-coverage:cyclic-flow-policy".into(), cyclic);
    }
    pdg.extensions.insert(EVIDENCE_KEY.into(), evidence.clone());
    cfg.extensions.insert(EVIDENCE_KEY.into(), evidence);

    let pair = Pair { pdg, cfg };
    if profile.supports("rust.concrete-read-only-data") {
        crate::data_documents::apply(pair, facts, graph)
    } else {
        Ok(pair)
    }
}

fn configure_cfg(cfg: &mut crate::CfgDocument, policy: &ProfileId, profile: ExtractionProfile) {
    cfg.profile = configured_cfg_profile(cfg, policy, profile);
}

fn configured_cfg_profile(
    cfg: &crate::CfgDocument,
    policy: &ProfileId,
    profile: ExtractionProfile,
) -> crate::cfg::CfgProfile {
    let exit_policy = if cfg.exit_node_ids.iter().any(|exit| exit.starts_with("P:")) {
        "normal completion and explicit checked-failure exits; divergence remains cyclic".into()
    } else if profile.supports("rust.scalar-loops") {
        "one normal exit; actual returns/fallthrough only; divergence remains cyclic".into()
    } else {
        cfg.profile.exit_policy.clone()
    };

    crate::cfg::CfgProfile {
        name: policy.id.clone(),
        version: policy.version.clone(),
        exit_policy,
        ..cfg.profile.clone()
    }
}

fn cyclic_policy(profile: ExtractionProfile) -> Result<Value, String> {
    let mut policy: Value = serde_json::from_str(r#"{"reaching_definitions":"finite-monotone-kill-gen/1","postdominance":"all-maximal-paths-least-fixed-point/1","execution":"nearest-strict-entry-dominator/1","value_availability":"strict-dominance-without-conditional-value-merges/1","virtual_executable_exits":false}"#).map_err(|e| e.to_string())?;
    if profile.supports("rust.outside-loop-value-merges") {
        policy["value_availability"] = "outside-cycle-guard-states-and-must-evaluation/1".into();
    }
    if profile.supports("rust.cyclic-value-merges") {
        policy["value_availability"] = "natural-loop-kill-gen-and-guard-states/1".into();
    }
    Ok(policy)
}

fn selection_evidence(
    facts: &dyn crate::unit_store::EvidenceFacts,
    profile: ExtractionProfile,
) -> Result<Value, String> {
    let frontend_ref = facts.frontend_reference();
    let selection = SelectionEvidence {
        profile: profile.policy(),
        selection: profile.token(),
        capabilities: profile.capabilities(),
        signature: &facts.signature,
        frontend: frontend_ref.is_none().then_some(facts),
        frontend_ref,
        semantics: if crate::mutable_scalars::enabled(facts) {
            "local expansion conformance; bounded scalar referent memory flow and explicit checked failures; no owned cleanup"
        } else {
            "local expansion conformance; bounded normal flow; exceptional flow omitted"
        },
    };

    serde_json::to_value(selection).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct SelectionEvidence<'a> {
    profile: ProfileId,
    selection: &'static str,
    capabilities: &'static [&'static str],
    signature: &'a crate::SignatureFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    frontend: Option<&'a dyn crate::unit_store::EvidenceFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frontend_ref: Option<Value>,
    semantics: &'static str,
}

#[cfg(test)]
fn convert<T: Serialize + DeserializeOwned>(value: &T, policy: &ProfileId) -> Result<T, String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let value = replace_policies(value, policy);

    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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
