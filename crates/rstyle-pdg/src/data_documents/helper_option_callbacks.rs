//! Independent helper callback guards, saved arguments, payload locations and result.
use super::*;
use crate::MemoryLocation;
use crate::helper_option_callbacks::{self as contract, Checked};

pub(super) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let calls = contract::sources(context.facts);
    super::verify_construct_inventory(
        pair,
        [contract::ENVIRONMENT, contract::SELECTOR, contract::RESULT],
        [calls.len(); 3],
    )?;
    if !calls.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, contract::CAPABILITY)
    {
        return Err("Helper callbacks require their serialized policy".into());
    }
    let mut entries = BTreeMap::new();

    for source in calls {
        let checked = contract::verify(source, context.facts)?;
        entries.extend(invocation(pair, &checked, context)?);
    }
    Ok(entries)
}

fn invocation(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let arguments = &checked.arguments;
    let environment =
        super::typed_operation(pair, arguments.callback, contract::ENVIRONMENT, "unit")?;
    let selector = super::typed_operation(pair, arguments.call, contract::SELECTOR, "bool")?;

    super::callbacks::effects(environment, NodeCategory::Computation)?;
    super::callbacks::effects(selector, NodeCategory::Control)?;
    super::verify_saved_operands(pair, environment, &[])?;

    let receiver = super::value_operation(pair, arguments.receiver)?;
    super::verify_saved_operands(pair, selector, &[receiver])?;

    super::callbacks::order::verify(pair, arguments, environment, selector)?;
    let entries = payload(pair, checked, context)?;
    guard_body(pair, checked, selector)?;
    let body = super::value_operation(pair, checked.body)?;
    super::callbacks::order::result(
        pair,
        super::callbacks::order::ResultValues {
            call: arguments.call,
            body,
            default: arguments.default,
        },
        selector,
    )?;
    Ok(entries)
}

fn payload(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let (construct, reads, entries) = if checked.borrowed {
        let (reads, entries) =
            context.read_accesses(contract::locations(checked.binding, context.facts)?)?;
        ("data.option-payload", reads, entries)
    } else {
        ("data.option-value-payload", Vec::new(), BTreeMap::new())
    };
    let load = super::typed_operation(pair, checked.binding, construct, "u8")?;
    super::checked_options::effects(load, construct, &reads)?;
    let receiver = super::value_operation(pair, checked.arguments.receiver)?;
    super::verify_saved_operands(pair, load, &[receiver])?;

    super::conditional_payloads::binding(pair, checked.binding, load)?;

    let selector =
        super::typed_operation(pair, checked.arguments.call, contract::SELECTOR, "bool")?;
    super::conditional_payloads::verify(pair, selector, load)?;
    let declaration = super::serialized_operation(pair, &checked.binding.span, |node| {
        node.category == NodeCategory::Declaration
    })?;
    super::conditional_payloads::verify(pair, selector, declaration)?;
    Ok(entries)
}

fn guard_body(pair: PairView<'_>, checked: &Checked<'_>, selector: &PdgNode) -> Result<(), String> {
    for source in std::iter::once(checked.body).chain(&checked.body.children) {
        let node = super::value_operation(pair, source)?;
        super::conditional_payloads::verify(pair, selector, node)?;
    }
    Ok(())
}
