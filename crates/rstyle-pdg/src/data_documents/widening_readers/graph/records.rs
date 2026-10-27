//! Immutable source expectations retain independent origin cases per operation.
use super::*;
use crate::copy_tagged_values::origins::Component;
use serde_json::json;

use crate::structural_json::object;

pub(super) fn derive(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let [short, converted_error] = origins::routes(checked)?;
    let values = ops.nodes.iter().map(|(name, node)| {
        record(name, facts, checked, [&short, &converted_error])
            .map(|record| (node.id.clone(), record))
    });
    let bindings = ops
        .bindings
        .into_iter()
        .map(|node| Ok((node.id.clone(), empty("immutable-scalar-callback-binding"))));
    let inputs = ops.inputs.into_iter().enumerate().map(|(parameter, node)| {
        Ok((
            node.id.clone(),
            object([
                ("role", json!("formal-shared-root")),
                ("parameter", json!(parameter)),
                ("binding", json!(origins::root(facts, parameter).binding)),
            ]),
        ))
    });
    let entries = values
        .chain(bindings)
        .chain(inputs)
        .collect::<Result<Vec<_>, String>>()?;

    let count = entries.len();
    let roles: BTreeMap<_, _> = entries.into_iter().collect();
    if roles.len() != count {
        return Err("Widening origin roles alias the same serialized operation".into());
    }
    crate::data_documents::guarded_sources::complete_roles(
        pair,
        roles,
        "Widening reader contains omitted, duplicate or unaccounted operations",
    )
}

fn record(
    name: &str,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    routes: [&[Component]; 2],
) -> Result<Value, String> {
    let [short, converted_error] = routes;
    match name {
        "slice" => Ok(reference("saved-byte-input", facts, 0)),
        "message" | "message-capture" => Ok(reference("saved-static-message", facts, 1)),
        "outer-environment" | "failure-environment" => Ok(environment(name, facts, checked)),
        "map-environment" => Ok(count_environment(checked)),
        "reader" => Ok(reader(facts, checked, short)),
        "reader-error-forward" => Ok(short_error(facts, checked, short)),
        "constructor" | "final-error" => Ok(error(facts, checked, converted_error)),
        "failure-merge" => Ok(conversion_cases(facts, checked, converted_error)),
        "outer-merge" | "returned" => result(facts, checked, converted_error),
        _ => Ok(empty(name)),
    }
}

fn empty(role: &str) -> Value {
    object([("role", json!(role)), ("origins", json!([]))])
}

fn reference(role: &str, facts: &dyn EvidenceFacts, parameter: usize) -> Value {
    let origins = json!([origins::reference(
        facts,
        parameter,
        &[Component::SharedReferent]
    )]);
    if parameter == 1 {
        return object([
            ("role", json!(role)),
            ("parameter", json!(parameter)),
            ("origins", origins),
        ]);
    }
    object([("role", json!(role)), ("origins", origins)])
}

fn environment(name: &str, facts: &dyn EvidenceFacts, checked: &contract::Checked<'_>) -> Value {
    let index = if name == "outer-environment" { 0 } else { 2 };
    let component = object([
        ("tuple_field", json!(0)),
        ("capture_mode", json!("shared")),
        ("captured_component", json!([Component::SharedReferent])),
    ]);
    object([
        ("role", json!("whole-shared-static-capture")),
        ("storage", json!(checked.proof.storage[index])),
        ("storage_component", component),
        ("capture_root", json!(origins::root(facts, 1))),
        (
            "origins",
            json!([origins::reference(facts, 1, &[Component::SharedReferent])]),
        ),
    ])
}

fn count_environment(checked: &contract::Checked<'_>) -> Value {
    object([
        ("role", json!("whole-shared-saved-count-capture")),
        ("storage", json!(checked.proof.storage[1])),
        (
            "storage_component",
            object([("tuple_field", json!(0)), ("capture_mode", json!("shared"))]),
        ),
        ("binding", json!(checked.roles.count.hir)),
        ("value_origins", json!([])),
        (
            "storage_permission",
            json!("initialized-nonescaping-callback-binding"),
        ),
    ])
}

fn reader(
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    short: &[Component],
) -> Value {
    object([
        ("role", json!("selected-reader-cases")),
        ("call", json!(checked.roles.reader.hir)),
        ("dependency", json!(checked.proof.reader_dependency)),
        ("callee_contract", json!(checked.proof.reader_contract)),
        ("condition_scope", json!("selected-callee-only")),
        ("ok_origins", json!([])),
        (
            "short_error_origins",
            json!([origins::reference(facts, 1, short)]),
        ),
    ])
}

fn short_error(
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    short: &[Component],
) -> Value {
    object([
        ("role", json!("guarded-short-error-forward")),
        ("reader", json!(checked.roles.reader.hir)),
        ("reader_guard", json!("err")),
        ("origins", json!([origins::reference(facts, 1, short)])),
    ])
}

fn error(facts: &dyn EvidenceFacts, checked: &contract::Checked<'_>, route: &[Component]) -> Value {
    object([
        ("role", json!("guarded-conversion-error")),
        ("dependency", json!(checked.proof.error_dependency)),
        ("callee_contract", json!(checked.proof.error_contract)),
        ("reader_guard", json!("ok")),
        ("conversion_guard", json!("err")),
        (
            "reachability",
            json!(
                if checked.roles.width == crate::native_reader_width::Width::U16 {
                    "excluded-by-proved-never-error-conversion"
                } else {
                    "excluded-by-pinned-total-u32-conversion"
                }
            ),
        ),
        ("origins", json!([origins::reference(facts, 1, route)])),
    ])
}

fn conversion_cases(
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    route: &[Component],
) -> Value {
    object([
        ("role", json!("guarded-conversion-result-cases")),
        ("reader_guard", json!("ok")),
        ("conversion", json!(checked.roles.conversion.hir)),
        ("ok_origins", json!([])),
        (
            "conversion_error_origins",
            json!([origins::reference(facts, 1, route)]),
        ),
        (
            "conversion_failure",
            json!("checked-unreachable-on-reviewed-target"),
        ),
    ])
}

fn result(
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
    route: &[Component],
) -> Result<Value, String> {
    Ok(object([
        ("role", json!("complete-guarded-result")),
        ("contract", json!(origins::contract(facts, checked)?)),
        ("conversion_error_route", json!(route)),
        (
            "conversion_failure",
            json!("checked-unreachable-on-reviewed-target"),
        ),
    ]))
}
