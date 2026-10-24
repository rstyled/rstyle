//! Whole-value forwarding binds a callee contract to one saved caller argument.
use super::*;
use crate::{HirRef, copy_tagged_values::origins::WireContract, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub(crate) const KEY: &str = "rstyle.data:copy-tagged-call-contract";
const ROLE_KEY: &str = "rstyle.data:copy-tagged-call-role";

/// Callee guards are scoped beneath this exact call; they are not caller CFG guards.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    call: HirRef,
    actual: HirRef,
    caller_binding: HirRef,
    caller_parameter: u32,
    callee: WireContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Input,
    Output,
}

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = crate::copy_tagged_values::call_wrappers::verify(facts)? else {
        return Ok(false);
    };
    let contract = contract(facts, &checked)?;
    let roles = graph(pair.as_view(), facts, &checked)?;

    pair.pdg.extensions.insert(
        KEY.into(),
        serde_json::to_value(contract).map_err(|error| error.to_string())?,
    );
    for node in &mut pair.pdg.nodes {
        let role = roles.get(&node.id).ok_or("Unaccounted tagged call node")?;
        node.annotations.insert(
            ROLE_KEY.into(),
            serde_json::to_value(role).map_err(|error| error.to_string())?,
        );
    }
    Ok(true)
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = crate::copy_tagged_values::call_wrappers::verify(facts)? else {
        if pair.pdg.extensions.contains_key(KEY)
            || pair
                .pdg
                .nodes
                .iter()
                .any(|node| node.annotations.contains_key(ROLE_KEY))
        {
            return Err("Unproved function claims a tagged call contract".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        "rust.copy-tagged-conversion-helper-calls",
        || true,
        "Tagged forwarding requires its registered document policy",
    )?;

    let actual: Contract = serde_json::from_value(
        pair.pdg
            .extensions
            .get(KEY)
            .ok_or("Missing scoped tagged call contract")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    if actual != contract(facts, &checked)? {
        return Err(
            "Tagged call contract changes its saved argument, scope or callee proof".into(),
        );
    }
    let roles = graph(pair, facts, &checked)?;
    for node in &pair.pdg.nodes {
        let role: Role = serde_json::from_value(
            node.annotations
                .get(ROLE_KEY)
                .ok_or("Missing tagged call value role")?
                .clone(),
        )
        .map_err(|error| error.to_string())?;
        if roles.get(&node.id) != Some(&role) {
            return Err("Tagged call changes input/output correspondence".into());
        }
    }
    Ok(true)
}

fn contract(
    facts: &dyn EvidenceFacts,
    checked: &crate::copy_tagged_values::call_wrappers::Checked<'_>,
) -> Result<Contract, String> {
    let formal = facts
        .signature
        .params
        .first()
        .ok_or("Missing tagged caller formal")?;
    Ok(Contract {
        call: checked.call().hir.clone(),
        actual: checked.actual().hir.clone(),
        caller_binding: formal.pat.hir.clone(),
        caller_parameter: formal.position,
        callee: checked.contract().clone(),
    })
}

struct Values<'a> {
    input: &'a PdgNode,
    saved: &'a PdgNode,
    call: &'a PdgNode,
    returned: &'a PdgNode,
}

fn operations<'a>(
    pair: PairView<'a>,
    facts: &dyn EvidenceFacts,
    checked: &crate::copy_tagged_values::call_wrappers::Checked<'_>,
) -> Result<Values<'a>, String> {
    let formal = &facts.signature.params[0];
    let values = Values {
        input: serialized_operation(pair, &formal.pat.span, |node| {
            node.category == NodeCategory::FormalInput
        })?,
        saved: value_operation(pair, checked.actual())?,
        call: value_operation(pair, checked.call())?,
        returned: serialized_operation(pair, &checked.call().span, |node| {
            node.category == NodeCategory::Return
        })?,
    };

    if pair.pdg.nodes.len() != 4
        || !pair.pdg.memory_locations.is_empty()
        || pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Complete
        || values.input.attributes.position != Some(0)
        || values.input.attributes.symbol_ids != [formal.symbol.clone()]
        || values.saved.category != NodeCategory::Read
        || values.call.category != NodeCategory::Call
    {
        return Err(
            "Tagged forwarding requires its complete four-operation saved-value graph".into(),
        );
    }
    call_effects(values.call)?;
    Ok(values)
}

fn call_effects(call: &PdgNode) -> Result<(), String> {
    if !call.attributes.reads.is_empty()
        || !call.attributes.writes.is_empty()
        || !call.attributes.defines.is_empty()
        || !call.attributes.symbol_ids.is_empty()
        || call.attributes.literal.is_some()
        || call.attributes.call_form != Some(crate::CallForm::Direct)
    {
        return Err("Tagged call invents effects or changes direct dispatch".into());
    }
    Ok(())
}

fn graph(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &crate::copy_tagged_values::call_wrappers::Checked<'_>,
) -> Result<BTreeMap<String, Role>, String> {
    let values = operations(pair, facts, checked)?;
    edges(pair, &values)?;
    copy_tagged_values::graph::returned(pair, checked.call(), values.call)?;

    let roles = BTreeMap::from([
        (values.input.id.clone(), Role::Input),
        (values.saved.id.clone(), Role::Input),
        (values.call.id.clone(), Role::Output),
        (values.returned.id.clone(), Role::Output),
    ]);
    let executable = [
        values.saved.id.clone(),
        values.call.id.clone(),
        values.returned.id.clone(),
    ]
    .into_iter()
    .collect();
    if live_operations(pair)? != executable {
        return Err("Tagged call includes unreachable or omitted value operations".into());
    }
    inventory(pair, checked, &roles)?;
    Ok(roles)
}

fn edges(pair: PairView<'_>, values: &Values<'_>) -> Result<(), String> {
    let Values {
        input,
        saved,
        call,
        returned,
    } = *values;
    copy_tagged_values::origins::verify_definition(pair, input, saved)?;
    verify_saved_operands(pair, call, &[saved])?;
    verify_saved_operands(pair, returned, &[call])?;

    for (value, consumer, kind) in [
        (input, saved, "reaching_definition"),
        (saved, call, "value_operand"),
        (call, returned, "value_operand"),
    ] {
        value_consumers::verify(pair, value, &[consumer], kind)?;
        if value.category != NodeCategory::FormalInput {
            callbacks::order::before(pair, &callbacks::order::completion(pair, value)?, consumer)?;
        }
    }
    value_consumers::verify(pair, returned, &[], "value_operand")
}

fn inventory(
    pair: PairView<'_>,
    checked: &crate::copy_tagged_values::call_wrappers::Checked<'_>,
    roles: &BTreeMap<String, Role>,
) -> Result<(), String> {
    if pair
        .pdg
        .extensions
        .contains_key(copy_tagged_values::origins::KEY)
    {
        return Err("Tagged caller cannot claim an unscoped callee value contract".into());
    }
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_origins::KEY)
            || node
                .annotations
                .contains_key(copy_tagged_values::origins::ROLE_KEY)
        {
            return Err("Tagged forwarding cannot flatten guarded component origins".into());
        }
        let role = roles.get(&node.id).ok_or("Unaccounted tagged call value")?;
        let tree = if *role == Role::Input {
            checked.input()
        } else {
            checked.output()
        };
        let token = crate::data_types::TypeToken::from_tree(tree).text()?;
        if node.annotations["compat.rust.hir-r0:type"]["value"].as_str() != Some(token.as_str()) {
            return Err("Tagged call value has another exact nominal endpoint type".into());
        }
    }
    Ok(())
}
