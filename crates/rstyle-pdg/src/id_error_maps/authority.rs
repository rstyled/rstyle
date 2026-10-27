//! Exact core map_err and monomorphized method interface; no callback purity by name.
use super::*;
use crate::concrete_types::Observation;

pub(super) fn verify(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let node = checked.source;

    let callee = crate::data_library::callee(node)?;

    crate::data_library::verify_core_definition(
        &callee,
        facts,
        crate::data_library::CoreDefinition {
            paths: &[
                "std::result::Result::<T, E>::map_err",
                "core::result::Result::<T, E>::map_err",
            ],
            unavailable: "Unpinned ID error map compiler/target",
            mismatch: "ID error map lacks exact selected core definition",
        },
    )?;

    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "ID error maps")?;

    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned ID error map compiler/target")?;

    if registry.source_digests.get("result.rs").map(String::as_str)
        != Some("sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1")
        || node.binding.is_some()
        || node.lit.is_some()
        || node.is_unsafe_block
    {
        return Err("ID error map loses its reviewed core source or ordinary invocation".into());
    }

    let observed = node
        .ty
        .as_ref()
        .ok_or("Missing ID error map observed output")?;

    crate::data_receivers::verify_dispatch(node, &callee.path, &[&observed.observed])?;
    signature(checked, facts)
}

fn signature(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let capture = crate::data_receivers::verify(facts)?;

    let record = capture
        .methods
        .iter()
        .find(|method| method.call == checked.source.hir)
        .ok_or("Missing ID error map method signature")?;

    let input = Observation::Captured {
        ty: checked.input.clone(),
    };

    if record.receiver_type != input
        || record.adjusted_type != input
        || !record.adjustments.is_empty()
        || record.signature_inputs.len() != 2
        || record.signature_inputs[0] != input
        || !matches!(&record.signature_inputs[1], Observation::Unsupported { reason } if !reason.is_empty())
        || record.signature_result
            != (Observation::Captured {
                ty: checked.output.clone(),
            })
    {
        return Err("ID error map changes concrete receiver, closure input or result".into());
    }

    Ok(())
}
