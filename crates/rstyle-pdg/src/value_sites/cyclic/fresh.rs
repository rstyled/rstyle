//! Temporary natural-loop kill sets derived from serialized CFG structure.
use super::*;

type Edge = (String, String);

#[derive(Default)]
pub(super) struct Resets(pub(super) BTreeMap<Edge, BTreeSet<String>>);

impl Resets {
    pub fn build(cfg: &CfgDocument) -> Result<Self, String> {
        let dominance = crate::cyclic_values::dominators(cfg);
        let edges: Vec<_> = cfg
            .edges
            .iter()
            .filter(|edge| {
                dominance.contains_key(&edge.source) && dominance.contains_key(&edge.target)
            })
            .map(|edge| (edge.source.clone(), edge.target.clone()))
            .collect();
        Self::derive(&dominance, &edges)
    }

    fn derive(
        dominance: &BTreeMap<String, BTreeSet<String>>,
        edges: &[Edge],
    ) -> Result<Self, String> {
        let backs: BTreeSet<_> = edges
            .iter()
            .filter(|(source, target)| dominance[source].contains(target))
            .cloned()
            .collect();
        check_reducible(dominance, edges, &backs)?;
        let mut regions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (source, header) in &backs {
            let region = natural_loop(source, header, edges);
            if region
                .iter()
                .any(|vertex| !dominance[vertex].contains(header))
            {
                return Err("Loop region has an entry outside its dominating header".into());
            }
            regions.entry(header.clone()).or_default().extend(region);
        }

        Ok(Self(
            backs
                .into_iter()
                .map(|edge| {
                    let region = regions[&edge.1].clone();
                    (edge, region)
                })
                .collect(),
        ))
    }

    pub fn region(&self, source: &str, target: &str) -> Option<&BTreeSet<String>> {
        if self.0.is_empty() {
            return None;
        }

        self.0.get(&(source.into(), target.into()))
    }

    pub fn clear_guards(
        &self,
        context: &Context<'_>,
        edge: &CfgEdge,
        outcomes: &mut BTreeMap<String, String>,
    ) {
        if let Some(region) = self.region(&edge.source, &edge.target) {
            outcomes.retain(|controller, _| {
                !region
                    .iter()
                    .any(|vertex| context.operation_at(vertex) == Some(controller.as_str()))
            });
        }
    }
}

fn natural_loop(source: &str, header: &str, edges: &[Edge]) -> BTreeSet<String> {
    let mut region = BTreeSet::from([header.to_string()]);
    let mut pending = vec![source.to_string()];
    while let Some(vertex) = pending.pop() {
        if region.insert(vertex.clone()) {
            pending.extend(
                edges
                    .iter()
                    .filter(|(_, target)| *target == vertex)
                    .map(|(source, _)| source.clone()),
            );
        }
    }
    region
}

fn check_reducible(
    dominance: &BTreeMap<String, BTreeSet<String>>,
    edges: &[Edge],
    backs: &BTreeSet<Edge>,
) -> Result<(), String> {
    let mut incoming = incoming_counts(dominance, edges, backs)?;
    let mut pending: BTreeSet<_> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(vertex, _)| vertex.clone())
        .collect();
    let mut visited = 0;
    while let Some(vertex) = pending.pop_first() {
        visited += 1;
        pending.extend(release_successors(&vertex, edges, backs, &mut incoming)?);
    }

    if visited != incoming.len() {
        return Err("Repeated value merges require reducible CFG loops".into());
    }
    Ok(())
}

fn incoming_counts(
    dominance: &BTreeMap<String, BTreeSet<String>>,
    edges: &[Edge],
    backs: &BTreeSet<Edge>,
) -> Result<BTreeMap<String, usize>, String> {
    let mut incoming: BTreeMap<_, usize> =
        dominance.keys().map(|vertex| (vertex.clone(), 0)).collect();
    for (_, target) in edges.iter().filter(|edge| !backs.contains(*edge)) {
        *incoming.get_mut(target).ok_or("Missing loop vertex")? += 1;
    }

    Ok(incoming)
}

fn release_successors(
    vertex: &str,
    edges: &[Edge],
    backs: &BTreeSet<Edge>,
    incoming: &mut BTreeMap<String, usize>,
) -> Result<Vec<String>, String> {
    edges
        .iter()
        .filter(|edge| edge.0 == vertex && !backs.contains(*edge))
        .map(|(_, target)| {
            let count = incoming.get_mut(target).ok_or("Missing loop successor")?;
            *count -= 1;
            Ok((*count == 0).then(|| target.clone()))
        })
        .filter_map(Result::transpose)
        .collect()
}

#[cfg(test)]
mod tests;
