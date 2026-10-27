//! Exact checked-reader transfers preserve every precheck and branch completion.
use crate::{
    CfgEdgeKind, CfgExitKind, CfgNodeKind, producer::PairView, slice_patterns::graph::flow::Flow,
};

pub(super) fn verify(pair: PairView<'_>, ops: &super::Operations<'_>) -> Result<(), String> {
    let flow = Flow::new(pair)?;
    boundaries(pair, &flow, ops)?;

    for chain in [
        vec![
            ops.arguments[0],
            ops.arguments[1],
            ops.arguments[2],
            ops.length,
            ops.selector,
        ],
        vec![ops.error, ops.error_return],
        vec![
            ops.unit,
            ops.reader_argument,
            ops.reader,
            ops.consumed,
            ops.tuple,
            ops.success,
            ops.returned,
        ],
    ] {
        for adjacent in chain.windows(2) {
            flow.outgoing(adjacent[0], &[(CfgEdgeKind::Normal, adjacent[1])])?;
        }
    }
    // Ordinary helper panic evidence remains an explicit Partial limitation;
    // this lowering emits no exceptional helper transfer around either call.
    flow.outgoing(
        ops.selector,
        &[
            (CfgEdgeKind::True, ops.error),
            (CfgEdgeKind::False, ops.unit),
        ],
    )
}

fn boundaries(
    pair: PairView<'_>,
    flow: &Flow<'_>,
    ops: &super::Operations<'_>,
) -> Result<(), String> {
    let exits: Vec<_> = pair
        .cfg
        .nodes
        .iter()
        .filter(|node| node.kind == CfgNodeKind::Exit)
        .collect();
    let [exit] = exits.as_slice() else {
        return Err("Checked reader requires one declared normal exit".into());
    };
    if exit.exit_kind != Some(CfgExitKind::Normal)
        || pair.cfg.exit_node_ids.as_slice() != [exit.id.clone()]
    {
        return Err("Checked reader changes its complete normal exit inventory".into());
    }

    for (source, kind, target) in [
        (
            pair.cfg.entry_node_id.as_str(),
            CfgEdgeKind::Normal,
            flow.operation(ops.arguments[0])?,
        ),
        (
            flow.operation(ops.error_return)?,
            CfgEdgeKind::Return,
            exit.id.as_str(),
        ),
        (
            flow.operation(ops.returned)?,
            CfgEdgeKind::Return,
            exit.id.as_str(),
        ),
    ] {
        let actual: Vec<_> = pair
            .cfg
            .edges
            .iter()
            .filter(|edge| edge.source == source)
            .map(|edge| (edge.kind, edge.target.as_str()))
            .collect();
        if !crate::validate::unique_inventory(&actual, &[(kind, target)]) {
            return Err("Checked reader entry or return changes its required completion".into());
        }
    }
    Ok(())
}
