//! Keep static routes inside their exact branch and selected callee scope.
use super::*;
use crate::copy_tagged_values::origins::{CaseKind, Component, WireContract};
use crate::data_documents::guarded_sources::insert_role as insert;
use crate::structural_json::object;
use serde_json::json;

pub(super) fn derive(
    checked: &source::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let mut roles = BTreeMap::new();
    precheck_roles(&mut roles, ops)?;
    short_error(&mut roles, checked, ops)?;
    reader(&mut roles, checked, ops)?;

    Ok(roles)
}

fn precheck_roles(roles: &mut BTreeMap<String, Value>, ops: &Operations<'_>) -> Result<(), String> {
    for node in [ops.length, ops.width, ops.condition, ops.predicate] {
        insert(
            roles,
            node,
            object([("role", json!("scalar-precheck")), ("origins", json!([]))]),
        )?;
    }

    for (input, nodes) in [
        (
            ops.inputs[0],
            vec![ops.inputs[0], ops.slice, ops.reader_actuals[0]],
        ),
        (
            ops.inputs[1],
            vec![ops.inputs[1], ops.message, ops.reader_actuals[1]],
        ),
    ] {
        for node in nodes {
            insert(
                roles,
                node,
                object([
                    ("role", json!("formal-reference")),
                    ("root", json!(input.id)),
                    ("route", json!([Component::SharedReferent])),
                ]),
            )?;
        }
    }

    Ok(())
}

fn short_error(
    roles: &mut BTreeMap<String, Value>,
    checked: &source::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    let route = error_route(&checked.error_contract, CaseKind::Whole, 0)?;

    for (node, result) in [
        (ops.constructor, false),
        (ops.error, true),
        (ops.error_return, true),
    ] {
        let output = if result {
            std::iter::once(Component::ResultErr)
                .chain(route.clone())
                .collect()
        } else {
            route.clone()
        };
        insert(
            roles,
            node,
            object([
                ("role", json!("short-input-static-error")),
                ("root", json!(ops.inputs[1].id)),
                (
                    "guard",
                    json!({"selector":ops.predicate.id,"outcome":"true"}),
                ),
                ("callee", json!(checked.constructor.hir)),
                ("dependency", checked.error_dependency.clone()),
                ("contract", json!(checked.error_contract)),
                ("actual", json!(ops.message.id)),
                ("route", json!(output)),
            ]),
        )?;
    }

    Ok(())
}

fn reader(
    roles: &mut BTreeMap<String, Value>,
    checked: &source::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    let route = error_route(&checked.reader_contract, CaseKind::ResultErr, 1)?;

    for node in [ops.reader, ops.returned] {
        insert(
            roles,
            node,
            object([
                ("role", json!("guarded-selected-id-reader")),
                (
                    "guard",
                    json!({"selector":ops.predicate.id,"outcome":"false"}),
                ),
                ("callee", json!(checked.reader.hir)),
                ("dependency", checked.reader_dependency.clone()),
                ("contract", json!(checked.reader_contract)),
                ("actuals", json!(ops.reader_actuals.map(|node| &node.id))),
                (
                    "condition_scope",
                    json!("exact-selected-callee-under-this-call/1"),
                ),
                ("ok_origins", json!([])),
                (
                    "error_origins",
                    Value::Array(vec![object([
                        ("root", json!(ops.inputs[1].id)),
                        ("route", json!(route)),
                    ])]),
                ),
            ]),
        )?;
    }

    Ok(())
}

fn error_route(
    contract: &WireContract,
    output: CaseKind,
    parameter: usize,
) -> Result<Vec<Component>, String> {
    let cases: Vec<_> = contract
        .alternatives
        .iter()
        .filter(|alternative| alternative.output == output)
        .collect();

    let [alternative] = cases.as_slice() else {
        return Err("Checked ID reader requires its single exact static error alternative".into());
    };

    let [leaf] = alternative.leaves.as_slice() else {
        return Err("Checked ID reader requires exactly one carried static message".into());
    };

    if leaf.root.parameter != parameter
        || leaf.input != [Component::SharedReferent]
        || leaf.output.last() != Some(&Component::SharedReferent)
        || leaf.guard.is_some()
    {
        return Err("Checked ID reader changes its exact static-message route".into());
    }

    Ok(leaf.output.clone())
}
