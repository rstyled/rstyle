//! Explicit static storage roots remain scoped to their original saved helper call.
use super::*;
use crate::copy_tagged_values::origins::{Component, WireContract};
use crate::data_documents::guarded_sources::insert_role as insert;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Root<'a> {
    Formal {
        parameter: usize,
        binding: &'a crate::HirRef,
    },
    StaticLiteral {
        descriptor: &'a crate::static_string_values::Descriptor,
    },
}
#[derive(Clone, Serialize)]
struct Origin<'a> {
    root: Root<'a>,
    route: Vec<Component>,
}
#[derive(Serialize)]
struct Carried<'a> {
    role: &'static str,
    origins: Vec<Origin<'a>>,
}
#[derive(Serialize)]
struct Scope<'a> {
    call: &'a crate::HirRef,
    call_pdg_node_id: &'a str,
    saved_actual_pdg_node_ids: [&'a str; 2],
    actuals: [&'a crate::HirRef; 2],
    dependency: &'a Value,
    callee_contract: &'a WireContract,
    condition_scope: &'static str,
}
#[derive(Serialize)]
struct Case<'a> {
    variant: &'static str,
    origins: Vec<Origin<'a>>,
}
#[derive(Serialize)]
struct ResultCases<'a> {
    role: &'static str,
    scope: &'a Scope<'a>,
    cases: [Case<'a>; 2],
}
#[derive(Serialize)]
struct Guard<'a> {
    scrutinee: &'a crate::HirRef,
    selector_pdg_node_id: &'a str,
    outcome: &'static str,
    variant: &'static str,
}
#[derive(Serialize)]
struct Error<'a> {
    role: &'static str,
    scope: &'a Scope<'a>,
    guard: Guard<'a>,
    origins: Vec<Origin<'a>>,
}

pub(super) fn derive(
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
    ops: &graph::Operations<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let mut roles = scalar_roles(ops)?;
    for (id, role) in reader_roles(facts, &checked.reader, &ops.reader, ops.inputs[0])? {
        if roles.insert(id, role).is_some() {
            return Err("Origin roles alias the same serialized operation".into());
        }
    }

    Ok(roles)
}

pub(in crate::data_documents) fn reader_roles(
    facts: &dyn EvidenceFacts,
    source: &source::Reader<'_>,
    ops: &graph::Reader<'_>,
    input: &PdgNode,
) -> Result<BTreeMap<String, Value>, String> {
    let mut roles = reference_roles(facts, source, ops, input)?;
    error_roles(&mut roles, source, ops)?;
    Ok(roles)
}

fn reference_roles(
    facts: &dyn EvidenceFacts,
    source: &source::Reader<'_>,
    ops: &graph::Reader<'_>,
    input: &PdgNode,
) -> Result<BTreeMap<String, Value>, String> {
    let mut roles = BTreeMap::new();

    let formal = Origin {
        root: Root::Formal {
            parameter: 0,
            binding: &facts.signature.params[0].pat.hir,
        },
        route: vec![Component::SharedReferent],
    };
    for node in [input, ops.slice] {
        insert(
            &mut roles,
            node,
            Carried {
                role: "formal_reference",
                origins: vec![formal.clone()],
            },
        )?;
    }
    let literal = Origin {
        root: Root::StaticLiteral {
            descriptor: &source.literal,
        },
        route: vec![Component::SharedReferent],
    };
    insert(
        &mut roles,
        ops.message,
        Carried {
            role: "static_literal",
            origins: vec![literal],
        },
    )?;
    Ok(roles)
}

fn error_roles(
    roles: &mut BTreeMap<String, Value>,
    source: &source::Reader<'_>,
    ops: &graph::Reader<'_>,
) -> Result<(), String> {
    let scope = call_scope(source, ops);
    let origin = Origin {
        root: Root::StaticLiteral {
            descriptor: &source.literal,
        },
        route: source::error_route(&source.contract)?,
    };

    insert(roles, ops.call, reader_cases(&scope, &origin))?;
    for node in [ops.error, ops.returned] {
        insert(
            roles,
            node,
            Error {
                role: "propagated_static_error",
                scope: &scope,
                guard: Guard {
                    scrutinee: &source.propagation.operand.hir,
                    selector_pdg_node_id: &ops.selector.id,
                    outcome: "true",
                    variant: "Err",
                },
                origins: vec![origin.clone()],
            },
        )?;
    }
    Ok(())
}

fn scalar_roles(ops: &graph::Operations<'_>) -> Result<BTreeMap<String, Value>, String> {
    let mut roles = BTreeMap::new();
    for node in ops
        .inputs
        .iter()
        .skip(1)
        .chain(&ops.bindings.components)
        .chain(&ops.bindings.declarations)
        .copied()
        .chain([
            ops.reader.selector,
            ops.reader.tuple,
            ops.assertion.read,
            ops.assertion.size,
            ops.assertion.call,
            ops.branch.compared[0],
            ops.branch.compared[1],
            ops.branch.comparison,
            ops.branch.predicate,
            ops.branch.arguments[0],
            ops.branch.arguments[1],
            ops.branch.call,
            ops.branch.failure,
            ops.branch.returned,
            ops.tail.read,
            ops.tail.success,
            ops.tail.returned,
        ])
    {
        insert(
            &mut roles,
            node,
            json!({"role":"scalar_or_explicit_empty_variant","origins":[]}),
        )?;
    }
    Ok(roles)
}

fn call_scope<'a>(source: &'a source::Reader<'_>, ops: &'a graph::Reader<'_>) -> Scope<'a> {
    Scope {
        call: &source.propagation.operand.hir,
        call_pdg_node_id: &ops.call.id,
        saved_actual_pdg_node_ids: [&ops.slice.id, &ops.message.id],
        actuals: [&source.slice.hir, &source.message.hir],
        dependency: &source.dependency,
        callee_contract: &source.contract,
        condition_scope: "exact-selected-callee-under-this-call/1",
    }
}

fn reader_cases<'a>(scope: &'a Scope<'a>, origin: &Origin<'a>) -> ResultCases<'a> {
    ResultCases {
        role: "guarded_reader_result",
        scope,
        cases: [
            Case {
                variant: "Ok",
                origins: vec![],
            },
            Case {
                variant: "Err",
                origins: vec![origin.clone()],
            },
        ],
    }
}
