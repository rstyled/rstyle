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
    cfg.profile.name = policy.id.clone();
    cfg.profile.version = policy.version.clone();
    if profile.scalar_loops() {
        cfg.profile.exit_policy =
            "one normal exit; actual returns/fallthrough only; divergence remains cyclic".into();
        let cyclic = json!({"reaching_definitions":"finite-monotone-kill-gen/1","postdominance":"all-maximal-paths-least-fixed-point/1","execution":"nearest-strict-entry-dominator/1","value_availability":"strict-dominance-without-conditional-value-merges/1","virtual_executable_exits":false});
        pdg.extensions.insert(
            "rstyle.hir-coverage:cyclic-flow-policy".into(),
            cyclic.clone(),
        );
        cfg.extensions
            .insert("rstyle.hir-coverage:cyclic-flow-policy".into(), cyclic);
    }
    let evidence = json!({
        "profile": policy,
        "selection": profile.token(),
        "capabilities": profile.capabilities(),
        "contract": "docs/extraction-profiles/coverage-v1.md",
        "signature": facts.signature,
        "frontend": facts,
        "semantics": "local expansion conformance; bounded normal flow; exceptional flow omitted"
    });
    pdg.extensions.insert(EVIDENCE_KEY.into(), evidence.clone());
    cfg.extensions.insert(EVIDENCE_KEY.into(), evidence);

    Ok(Pair { pdg, cfg })
}

fn convert<T: Serialize + DeserializeOwned>(value: &T, policy: &ProfileId) -> Result<T, String> {
    let mut value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    replace_policies(&mut value, policy);

    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn replace_policies(value: &mut Value, policy: &ProfileId) {
    match value {
        Value::Object(object) => {
            if object.get("id").and_then(Value::as_str) == Some(crate::R0_POLICY_ID)
                && object.get("version").and_then(Value::as_str) == Some(crate::R0_POLICY_VERSION)
            {
                object.insert("id".into(), json!(policy.id));
                object.insert("version".into(), json!(policy.version));
            }
            for (key, child) in object {
                if matches!(key.as_str(), "description" | "normal_flow_scope")
                    && let Some(text) = child.as_str()
                {
                    *child = json!(text.replace("R0", "local scalar").replace(
                        "compat.rust.hir-r0/0.1.0",
                        &format!("{}/{}", policy.id, policy.version)
                    ));
                } else {
                    replace_policies(child, policy);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                replace_policies(child, policy);
            }
        }
        _ => {}
    }
}
