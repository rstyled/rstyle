//! Cut actual true CFG edges; metadata cannot establish dominance.
use super::*;
use crate::scalar_product_instances::activation::Checked;

pub(super) struct Guards<'a> {
    selectors: Vec<&'a str>,
    outgoing: BTreeMap<&'a str, Vec<(&'a str, &'a str)>>,
}

impl<'a> Guards<'a> {
    pub(super) fn new(graph: &'a Graph, checked: &Checked<'_>) -> Result<Self, String> {
        let prefix = checked.prefix();
        let saved = source_operation(graph, prefix.scrutinee())?;
        let mut selectors = Vec::new();
        for (index, selector) in prefix.contract().selectors.iter().enumerate() {
            let source = if index == 0 {
                prefix.prefix().pattern()
            } else {
                prefix.digits()[index - 1].range()
            };
            let construct = if index == 0 {
                "slice.prefix-length"
            } else {
                "slice.prefix-digit"
            };
            let node = operation(graph, source, construct)?;
            if node.category != NodeCategory::Control
                || node.ty != "bool"
                || node.literal.as_ref()
                    != Some(
                        &serde_json::to_string(&selector.kind)
                            .map_err(|error| error.to_string())?,
                    )
            {
                return Err("Decimal guard differs from original prefix predicate".into());
            }
            let input = if index == 0 {
                saved
            } else {
                operation(graph, source, "slice.prefix-byte")?
            };
            if node.operands != [input.id.clone()]
                || index != 0
                    && (input.category != NodeCategory::Read
                        || input.ty != "u8"
                        || input.operands != [saved.id.clone()])
            {
                return Err("Decimal guard loses its distinct saved-input byte load".into());
            }
            selectors.push(node.id.as_str());
        }

        Ok(Self {
            selectors,
            outgoing: outgoing(graph),
        })
    }

    pub(super) fn success(&self, node: &Operation, analysis: &Analysis) -> Result<(), String> {
        if !analysis.reachable.contains(&node.id)
            || self
                .selectors
                .iter()
                .any(|selector| self.reaches(&node.id, selector))
        {
            return Err("Decimal scalar operation bypasses an original digit true guard".into());
        }
        Ok(())
    }

    fn reaches(&self, target: &str, removed: &str) -> bool {
        let mut pending = vec!["E"];
        let mut seen = BTreeSet::new();
        while let Some(node) = pending.pop() {
            if !seen.insert(node) {
                continue;
            }
            if node == target {
                return true;
            }
            if let Some(edges) = self.outgoing.get(node) {
                pending.extend(
                    edges
                        .iter()
                        .filter(|(_, kind)| node != removed || *kind != "true")
                        .map(|(target, _)| *target),
                );
            }
        }
        false
    }
}

fn outgoing(graph: &Graph) -> BTreeMap<&str, Vec<(&str, &str)>> {
    let mut outgoing: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for edge in &graph.transfers {
        outgoing
            .entry(&edge.source)
            .or_default()
            .push((&edge.target, &edge.kind));
    }
    outgoing
}

fn operation<'a>(
    graph: &'a Graph,
    source: &HirNode,
    construct: &str,
) -> Result<&'a Operation, String> {
    let mut nodes = graph.nodes.iter().filter(|node| {
        node.span == source.span
            && node.original_kind == source.kind
            && node.construct.as_deref() == Some(construct)
    });
    let node = nodes
        .next()
        .ok_or("Missing original decimal prefix guard operation")?;
    if nodes.next().is_some() {
        return Err("Ambiguous original decimal prefix guard operation".into());
    }
    Ok(node)
}

pub(super) fn binding(
    graph: &Graph,
    analysis: &Analysis,
    digit: &crate::slice_patterns::semantics::Digit<'_>,
    read: &Operation,
) -> Result<(), String> {
    let pointer = source_operation(graph, digit.pointer())?;
    let symbol = digit
        .binding()
        .binding
        .as_ref()
        .ok_or("Missing original digit binder symbol")?;
    let element = operation(graph, digit.binding(), "slice.prefix-element")?;
    let mut declarations = graph.nodes.iter().filter(|node| {
        node.span == digit.binding().span && node.category == NodeCategory::Declaration
    });
    let declaration = declarations
        .next()
        .ok_or("Missing guarded original digit binding")?;
    let definitions = analysis
        .definition_in
        .get(&pointer.id)
        .and_then(|incoming| incoming.get(symbol))
        .ok_or("Missing original digit pointer reaching definition")?;
    if declarations.next().is_some()
        || declaration.symbol.as_ref() != Some(symbol)
        || declaration.write_form.as_deref() != Some("initialization")
        || declaration.operands != [element.id.clone()]
        || declaration
            .definition
            .as_ref()
            .is_none_or(|definition| definitions.len() != 1 || !definitions.contains(definition))
        || pointer.category != NodeCategory::Read
        || pointer.symbol.as_ref() != Some(symbol)
        || read.operands != [pointer.id.clone()]
    {
        return Err("Decimal load loses its exact guarded borrowed digit definition".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guards(edges: &[(&'static str, &'static str, &'static str)]) -> Guards<'static> {
        let mut outgoing: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for (source, target, kind) in edges {
            outgoing.entry(source).or_default().push((target, kind));
        }
        Guards {
            selectors: vec!["digit"],
            outgoing,
        }
    }

    #[test]
    fn mismatch_and_bypass_edges_cannot_prove_decimal_bytes() {
        let checked = guards(&[
            ("E", "digit", "normal"),
            ("digit", "read", "true"),
            ("digit", "none", "false"),
        ]);
        assert!(!checked.reaches("read", "digit"));
        assert!(checked.reaches("none", "digit"));
        let bypass = guards(&[
            ("E", "digit", "normal"),
            ("digit", "read", "true"),
            ("digit", "other", "false"),
            ("other", "read", "normal"),
        ]);
        assert!(bypass.reaches("read", "digit"));
    }

    #[test]
    fn cycles_and_missing_guards_remain_reachable_after_cut() {
        let cyclic = guards(&[
            ("E", "read", "normal"),
            ("read", "digit", "normal"),
            ("digit", "read", "true"),
        ]);
        assert!(cyclic.reaches("read", "digit"));
        assert!(cyclic.reaches("read", "missing"));
    }
}
