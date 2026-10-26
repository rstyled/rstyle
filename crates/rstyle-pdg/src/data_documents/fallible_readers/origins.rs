//! Explicit output cases; the callee predicate stays beneath its exact saved call.
use super::*;
use crate::copy_tagged_values::origins::{CaseKind, Component, FormalRoot};
use crate::data_documents::guarded_sources::insert_role as insert;
use serde_json::Value;

pub(super) fn contract(
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<crate::copy_tagged_values::origins::WireContract, String> {
    use crate::copy_tagged_values::origins::{Alternative, Case, Condition, Leaf, WireContract};
    let route = error_route(checked)?;

    let error = Leaf {
        root: root(facts, 1),
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
                scrutinee: checked.propagation.operand.hir.clone(),
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

#[derive(serde::Serialize)]
struct Origin {
    root: FormalRoot,
    route: Vec<Component>,
}

#[derive(serde::Serialize)]
struct References {
    role: &'static str,
    origins: Vec<Origin>,
}

#[derive(serde::Serialize)]
struct Scope<'a> {
    call: &'a crate::HirRef,
    call_pdg_node_id: &'a str,
    actuals: [&'a crate::HirRef; 3],
    saved_actual_pdg_node_ids: [&'a str; 3],
    dependency: &'a Value,
    callee_contract: &'a crate::copy_tagged_values::origins::WireContract,
    condition_scope: &'static str,
}

#[derive(serde::Serialize)]
struct Case {
    variant: &'static str,
    origins: Vec<Origin>,
}

#[derive(serde::Serialize)]
struct Length<'a> {
    role: &'static str,
    scope: &'a Scope<'a>,
    cases: [Case; 2],
}

#[derive(serde::Serialize)]
struct Guard<'a> {
    scrutinee: &'a crate::HirRef,
    selector_pdg_node_id: &'a str,
    outcome: &'static str,
    variant: &'static str,
}

#[derive(serde::Serialize)]
struct Error<'a> {
    role: &'static str,
    scope: &'a Scope<'a>,
    guard: Guard<'a>,
    origins: [Origin; 1],
}

pub(super) fn derive(
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
    ops: &graph::Operations<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    if facts.body.children.iter().any(|node| node.is_unsafe_block) {
        return Err("Reader cannot publish origins for unsafe work".into());
    }

    let mut roles = BTreeMap::new();
    scalar_roles(&mut roles, ops)?;
    formal_roles(&mut roles, facts, ops)?;
    error_roles(&mut roles, facts, checked, ops)?;

    Ok(roles)
}

fn scalar_roles(
    roles: &mut BTreeMap<String, Value>,
    ops: &graph::Operations<'_>,
) -> Result<(), String> {
    for (role, nodes) in [
        ("scalar_size", vec![ops.arguments[1], ops.consumed]),
        ("tag_only", vec![ops.selector]),
        ("unit_success", vec![ops.unit]),
        ("native_scalar", vec![ops.reader]),
        ("scalar_tuple", vec![ops.tuple]),
        ("ok_result", vec![ops.success, ops.returned]),
    ] {
        for node in nodes {
            insert(
                roles,
                node,
                References {
                    role,
                    origins: vec![],
                },
            )?;
        }
    }
    Ok(())
}

fn formal_roles(
    roles: &mut BTreeMap<String, Value>,
    facts: &dyn EvidenceFacts,
    ops: &graph::Operations<'_>,
) -> Result<(), String> {
    for (position, nodes) in [
        (
            0,
            vec![ops.inputs[0], ops.arguments[0], ops.reader_argument],
        ),
        (1, vec![ops.inputs[1], ops.arguments[2]]),
    ] {
        for node in nodes {
            insert(
                roles,
                node,
                References {
                    role: "formal_reference",
                    origins: vec![Origin {
                        root: root(facts, position),
                        route: vec![Component::SharedReferent],
                    }],
                },
            )?;
        }
    }
    Ok(())
}

fn error_roles(
    roles: &mut BTreeMap<String, Value>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
    ops: &graph::Operations<'_>,
) -> Result<(), String> {
    let route = error_route(checked)?;
    let scope = scope(checked, ops);
    insert(roles, ops.length, length(&scope, root(facts, 1), &route))?;

    for node in [ops.error, ops.error_return] {
        insert(
            roles,
            node,
            Error {
                role: "propagated_error",
                scope: &scope,
                guard: Guard {
                    scrutinee: &checked.propagation.operand.hir,
                    selector_pdg_node_id: &ops.selector.id,
                    outcome: "true",
                    variant: "Err",
                },
                origins: [Origin {
                    root: root(facts, 1),
                    route: route.clone(),
                }],
            },
        )?;
    }
    Ok(())
}

fn length<'a>(scope: &'a Scope<'a>, root: FormalRoot, route: &[Component]) -> Length<'a> {
    Length {
        role: "length_result",
        scope,
        cases: [
            Case {
                variant: "Ok",
                origins: vec![],
            },
            Case {
                variant: "Err",
                origins: vec![Origin {
                    root,
                    route: route.to_vec(),
                }],
            },
        ],
    }
}

fn scope<'a>(checked: &'a source::Checked<'_>, ops: &'a graph::Operations<'_>) -> Scope<'a> {
    Scope {
        call: &checked.propagation.operand.hir,
        call_pdg_node_id: &ops.length.id,
        actuals: checked.length_arguments.map(|argument| &argument.hir),
        saved_actual_pdg_node_ids: ops.arguments.map(|argument| argument.id.as_str()),
        dependency: &checked.length_dependency,
        callee_contract: &checked.length_contract,
        condition_scope: "exact-selected-callee-under-this-call/1",
    }
}

fn error_route(checked: &source::Checked<'_>) -> Result<Vec<Component>, String> {
    let errors: Vec<_> = checked
        .length_contract
        .alternatives
        .iter()
        .filter(|alternative| alternative.output == CaseKind::ResultErr)
        .collect();

    let [alternative] = errors.as_slice() else {
        return Err("Reader requires one independently derived error alternative".into());
    };
    let [leaf] = alternative.leaves.as_slice() else {
        return Err("Reader error must carry exactly one saved static message component".into());
    };
    if leaf.root.parameter != 2
        || leaf.input != [Component::SharedReferent]
        || leaf.output.first() != Some(&Component::ResultErr)
        || leaf.output.last() != Some(&Component::SharedReferent)
        || leaf.guard.is_some()
    {
        return Err(
            "Reader error changes the exact static-formal/ResultErr component route".into(),
        );
    }
    Ok(leaf.output.clone())
}

fn root(facts: &dyn EvidenceFacts, position: usize) -> FormalRoot {
    FormalRoot {
        parameter: position,
        binding: facts.signature.params[position].pat.hir.clone(),
    }
}
