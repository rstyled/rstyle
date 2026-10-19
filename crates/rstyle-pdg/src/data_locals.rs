//! Whole local values use existing binding definitions and reads.

use crate::{HirNode, extraction_profile::ExtractionProfile, unit_store::EvidenceFacts};

pub(crate) fn supported(token: &str, profile: ExtractionProfile) -> bool {
    !crate::concrete_types::scalar(token)
        && (profile.supports("rust.local-aggregate-values")
            || profile.supports("rust.shared-reference-locals")
            || profile.supports("rust.reference-bearing-values"))
        && crate::data_types::parse(token).is_ok_and(|ty| {
            (profile.supports("rust.local-aggregate-values")
                && crate::data_aggregates::reference_free(&ty))
                || crate::data_reference_locals::supported(&ty, profile)
        })
}

pub(crate) fn verify_node(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let (target, value) = match node.kind.as_str() {
        "stmt_let" => {
            let Some(target) = node
                .children
                .iter()
                .find(|child| child.kind == "pat_binding")
            else {
                return Ok(());
            };
            let Some(value) = node
                .children
                .iter()
                .find(|child| child.kind.starts_with("expr_"))
            else {
                return Ok(());
            };
            (target, value)
        }
        "expr_assign" => {
            let [target, value] = node.children.as_slice() else {
                return Ok(());
            };
            (target, value)
        }
        _ => return Ok(()),
    };
    let token = crate::data_aggregates::token(target)?;
    if crate::concrete_types::scalar(token)
        || !supported(token, ExtractionProfile::from_facts(facts)?)
    {
        return Ok(());
    }

    if (node.kind == "expr_assign" || target.binding_mut == Some(true))
        && crate::data_types::parse(token).is_ok_and(|ty| crate::data_origins::shared(&ty))
    {
        return Err("Reference-bearing locals require immutable initialized bindings".into());
    }

    if value
        .ty
        .as_ref()
        .is_none_or(|ty| ty.observed != "!" && ty.normalized.as_deref() != Some(token))
    {
        return Err("Local aggregate definition value disagrees with its binding type".into());
    }
    Ok(())
}
