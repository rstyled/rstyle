//! Final permissions come only from completed nominal selection and helper proof.
use super::{request::TransportRequest, shape::Chain};
use crate::{nominal_boundary::resolution::Resolution, unit_store::EvidenceFacts};

pub(super) fn verify(
    facts: &dyn EvidenceFacts,
    resolution: &Resolution,
    request: &TransportRequest,
    chain: &Chain<'_>,
) -> Result<(), String> {
    for tree in request.copy_requests() {
        if !resolution.checked_owned_copy(tree) {
            return Err("Constructor Result transport awaits exact success Copy permission".into());
        }
    }
    for tree in request.cleanup_requests() {
        let nominal = tree
            .nominal_identity()
            .ok_or("Transport cleanup payload lacks nominal owner")?;
        if !resolution.checked_no_drop(nominal) || resolution.checked_owned_copy(tree) {
            return Err(
                "Constructor Result transport awaits non-Copy no-drop error permission".into(),
            );
        }
    }
    helper(facts, request, chain)
}

fn helper(
    facts: &dyn EvidenceFacts,
    request: &TransportRequest,
    chain: &Chain<'_>,
) -> Result<(), String> {
    let dependency = crate::helper_calls::verify_use(chain.leaf, facts)?;
    let summary = dependency.summary();
    let expected = crate::data_types::token(request.result_types()[0])?;
    let parameter = crate::data_aggregates::token(chain.argument)?;
    if summary.result_type != expected
        || summary.argument_types != [parameter]
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
        || summary.bounded_loop.is_some()
        || summary.reference_outputs.is_some()
        || summary.component_outputs.is_some()
        || summary.memory_reads.is_some()
    {
        return Err("Transport leaf lacks its exact pure, total, owned Result helper proof".into());
    }

    Ok(())
}
