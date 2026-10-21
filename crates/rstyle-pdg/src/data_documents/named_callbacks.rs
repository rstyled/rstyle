//! Independent checks for exact named Option callbacks and guarded helper inputs.
use super::*;
use crate::{HirNode, named_callbacks as contract, unit_store::EvidenceFacts};
use serde_json::Value;

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let calls =
        crate::option_callbacks::invocations(facts, crate::option_callbacks::Kind::FunctionItem);
    super::verify_construct_inventory(
        pair,
        [
            contract::ITEM,
            contract::SELECTOR,
            contract::PAYLOAD,
            contract::INVOKE,
            contract::RESULT,
        ],
        [calls.len(); 5],
    )?;
    if !calls.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, contract::CAPABILITY)
    {
        return Err("Named callbacks require their registered document policy".into());
    }

    for source in calls {
        invocation(pair, source, facts)?;
    }
    Ok(())
}

fn invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = contract::verify(source, facts)?;
    let item = super::typed_operation(pair, checked.callback, contract::ITEM, "unit")?;
    let selector = super::typed_operation(pair, source, contract::SELECTOR, "bool")?;
    let payload = super::typed_operation(pair, checked.callback, contract::PAYLOAD, "u8")?;
    let call = super::typed_operation(pair, checked.callback, contract::INVOKE, "bool")?;

    let result = super::value_operation(pair, source)?;
    if result.attributes.construct.as_deref() != Some(contract::RESULT) {
        return Err("Named callback loses its registered result merge".into());
    }
    call_effects(call, checked.callback)?;
    callbacks::effects(item, NodeCategory::Computation)?;
    callbacks::effects(selector, NodeCategory::Control)?;
    callbacks::effects(payload, NodeCategory::Computation)?;
    let receiver = super::value_operation(pair, checked.receiver)?;
    super::verify_saved_operands(pair, item, &[])?;
    super::verify_saved_operands(pair, selector, &[receiver])?;
    super::verify_saved_operands(pair, payload, &[receiver])?;
    super::verify_saved_operands(pair, call, &[payload])?;
    super::conditional_payloads::verify(pair, selector, payload)?;
    super::conditional_payloads::verify(pair, selector, call)?;
    callbacks::order::verify(pair, &checked, item, selector)?;
    callbacks::order::before(pair, &callbacks::order::completion(pair, payload)?, call)?;
    callbacks::order::result(
        pair,
        callbacks::order::ResultValues {
            call: source,
            body: call,
            default: checked.default,
        },
        selector,
    )
}

/// The source path has no argument children. Its invocation consumes precisely
/// the guarded payload event; the full invocation verifier checks its origin.
pub(crate) fn operands(
    pair: PairView<'_>,
    source: &HirNode,
    call: &PdgNode,
    record: &Value,
) -> Result<(), String> {
    let payload = super::typed_operation(pair, source, contract::PAYLOAD, "u8")?;
    if call.category != NodeCategory::Call
        || call.attributes.construct.as_deref() != Some(contract::INVOKE)
        || record["operand_pdg_node_ids"] != json!([payload.id])
    {
        return Err("Named invocation loses its exact guarded payload argument".into());
    }
    super::verify_saved_operands(pair, call, &[payload])
}

fn call_effects(call: &PdgNode, source: &HirNode) -> Result<(), String> {
    let callee = source
        .dispatch
        .as_ref()
        .ok_or("Missing named callback signature")?;
    if call.category != NodeCategory::Call
        || !call.attributes.reads.is_empty()
        || !call.attributes.writes.is_empty()
        || !call.attributes.defines.is_empty()
        || !call.attributes.symbol_ids.is_empty()
        || call
            .attributes
            .callee
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(callee.op.as_str())
        || call
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(callee.op.as_str())
        || call.attributes.call_form != Some(crate::CallForm::Direct)
        || call.attributes.literal.is_some()
        || call.attributes.write_form.is_some()
        || call.attributes.position.is_some()
        || call.attributes.exit_kind.is_some()
        || call.attributes.description.is_some()
        || !call.attributes.outcomes.is_empty()
        || call.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Named invocation invents effects or loses its exact direct callee".into());
    }
    Ok(())
}
