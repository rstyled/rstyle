//! Independently substitute the saved static message through the selected callback.
use super::*;
use crate::copy_tagged_values::origins::{
    Alternative, Case, CaseKind, Component, Condition, FormalRoot, Leaf, WireContract,
};

pub(super) fn message_route(
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<Vec<Component>, String> {
    let dependency = crate::helper_calls::verify_use(checked.map.constructor, facts)?;
    let contract = crate::helper_calls::named_errors::validate_shape(dependency.summary())?;
    let [alternative] = contract.alternatives.as_slice() else {
        return Err("ID callback must have one complete constructor alternative".into());
    };
    let [leaf] = alternative.leaves.as_slice() else {
        return Err("ID callback must retain exactly one static message component".into());
    };
    if leaf.root.parameter != 1
        || leaf.input != [Component::SharedReferent]
        || leaf.guard.is_some()
        || alternative.condition != Condition::Always
        || alternative.input != CaseKind::Whole
        || alternative.output != CaseKind::Whole
    {
        return Err("ID callback changes its selected constructor static-message route".into());
    }

    Ok(leaf.output.clone())
}

pub(super) fn contract(
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<WireContract, String> {
    let route = message_route(facts, checked)?;
    let error = Leaf {
        root: FormalRoot {
            parameter: 1,
            binding: facts.signature.params[1].pat.hir.clone(),
        },
        input: vec![Component::SharedReferent],
        output: route.clone(),
        guard: None,
    };
    let alternatives = [(true, CaseKind::ResultOk), (false, CaseKind::ResultErr)]
        .into_iter()
        .map(|(is_ok, output)| Alternative {
            input: CaseKind::Whole,
            output,
            condition: Condition::ResultTag {
                scrutinee: checked.decoder.hir.clone(),
                is_ok,
            },
            leaves: if is_ok { vec![] } else { vec![error.clone()] },
        })
        .collect();
    Ok(WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![vec![Component::SharedReferent]],
        }],
        output_cases: vec![
            Case {
                kind: CaseKind::ResultOk,
                components: vec![],
            },
            Case {
                kind: CaseKind::ResultErr,
                components: vec![route],
            },
        ],
        alternatives,
    })
}
