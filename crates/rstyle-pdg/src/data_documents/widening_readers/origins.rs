//! Callee-scoped short errors and checked unreachable conversion errors stay distinct.
use super::*;
use crate::copy_tagged_values::origins::{
    Alternative, CaseKind, Component, Condition, FormalRoot, WireContract,
};
use serde_json::{Value, json};

pub(super) fn routes(checked: &contract::Checked<'_>) -> Result<[Vec<Component>; 2], String> {
    let reader = &checked.proof.reader_contract;
    let [success, failure] = reader.output_cases.as_slice() else {
        return Err("Widening checked reader requires both complete Result cases".into());
    };
    if success.kind != CaseKind::ResultOk
        || !success.components.is_empty()
        || failure.kind != CaseKind::ResultErr
        || failure.components.len() != 1
        || reader.alternatives.len() != 2
    {
        return Err("Widening checked reader changes its scalar success or short error".into());
    }
    let short = failure.components[0].clone();
    for alternative in &reader.alternatives {
        reader_alternative(alternative, &short)?;
    }

    let error = &checked.proof.error_contract;
    let [alternative] = error.alternatives.as_slice() else {
        return Err("Widening conversion error requires one whole constructor alternative".into());
    };
    let [leaf] = alternative.leaves.as_slice() else {
        return Err("Widening conversion error loses its exact static-message component".into());
    };
    if alternative.input != CaseKind::Whole
        || alternative.output != CaseKind::Whole
        || alternative.condition != Condition::Always
        || leaf.root.parameter != 0
        || leaf.input != [Component::SharedReferent]
        || leaf.guard.is_some()
    {
        return Err("Widening conversion error changes its selected constructor route".into());
    }
    Ok([short, leaf.output.clone()])
}

fn reader_alternative(alternative: &Alternative, route: &[Component]) -> Result<(), String> {
    if alternative.input != CaseKind::Whole {
        return Err("Widening reader changes its whole selected input".into());
    }
    match alternative.output {
        CaseKind::ResultOk if alternative.leaves.is_empty() => Ok(()),
        CaseKind::ResultErr => {
            let [leaf] = alternative.leaves.as_slice() else {
                return Err("Widening short error requires exactly one static origin".into());
            };
            if leaf.root.parameter != 1
                || leaf.input != [Component::SharedReferent]
                || leaf.output != route
                || leaf.guard.is_some()
            {
                return Err("Widening short error changes its selected message route".into());
            }
            Ok(())
        }
        _ => Err("Widening reader changes its complete selected Result alternatives".into()),
    }
}

pub(super) fn contract(
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
) -> Result<WireContract, String> {
    let [short, _] = routes(checked)?;
    Ok(super::super::checked_roles::static_error_contract(
        &checked.roles.reader.hir,
        root(facts, 1),
        short,
    ))
}

pub(super) fn root(facts: &dyn EvidenceFacts, parameter: usize) -> FormalRoot {
    FormalRoot {
        parameter,
        binding: facts.signature.params[parameter].pat.hir.clone(),
    }
}

pub(super) fn reference(facts: &dyn EvidenceFacts, parameter: usize, route: &[Component]) -> Value {
    json!({"root":root(facts, parameter), "route":route})
}
