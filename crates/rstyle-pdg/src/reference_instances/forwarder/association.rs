//! Selected outer From implementation and inner call must share the checked owner.
use super::{Result, source};
use crate::reference_instances::body::model::{Argument, Definition, Observation};
use crate::{HirFnFacts, HirNode};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Association {
    identity: String,
    complete: bool,
    method: Definition,
    implementation: Definition,
    trait_definition: Value,
    trait_method: Value,
    original_trait_arguments: Vec<Value>,
    substituted_trait_arguments: Vec<Argument>,
    impl_arguments: Vec<Argument>,
    selected: Definition,
    selected_arguments: Vec<Argument>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    hir: crate::HirRef,
    requested: Definition,
    requested_arguments: Vec<Argument>,
    selected: Definition,
    selected_arguments: Vec<Argument>,
}

pub(super) fn verify(
    canonical: &HirFnFacts,
    record: &Observation,
    constructor: &Observation,
    call: &HirNode,
) -> Result<()> {
    let raw = record
        .outer_trait_association
        .as_ref()
        .ok_or("Legacy reference observation lacks the outer From association proof")?;
    let association = Association::deserialize(raw).map_err(|error| error.to_string())?;
    owner(&association, record)?;
    arguments(&association, record)?;

    inner(record, constructor, call)?;
    crate::reference_instances::operations::authority::bound(
        canonical,
        &association.trait_definition,
        "core::convert::From",
    )?;
    crate::reference_instances::operations::authority::bound(
        canonical,
        &association.trait_method,
        "core::convert::From::from",
    )
}

fn owner(association: &Association, record: &Observation) -> Result<()> {
    let original = &record.provenance;
    let parent: Definition = serde_json::from_value(original["owners"][0]["definition"].clone())
        .map_err(|error| error.to_string())?;
    if association.identity != "rstyle.rust.reference-from-association/1"
        || !association.complete
        || association.method != record.definition
        || association.selected != record.definition
        || association.implementation != parent
        || !same(&association.impl_arguments, &record.arguments)
        || !same(&association.selected_arguments, &record.arguments)
    {
        return Err(
            "Outer From selection changes its checked original method/impl/substitution".into(),
        );
    }
    Ok(())
}

fn arguments(association: &Association, record: &Observation) -> Result<()> {
    if association.original_trait_arguments.len() != 2
        || !symbolic(
            &association.original_trait_arguments[0],
            &record.provenance["signature"]["output"],
        )
        || !symbolic(
            &association.original_trait_arguments[1],
            &record.provenance["signature"]["inputs"][0],
        )
        || association.substituted_trait_arguments.len() != 2
        || !source::equal(
            &association.substituted_trait_arguments[0],
            &record.projected_result[0],
        )
    {
        return Err(
            "Outer From association loses its original symbolic self/input relation".into(),
        );
    }
    let formal = &record.nodes[0].ty[0];
    if !source::equal(&association.substituted_trait_arguments[1], formal) {
        return Err("Outer From trait input differs from the exact projected formal".into());
    }
    Ok(())
}

fn symbolic(argument: &Value, expected: &Value) -> bool {
    argument.as_object().is_some_and(|object| {
        object.len() == 2
            && object.get("kind").and_then(Value::as_str) == Some("type")
            && object.get("value") == Some(expected)
    })
}

fn same(left: &[Argument], right: &[Argument]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| source::equal(a, b))
}

fn inner(record: &Observation, constructor: &Observation, node: &HirNode) -> Result<()> {
    let call = Call::deserialize(&record.calls[0]).map_err(|error| error.to_string())?;
    if call.hir != node.hir
        || call.requested != constructor.definition
        || call.selected != constructor.definition
        || !same(&call.requested_arguments, &constructor.arguments)
        || !same(&call.selected_arguments, &constructor.arguments)
    {
        return Err(
            "From forwarder does not select its exact checked constructor body/instance".into(),
        );
    }
    Ok(())
}
