//! Boolean guards retain scope and explicit empty success; no nominal guard fiction.
use super::*;
use crate::copy_tagged_values::origins::{
    Alternative, Case, CaseKind, Component, Condition, FormalRoot, Leaf,
};
use serde::Serialize;

pub(super) fn contract(checked: &source::Checked<'_>) -> Result<WireContract, String> {
    let leaf = constructor_leaf(&checked.constructor)?;
    let route = std::iter::once(Component::ResultErr)
        .chain(leaf.output.clone())
        .collect::<Vec<_>>();
    let error = Leaf {
        root: FormalRoot {
            parameter: 2,
            binding: checked.instance.facts().signature.params[2].pat.hir.clone(),
        },
        input: leaf.input.clone(),
        output: route.clone(),
        guard: None,
    };

    let condition = checked.instance.condition().hir.clone();
    Ok(WireContract {
        input_cases: vec![Case {
            kind: CaseKind::Whole,
            components: vec![leaf.input.clone()],
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
        alternatives: alternatives(condition, error),
    })
}

fn constructor_leaf(contract: &WireContract) -> Result<&Leaf, String> {
    let [constructor] = contract.alternatives.as_slice() else {
        return Err("Slice error requires one complete constructor alternative".into());
    };
    let [leaf] = constructor.leaves.as_slice() else {
        return Err("Slice error requires its exact static constructor root".into());
    };
    if constructor.condition != Condition::Always
        || leaf.input != [Component::SharedReferent]
        || leaf.guard.is_some()
    {
        return Err("Slice error constructor changes its unconditional static input".into());
    }
    Ok(leaf)
}

fn alternatives(condition: crate::HirRef, error: Leaf) -> Vec<Alternative> {
    vec![
        Alternative {
            input: CaseKind::Whole,
            output: CaseKind::ResultOk,
            condition: Condition::Boolean {
                condition: condition.clone(),
                outcome: false,
            },
            leaves: vec![],
        },
        Alternative {
            input: CaseKind::Whole,
            output: CaseKind::ResultErr,
            condition: Condition::Boolean {
                condition,
                outcome: true,
            },
            leaves: vec![error],
        },
    ]
}

#[derive(Serialize)]
struct Origin<'a> {
    role: &'static str,
    condition: &'a crate::HirRef,
    selector_pdg_node_id: &'a str,
    outcome: Option<bool>,
    components: Vec<Leaf>,
}

pub(super) fn document(
    checked: &source::Checked<'_>,
    ops: &graph::Operations<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let contract = contract(checked)?;
    let leaf = &contract.alternatives[1].leaves[0];
    let context = Context {
        checked,
        selector: ops.selector,
    };
    ops.all()
        .into_iter()
        .map(|node| {
            let (role, outcome, components) = role(node, ops, leaf);
            Ok((
                node.id.clone(),
                context.annotation(role, outcome, components)?,
            ))
        })
        .collect()
}

fn role(
    node: &PdgNode,
    ops: &graph::Operations<'_>,
    leaf: &Leaf,
) -> (&'static str, Option<bool>, Vec<Leaf>) {
    if node.id == ops.inputs[2].id || node.id == ops.message.id {
        (
            "static_input",
            None,
            vec![Leaf {
                output: leaf.input.clone(),
                ..leaf.clone()
            }],
        )
    } else if node.id == ops.call.id {
        (
            "constructor",
            Some(true),
            vec![Leaf {
                output: leaf.output[1..].to_vec(),
                ..leaf.clone()
            }],
        )
    } else if node.id == ops.failure.id || node.id == ops.early_return.id {
        ("error", Some(true), vec![leaf.clone()])
    } else if node.id == ops.success.id || node.id == ops.returned.id {
        ("success", Some(false), vec![])
    } else {
        ("scalar", None, vec![])
    }
}

struct Context<'a, 'b> {
    checked: &'a source::Checked<'b>,
    selector: &'a PdgNode,
}

impl Context<'_, '_> {
    fn annotation(
        &self,
        role: &'static str,
        outcome: Option<bool>,
        components: Vec<Leaf>,
    ) -> Result<serde_json::Value, String> {
        serde_json::to_value(Origin {
            role,
            condition: &self.checked.instance.condition().hir,
            selector_pdg_node_id: &self.selector.id,
            outcome,
            components,
        })
        .map_err(|error| error.to_string())
    }
}
