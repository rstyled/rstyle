//! Payloads must be reachable, and require their successful variant transfer.
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
    Guards::new(pair)?.guarded(selector, load, outcome)
}

pub(super) struct Guards<'a> {
    pair: PairView<'a>,
    operations: BTreeMap<String, Vec<String>>,
    outgoing: BTreeMap<&'a str, Vec<&'a crate::CfgEdge>>,
    reachable: BTreeSet<&'a str>,
}

impl<'a> Guards<'a> {
    pub(super) fn new(pair: PairView<'a>) -> Result<Self, String> {
        let mut operations = BTreeMap::<_, Vec<_>>::new();
        for entry in crate::read_operation_map(&pair.pdg.extensions)? {
            operations
                .entry(entry.pdg_node_id)
                .or_default()
                .push(entry.cfg_node_id);
        }
        let mut outgoing = BTreeMap::<_, Vec<_>>::new();
        for edge in &pair.cfg.edges {
            outgoing.entry(edge.source.as_str()).or_default().push(edge);
        }

        let reachable = reachable(pair.cfg.entry_node_id.as_str(), &outgoing, None, None);
        Ok(Self {
            pair,
            operations,
            outgoing,
            reachable,
        })
    }

    fn operation(&self, id: &str) -> Result<&str, String> {
        match self.operations.get(id).map(Vec::as_slice) {
            Some([operation]) => Ok(operation),
            _ => Err("Conditional payload requires unique operation mapping".into()),
        }
    }

    pub(super) fn guarded(
        &self,
        selector: &PdgNode,
        load: &PdgNode,
        outcome: crate::CfgEdgeKind,
    ) -> Result<(), String> {
        let controller = self.operation(&selector.id)?;
        let target = self.operation(&load.id)?;
        if !self.reachable.contains(controller) || !self.reachable.contains(target) {
            return Err("Variant selector or guarded payload is unreachable".into());
        }
        if !self
            .outgoing
            .get(controller)
            .into_iter()
            .flatten()
            .any(|edge| edge.kind == outcome)
        {
            return Err("Variant selector lacks its successful payload branch".into());
        }

        if reachable(
            &self.pair.cfg.entry_node_id,
            &self.outgoing,
            Some((controller, outcome)),
            Some(target),
        )
        .contains(target)
        {
            return Err("Payload is reachable without its successful variant guard".into());
        }
        Ok(())
    }
}

fn reachable<'a>(
    entry: &'a str,
    outgoing: &BTreeMap<&'a str, Vec<&'a crate::CfgEdge>>,
    removed: Option<(&str, crate::CfgEdgeKind)>,
    stop: Option<&str>,
) -> BTreeSet<&'a str> {
    let mut pending = vec![entry];
    let mut seen = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if seen.insert(node) {
            if stop == Some(node) {
                break;
            }
            pending.extend(
                outgoing
                    .get(node)
                    .into_iter()
                    .flatten()
                    .filter(|edge| removed != Some((node, edge.kind)))
                    .map(|edge| edge.target.as_str()),
            );
        }
    }
    seen
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
