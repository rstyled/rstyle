//! Independent source-to-serialized checks for immediate Option callbacks.

pub(super) mod order;

use super::*;
use crate::{HirNode, option_callbacks, unit_store::EvidenceFacts};

const ENVIRONMENT: &str = "callback.shared-byte-environment";
const SELECTOR: &str = "callback.option-some";

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let calls = option_callbacks::invocations(facts, crate::option_callbacks::Kind::Closure);
    super::verify_construct_inventory(
        pair,
        [ENVIRONMENT, SELECTOR, "callback.option-result"],
        [calls.len(); 3],
    )?;
    if !calls.is_empty()
        && !crate::extraction_profile::policy_supports(
            &pair.pdg.profile,
            "rust.immediate-option-callbacks",
        )
    {
        return Err("Immediate callbacks require their serialized document policy".into());
    }

    for source in calls {
        verify_invocation(pair, source, facts)?;
    }
    Ok(())
}

fn verify_invocation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = option_callbacks::verify_map_or(source, facts)?;
    let environment = super::typed_operation(pair, checked.callback, ENVIRONMENT, "unit")?;
    let selector = super::typed_operation(pair, source, SELECTOR, "bool")?;
    effects(environment, NodeCategory::Computation)?;
    effects(selector, NodeCategory::Control)?;
    super::verify_saved_operands(pair, environment, &[])?;
    let receiver = super::value_operation(pair, checked.receiver)?;
    super::verify_saved_operands(pair, selector, &[receiver])?;

    payload(pair, &checked, selector, receiver)
        .map_err(|error| format!("Callback payload/result: {error}"))?;
    order::verify(pair, &checked.arguments(), environment, selector)
        .map_err(|error| format!("Callback argument order: {error}"))?;
    Ok(())
}

fn payload(
    pair: PairView<'_>,
    checked: &option_callbacks::Invocation<'_>,
    selector: &PdgNode,
    receiver: &PdgNode,
) -> Result<(), String> {
    let binding = checked.predicate.binding;
    let load = super::typed_operation(pair, binding, "data.option-value-payload", "u8")?;
    super::checked_options::effects(load, "data.option-value-payload", &[])?;
    super::verify_saved_operands(pair, load, &[receiver])?;
    super::conditional_payloads::binding(pair, binding, load)?;
    super::conditional_payloads::verify(pair, selector, load)?;

    for body in std::iter::once(checked.predicate.body).chain(&checked.predicate.body.children) {
        let node = super::value_operation(pair, body)?;
        super::conditional_payloads::verify(pair, selector, node)?;
    }
    let declaration = super::serialized_operation(pair, &binding.span, |node| {
        node.category == NodeCategory::Declaration
    })?;
    super::conditional_payloads::verify(pair, selector, declaration)?;
    let body = super::value_operation(pair, checked.predicate.body)?;
    order::result(
        pair,
        order::ResultValues {
            call: checked.call,
            body,
            default: checked.default,
        },
        selector,
    )
}

pub(super) fn effects(node: &PdgNode, category: NodeCategory) -> Result<(), String> {
    if node.category != category
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.attributes.operator.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || node.attributes.exit_kind.is_some()
        || node.attributes.description.is_some()
        || node.attributes.outcomes
            != if category == NodeCategory::Control {
                vec!["true", "false"]
            } else {
                Vec::new()
            }
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err(
            "Callback environment/tag selection invents content reads, writes or calls".into(),
        );
    }
    Ok(())
}
