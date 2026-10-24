//! Independent bit widths, rounding sites and ordered saved float operands.
use super::*;
use crate::{HirNode, float_scalars, unit_store::EvidenceFacts};

pub(super) fn sources(facts: &dyn EvidenceFacts) -> Vec<&HirNode> {
    crate::field_locations::operations(
        &facts.body,
        &[
            "expr_lit",
            "expr_path",
            "expr_binary",
            "expr_unary",
            "expr_cast",
        ],
    )
    .into_iter()
    .filter(|node| float_scalars::computation(node))
    .collect()
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    float_scalars::verify_document_policy(&pair.pdg.profile, facts)?;
    let sources = sources(facts);
    let expected: BTreeSet<_> = sources
        .iter()
        .map(|source| super::value_operation(pair, source).map(|node| node.id.as_str()))
        .collect::<Result<_, _>>()?;

    let actual: BTreeSet<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.annotations.contains_key(float_scalars::KEY)
                || node
                    .attributes
                    .construct
                    .as_deref()
                    .is_some_and(|construct| {
                        [
                            float_scalars::LITERAL,
                            float_scalars::CONSTANT,
                            float_scalars::BINARY,
                            float_scalars::NEGATE,
                            float_scalars::NARROW,
                        ]
                        .contains(&construct)
                    })
        })
        .map(|node| node.id.as_str())
        .collect();

    if actual != expected {
        return Err("Float computation inventory disagrees with complete original source".into());
    }
    for source in sources {
        verify_operation(pair, source, facts)?;
    }
    Ok(())
}

fn verify_operation(
    pair: PairView<'_>,
    source: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let node = super::value_operation(pair, source)?;
    float_scalars::verify_node(source, facts)?;
    if node.category != NodeCategory::Computation
        || node.original_kind.known_value() != Some(source.kind.as_str())
        || node.attributes.construct.as_deref() != float_scalars::construct(source)
        || node.annotations.get(float_scalars::KEY)
            != Some(&float_scalars::semantics(source, facts)?)
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(|operator| operator.known_value())
            != source
                .dispatch
                .as_ref()
                .map(|dispatch| dispatch.op.as_str())
    {
        return Err(
            "Float operation loses its exact width, operator or rounding interpretation".into(),
        );
    }

    effects(node)?;
    literal(node, source, facts)?;
    let operands = source
        .children
        .iter()
        .map(|child| super::value_operation(pair, child))
        .collect::<Result<Vec<_>, _>>()?;
    super::verify_saved_operands(pair, node, &operands)
}

fn effects(node: &PdgNode) -> Result<(), String> {
    if node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || !node.attributes.outcomes.is_empty()
        || node.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Float computation invents memory, call, binding or control effects".into());
    }
    Ok(())
}

fn literal(node: &PdgNode, source: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let expected = match source.kind.as_str() {
        "expr_lit" => Some(float_scalars::literal(source, facts)?.bits),
        "expr_path" => Some(float_scalars::constant(source)?.bits),
        _ => None,
    };
    if node
        .attributes
        .literal
        .as_ref()
        .and_then(|value| value.known_value())
        != expected.as_deref()
    {
        return Err("Float computation loses canonical exact bits or invents a literal".into());
    }
    Ok(())
}
