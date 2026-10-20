//! Removing the Some-true transfer must make its payload load unreachable.
use super::*;

pub(super) fn verify(pair: PairView<'_>, selector: &PdgNode, load: &PdgNode) -> Result<(), String> {
    let map = crate::read_operation_map(&pair.pdg.extensions)?;
    let cfg = |id: &str| -> Result<String, String> {
        let candidates: Vec<_> = map.iter().filter(|entry| entry.pdg_node_id == id).collect();
        let [entry] = candidates.as_slice() else {
            return Err("Checked Option requires unique operation mapping".into());
        };
        Ok(entry.cfg_node_id.clone())
    };
    let controller = cfg(&selector.id)?;
    let target = cfg(&load.id)?;
    if !pair
        .cfg
        .edges
        .iter()
        .any(|edge| edge.source == controller && edge.kind == crate::CfgEdgeKind::True)
    {
        return Err("Some selector lacks its successful payload branch".into());
    }

    let mut pending = vec![pair.cfg.entry_node_id.as_str()];
    let mut seen = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        if node == target {
            return Err("Option payload is reachable without a successful Some guard".into());
        }
        pending.extend(
            pair.cfg
                .edges
                .iter()
                .filter(|edge| {
                    edge.source == node
                        && !(edge.source == controller && edge.kind == crate::CfgEdgeKind::True)
                })
                .map(|edge| edge.target.as_str()),
        );
    }
    Ok(())
}
