//! Order and result selection are checked from serialized CFG paths/value sites.

use super::*;
use serde_json::Value;

const SITES: &str = "compat.rust.hir-r0:value-sites";

pub(in crate::data_documents) fn verify(
    pair: PairView<'_>,
    checked: &option_callbacks::Arguments<'_>,
    environment: &PdgNode,
    selector: &PdgNode,
) -> Result<(), String> {
    let receiver = super::super::value_operation(pair, checked.receiver)?;
    let default = super::super::value_operation(pair, checked.default)?;
    before(pair, &completion(pair, default)?, environment)?;
    before(pair, &completion(pair, environment)?, selector)?;
    let receiver = completion(pair, receiver)?;

    for node in &pair.pdg.nodes {
        if node.category != NodeCategory::Merge
            && node.source_span.as_ref().is_some_and(|span| {
                span.start >= u64::from(checked.default.span.start)
                    && span.end <= u64::from(checked.default.span.end)
            })
        {
            before(pair, &receiver, node)?;
        }
    }
    Ok(())
}

pub(in crate::data_documents) struct ResultValues<'a> {
    pub call: &'a HirNode,
    pub body: &'a PdgNode,
    pub default: &'a HirNode,
}

pub(in crate::data_documents) fn result(
    pair: PairView<'_>,
    checked: ResultValues<'_>,
    selector: &PdgNode,
) -> Result<(), String> {
    let merge = super::super::value_operation(pair, checked.call)?;
    if merge.category != NodeCategory::Merge {
        return Err("Callback invocation lacks its conditional result merge".into());
    }
    super::effects(merge, NodeCategory::Merge)?;
    let site = site(pair, &merge.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing callback result alternatives")?;
    let body = checked.body;
    let default = super::super::value_operation(pair, checked.default)?;
    if site["controller_pdg_node_id"] != selector.id || alternatives.len() != 2 {
        return Err("Callback result loses its exact tag controller or alternatives".into());
    }

    for (outcome, expected) in [("true", body), ("false", default)] {
        if alternatives
            .iter()
            .filter(|alternative| {
                alternative["outcome"] == outcome && alternative["value_pdg_node_id"] == expected.id
            })
            .count()
            != 1
        {
            return Err(
                "Callback result must select body on Some and saved eager default on None".into(),
            );
        }
    }
    Ok(())
}

pub(in crate::data_documents) fn site<'a>(
    pair: PairView<'a>,
    id: &str,
) -> Result<&'a Value, String> {
    let candidates = pair
        .pdg
        .extensions
        .get(SITES)
        .and_then(Value::as_array)
        .ok_or("Missing callback value sites")?;
    let mut candidates = candidates.iter().filter(|site| site["pdg_node_id"] == id);
    let site = candidates.next().ok_or("Missing callback value site")?;
    if candidates.next().is_some() {
        return Err("Duplicate callback value site".into());
    }

    Ok(site)
}

fn operation(pair: PairView<'_>, node: &PdgNode) -> Result<String, String> {
    let map = crate::read_operation_map(&pair.pdg.extensions)?;
    let mut entries = map.iter().filter(|entry| entry.pdg_node_id == node.id);
    let found = entries
        .next()
        .ok_or("Callback order lacks a real CFG operation")?;
    if entries.next().is_some() {
        return Err("Callback order has ambiguous CFG operations".into());
    }

    Ok(found.cfg_node_id.clone())
}

pub(in crate::data_documents) fn completion(
    pair: PairView<'_>,
    node: &PdgNode,
) -> Result<BTreeSet<String>, String> {
    if node.category != NodeCategory::Merge {
        return Ok(BTreeSet::from([operation(pair, node)?]));
    }
    let site = site(pair, &node.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing eager argument completions")?;
    let mut completed = BTreeSet::new();

    for alternative in alternatives {
        for id in alternative["completion_cfg_node_ids"]
            .as_array()
            .ok_or("Missing eager completion frontier")?
        {
            completed.insert(
                id.as_str()
                    .ok_or("Invalid eager completion frontier")?
                    .to_owned(),
            );
        }
    }
    if completed.is_empty() {
        return Err("Empty eager argument completion frontier".into());
    }
    Ok(completed)
}

pub(in crate::data_documents) fn before(
    pair: PairView<'_>,
    earlier: &BTreeSet<String>,
    later: &PdgNode,
) -> Result<(), String> {
    let target = operation(pair, later)?;
    let mut pending = vec![pair.cfg.entry_node_id.as_str()];
    let mut seen = BTreeSet::new();

    while let Some(node) = pending.pop() {
        if earlier.contains(node) || !seen.insert(node) {
            continue;
        }
        if node == target {
            return Err("Callback argument/environment order can be bypassed".into());
        }
        pending.extend(
            pair.cfg
                .edges
                .iter()
                .filter(|edge| edge.source == node)
                .map(|edge| edge.target.as_str()),
        );
    }
    Ok(())
}
