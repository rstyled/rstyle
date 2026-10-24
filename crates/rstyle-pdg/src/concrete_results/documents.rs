//! Independent Result construction inventory and exact saved payload operands.
use super::*;
use crate::{
    NodeCategory, PdgNode,
    producer::{Pair, PairView},
};

#[derive(serde::Serialize)]
struct ConstructionSemantics<'a> {
    rule: &'a str,
    variant: &'a str,
    payload_operand: u8,
    evaluation: &'a str,
}

pub(crate) fn semantics(construct: &str) -> serde_json::Value {
    let variant = match construct {
        "data.result-ok" => "ok",
        "data.result-err" => "err",
        _ => return serde_json::Value::Null,
    };

    serde_json::to_value(ConstructionSemantics {
        rule: "construct-core-result/1",
        variant,
        payload_operand: 0,
        evaluation: "payload-once-before-wrap/1",
    })
    .expect("Result construction semantics contain only strings and an integer")
}

pub(crate) fn install(pair: &mut Pair) {
    for node in &mut pair.pdg.nodes {
        if let Some(construct @ ("data.result-ok" | "data.result-err")) =
            node.attributes.construct.as_deref()
        {
            node.annotations.insert(KEY.into(), semantics(construct));
        }
    }
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let checked = constructors(facts)
        .into_iter()
        .map(|source| verify_constructor(source, facts).map(|checked| (source, checked)))
        .collect::<Result<Vec<_>, _>>()?;
    for construct in ["data.result-ok", "data.result-err"] {
        let expected = checked
            .iter()
            .filter(|(_, checked)| checked.construct == construct)
            .count();
        let actual = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some(construct))
            .count();
        if actual != expected {
            return Err("Concrete Result constructor inventory is incomplete or duplicated".into());
        }
    }
    for (source, checked) in checked {
        let token = crate::data_aggregates::token(source)?;
        let node = crate::data_documents::typed_operation(pair, source, checked.construct, token)?;
        effects(node, checked.construct)?;
        let payload = crate::data_documents::value_operation(pair, checked.payload)?;
        crate::data_documents::verify_saved_operands(pair, node, &[payload])?;
    }
    Ok(())
}

fn effects(node: &PdgNode, construct: &str) -> Result<(), String> {
    if node.category != NodeCategory::Computation
        || node.annotations.get(KEY) != Some(&semantics(construct))
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.literal.is_some()
        || node.attributes.operator.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Result construction invents a read, call, ABI tag or other effect".into());
    }
    Ok(())
}
