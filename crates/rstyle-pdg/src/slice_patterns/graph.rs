//! Independent replay of serialized prefix operations, never producer output.
mod bindings;
pub(crate) mod flow;
mod join;
mod operations;

use super::semantics::{Checked, Region};
use crate::data_documents::{
    serialized_operation, typed_operation, value_operation, verify_saved_operands,
};
use crate::{CfgEdgeKind, HirNode, NodeCategory, PdgNode, producer::PairView};

const LENGTH: &str = "slice.prefix-length";
const BYTE: &str = "slice.prefix-byte";
const DIGIT: &str = "slice.prefix-digit";
const ELEMENT: &str = "slice.prefix-element";
const SUFFIX: &str = "slice.prefix-suffix";

/// Regions come from independently checked original source, not wire literals.
/// The caller still installs/verifies physical origins and memory hazard edges.
pub(crate) struct Replay<'a> {
    pub saved: &'a PdgNode,
    pub length: &'a PdgNode,
    pub selector_reads: Vec<(&'a PdgNode, Region)>,
    pub selectors: Vec<&'a PdgNode>,
    pub elements: Vec<(&'a PdgNode, Region)>,
    pub suffix: (&'a PdgNode, Region),
    pub none: &'a PdgNode,
}

pub(crate) fn verify<'a>(pair: PairView<'a>, checked: &Checked<'_>) -> Result<Replay<'a>, String> {
    let replay = operations::collect(pair, checked)?;
    operations::saved(pair, checked, replay.saved)?;
    operations::construct_spans(checked, &replay)?;
    let cfg = flow::Flow::new(pair)?;
    cfg.before(replay.saved, replay.length)?;
    selectors(pair, &cfg, &replay)?;
    bindings::verify(pair, checked, &cfg, &replay)?;
    join::verify(pair, checked, &replay)?;

    Ok(replay)
}

fn selectors(pair: PairView<'_>, cfg: &flow::Flow<'_>, replay: &Replay<'_>) -> Result<(), String> {
    verify_saved_operands(pair, replay.length, &[replay.saved])?;
    for (index, (read, _)) in replay.selector_reads.iter().enumerate() {
        let previous = if index == 0 {
            replay.length
        } else {
            replay.selectors[index - 1]
        };
        let selector = replay.selectors[index];
        cfg.outgoing(
            previous,
            &[(CfgEdgeKind::True, read), (CfgEdgeKind::False, replay.none)],
        )?;
        cfg.guarded(previous, read, CfgEdgeKind::True)?;
        cfg.outgoing(read, &[(CfgEdgeKind::Normal, selector)])?;
        verify_saved_operands(pair, read, &[replay.saved])?;
        verify_saved_operands(pair, selector, &[read])?;
    }
    let last = replay
        .selectors
        .last()
        .ok_or("Empty prefix selector chain")?;
    cfg.outgoing(
        last,
        &[
            (CfgEdgeKind::True, replay.elements[0].0),
            (CfgEdgeKind::False, replay.none),
        ],
    )?;

    let failures: Vec<_> = std::iter::once(replay.length)
        .chain(replay.selectors.iter().copied())
        .collect();
    cfg.incoming_false(replay.none, &failures)
}

pub(crate) fn guards(replay: &Replay<'_>, success: bool) -> Vec<Vec<crate::producer::Atom>> {
    let selectors: Vec<_> = std::iter::once(replay.length)
        .chain(replay.selectors.iter().copied())
        .collect();
    let atom = |node: &PdgNode, outcome: &str| crate::producer::Atom {
        controller_pdg_node_id: node.id.clone(),
        outcome: outcome.into(),
    };
    if success {
        return vec![selectors.iter().map(|node| atom(node, "true")).collect()];
    }

    selectors
        .iter()
        .enumerate()
        .map(|(index, node)| {
            selectors[..index]
                .iter()
                .map(|node| atom(node, "true"))
                .chain(std::iter::once(atom(node, "false")))
                .collect()
        })
        .collect()
}
