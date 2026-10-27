//! Shared exact source, branch, and static-error evidence replay.
use super::*;
use crate::copy_tagged_values::origins::{
    Alternative, Case, CaseKind, Component, Condition, FormalRoot, Leaf, WireContract,
};

use crate::unit_store::EvidenceFacts;

type SourceNode =
    for<'a> fn(&PdgNode, &'a dyn EvidenceFacts) -> Option<Result<&'a crate::HirNode, String>>;
type ExpectedType = fn(&dyn EvidenceFacts, &crate::HirNode, &str) -> Result<String, String>;

/// Keep each family's original-source join and complete type proof together.
pub(crate) struct TypeReplay {
    pub source: SourceNode,
    pub expected: ExpectedType,
}

impl TypeReplay {
    pub fn verify(&self, node: &PdgNode, facts: &dyn EvidenceFacts) -> Option<Result<(), String>> {
        (self.source)(node, facts).map(|source| {
            let source = source?;
            let construct = node
                .attributes
                .construct
                .as_deref()
                .ok_or("Missing source construct")?;
            let token = (self.expected)(facts, source, construct)?;

            if wire_type(node)? != token {
                return Err("Source operation changes its exact complete value type".into());
            }
            Ok(())
        })
    }
}

pub(crate) fn alternative_edges(
    pair: PairView<'_>,
    merge: &PdgNode,
    expected: [(&PdgNode, &str); 2],
) -> Result<(), String> {
    let incoming: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == merge.id && edge.kind == crate::EdgeFamily::Data)
        .collect();
    if incoming.len() != expected.len() {
        return Err("Merge changes its complete data alternatives".into());
    }

    for (position, (value, outcome)) in expected.into_iter().enumerate() {
        if incoming
            .iter()
            .filter(|edge| {
                edge.source == value.id
                    && edge.subkind == "value_alternative"
                    && edge.operand_position == Some(position as u64)
                    && edge.outcome.as_deref() == Some(outcome)
            })
            .count()
            != 1
        {
            return Err("Merge changes its exact branch value, position or outcome".into());
        }
    }
    Ok(())
}

/// Callers independently prove the error route and exact saved Result scrutinee.
pub(crate) fn static_error_contract(
    scrutinee: &crate::HirRef,
    root: FormalRoot,
    route: Vec<Component>,
) -> WireContract {
    let error = Leaf {
        root,
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
                scrutinee: scrutinee.clone(),
                is_ok,
            },
            leaves: if is_ok { vec![] } else { vec![error.clone()] },
        })
        .collect();

    WireContract {
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
    }
}
