//! Every branch completes its required constructor or reader; extra exits cannot bypass it.
use super::*;
use crate::{CfgEdgeKind, slice_patterns::graph::flow::Flow};

pub(super) fn verify(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    use CfgEdgeKind::{False, Normal, True};

    let flow = Flow::new(pair)?;
    flow.outgoing(
        ops.predicate,
        &[(True, ops.message), (False, ops.reader_actuals[0])],
    )?;

    for chain in [
        vec![
            ops.slice,
            ops.length,
            ops.width,
            ops.condition,
            ops.predicate,
        ],
        vec![ops.message, ops.constructor, ops.error, ops.error_return],
        vec![
            ops.reader_actuals[0],
            ops.reader_actuals[1],
            ops.reader,
            ops.returned,
        ],
    ] {
        for adjacent in chain.windows(2) {
            flow.outgoing(adjacent[0], &[(Normal, adjacent[1])])?;
        }
    }
    // Current helper-call panic evidence is an explicit Partial limitation; it
    // grants no additional ordinary/return/exception edge around the saved call.
    entry(pair, flow.operation(ops.slice)?)
}

fn entry(pair: PairView<'_>, expected: &str) -> Result<(), String> {
    let actual = pair
        .cfg
        .edges
        .iter()
        .filter(|edge| edge.source == pair.cfg.entry_node_id)
        .map(|edge| (edge.kind, edge.target.as_str()));

    if !crate::validate::unique_inventory(
        &actual.collect::<Vec<_>>(),
        &[(CfgEdgeKind::Normal, expected)],
    ) {
        return Err("Checked ID reader entry can bypass its required length precheck".into());
    }

    Ok(())
}
