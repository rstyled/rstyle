//! Removing a successful variant transfer must make its payload unreachable.
use super::*;
use crate::HirNode;

pub(super) fn verify(pair: PairView<'_>, selector: &PdgNode, load: &PdgNode) -> Result<(), String> {
    guarded(pair, selector, load, crate::CfgEdgeKind::True)
}

pub(super) fn guarded(
    pair: PairView<'_>,
    selector: &PdgNode,
    load: &PdgNode,
    outcome: crate::CfgEdgeKind,
) -> Result<(), String> {
    let map = crate::read_operation_map(&pair.pdg.extensions)?;
    let cfg = |id: &str| -> Result<String, String> {
        let candidates: Vec<_> = map.iter().filter(|entry| entry.pdg_node_id == id).collect();
        let [entry] = candidates.as_slice() else {
            return Err("Conditional payload requires unique operation mapping".into());
        };
        Ok(entry.cfg_node_id.clone())
    };
    let controller = cfg(&selector.id)?;
    let target = cfg(&load.id)?;
    if !pair
        .cfg
        .edges
        .iter()
        .any(|edge| edge.source == controller && edge.kind == outcome)
    {
        return Err("Variant selector lacks its successful payload branch".into());
    }

    let mut pending = vec![pair.cfg.entry_node_id.as_str()];
    let mut seen = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        if node == target {
            return Err("Payload is reachable without its successful variant guard".into());
        }
        pending.extend(
            pair.cfg
                .edges
                .iter()
                .filter(|edge| {
                    edge.source == node && !(edge.source == controller && edge.kind == outcome)
                })
                .map(|edge| edge.target.as_str()),
        );
    }
    Ok(())
}

pub(super) fn binding(pair: PairView<'_>, binding: &HirNode, load: &PdgNode) -> Result<(), String> {
    let declaration = super::serialized_operation(pair, &binding.span, |node| {
        node.category == NodeCategory::Declaration
    })?;
    let symbol = binding
        .binding
        .as_ref()
        .ok_or("Missing scalar payload binding")?;
    if declaration.attributes.symbol_ids != [symbol.clone()]
        || declaration.attributes.defines.len() != 1
        || declaration.attributes.writes.len() != 1
        || declaration.attributes.writes[0].symbol_id.as_ref() != Some(symbol)
        || declaration.attributes.writes[0].location_id.is_some()
        || !declaration.attributes.reads.is_empty()
        || declaration.attributes.write_form.as_deref() != Some("initialization")
    {
        return Err("Payload does not define its exact scalar binding".into());
    }
    super::verify_saved_operands(pair, declaration, &[load])
}
