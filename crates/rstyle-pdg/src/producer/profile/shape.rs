//! Preserve fact checks previously performed by the whole-document deserializer.
use crate::{StringFact, model::FactShape, producer::Pair};

pub(super) fn check(pair: &Pair) -> Result<(), String> {
    pdg(&pair.pdg)?;
    string(&pair.cfg.function.name)?;
    string(&pair.cfg.producer.language_version)?;
    pair.cfg
        .sources
        .iter()
        .try_for_each(|source| string(&source.content_identity))
}

fn pdg(doc: &crate::PdgDocument) -> Result<(), String> {
    interface(&doc.function)?;
    doc.nodes.iter().try_for_each(node)?;

    string(&doc.producer.language_version)?;
    doc.sources
        .iter()
        .try_for_each(|source| string(&source.content_identity))?;
    doc.symbols.iter().try_for_each(|symbol| {
        string(&symbol.name)?;
        string(&symbol.type_fact)
    })
}

fn interface(function: &crate::FunctionInfo) -> Result<(), String> {
    function.inputs.iter().try_for_each(parameter)?;
    string(&function.name)?;
    function.outputs.iter().try_for_each(parameter)?;
    string(&function.qualified_name)?;
    let count = &function.reference_count;

    FactShape::new(count.status, count.value.is_some(), count.reason.as_deref()).check("count")
}

fn parameter(parameter: &crate::Parameter) -> Result<(), String> {
    string(&parameter.name)?;
    string(&parameter.type_fact)?;
    let value = &parameter.variadic;

    FactShape::new(value.status, value.value.is_some(), value.reason.as_deref()).check("boolean")
}

fn node(node: &crate::PdgNode) -> Result<(), String> {
    for value in [
        &node.attributes.callee,
        &node.attributes.literal,
        &node.attributes.operator,
    ]
    .into_iter()
    .flatten()
    {
        string(value)?;
    }
    string(&node.original_kind)
}

fn string(value: &StringFact) -> Result<(), String> {
    FactShape::new(value.status, value.value.is_some(), value.reason.as_deref()).check("string")
}
