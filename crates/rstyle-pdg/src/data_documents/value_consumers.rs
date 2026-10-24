//! Exact outgoing data-dependence inventory shared by value contracts.
use super::*;

pub(super) fn verify(
    pair: PairView<'_>,
    value: &PdgNode,
    expected: &[&PdgNode],
    kind: &str,
) -> Result<(), String> {
    let actual: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.source == value.id && edge.kind == crate::EdgeFamily::Data)
        .collect();
    if actual.len() != expected.len()
        || expected.iter().any(|node| {
            actual
                .iter()
                .filter(|edge| edge.target == node.id && edge.subkind == kind)
                .count()
                != 1
        })
    {
        return Err("Value has missing, duplicate, or unrelated data consumers".into());
    }
    Ok(())
}
