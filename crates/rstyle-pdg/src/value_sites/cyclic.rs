//! Finite serialized-CFG proof for conditional values whose controllers cannot repeat.
use super::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct State {
    vertex: String,
    outcomes: BTreeMap<String, String>,
}

struct Flow {
    states: Vec<State>,
    edges: BTreeSet<(usize, usize)>,
    before: Vec<BTreeSet<String>>,
}

pub(crate) fn validate(pdg: &PdgDocument, cfg: &CfgDocument) -> Result<BTreeSet<String>, String> {
    let value = pdg
        .extensions
        .get(SITES)
        .ok_or("Missing cyclic value sites")?;
    if cfg.extensions.get(SITES) != Some(value) {
        return Err("Paired cyclic value sites disagree".into());
    }
    let sites: Vec<Site> =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    let context = Context {
        pdg,
        cfg,
        sites,
        paths: Vec::new(),
        operations: read_operation_map(&pdg.extensions)?
            .into_iter()
            .map(|entry| (entry.cfg_node_id, entry.pdg_node_id))
            .collect(),
    };

    for site in &context.sites {
        check_nonrepeating(&context, site)?;
    }
    let flow = Flow::build(&context)?;
    let mut available = BTreeSet::new();
    for site in &context.sites {
        if flow.check_site(&context, site)? {
            available.insert(site.pdg_node_id.clone());
        }
    }
    let ids: BTreeSet<_> = context
        .sites
        .iter()
        .map(|site| site.pdg_node_id.as_str())
        .collect();
    let merges: BTreeSet<_> = pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Merge)
        .map(|node| node.id.as_str())
        .collect();
    if ids.len() != context.sites.len() || ids != merges {
        return Err("Cyclic sites must cover every merge exactly once".into());
    }

    flow.check_consumers(&context)?;
    Ok(available)
}

impl Flow {
    fn build(context: &Context<'_>) -> Result<Self, String> {
        let relevant = relevant_controllers(context);
        let first = State {
            vertex: context.cfg.entry_node_id.clone(),
            outcomes: BTreeMap::new(),
        };
        let mut network = Network {
            index: BTreeMap::from([(first.clone(), 0)]),
            states: vec![first],
            edges: BTreeSet::new(),
        };
        let mut next = 0;
        while next < network.states.len() {
            network.extend(context, next, &relevant)?;
            next += 1;
        }
        let Network { states, edges, .. } = network;

        let before = must_evaluate(&states, &edges);
        Ok(Self {
            states,
            edges,
            before,
        })
    }

    fn availability<'a>(
        &'a self,
        index: usize,
        before: &'a BTreeSet<&'a String>,
    ) -> Availability<'a> {
        Availability {
            at: Some(&self.states[index].vertex),
            outcomes: &self.states[index].outcomes,
            before,
            visiting: BTreeSet::new(),
        }
    }

    fn check_site(&self, context: &Context<'_>, site: &Site) -> Result<bool, String> {
        context.check_site_metadata(site)?;
        context.check_guard(&site.annotations.guards)?;
        if context.node(&site.pdg_node_id)?.annotations.get(GUARDS)
            != Some(
                &serde_json::to_value(&site.annotations.guards)
                    .map_err(|error| error.to_string())?,
            )
        {
            return Err("Cyclic merge/site guard disagreement".into());
        }

        self.check_alternatives(context, site)?;
        let mut available = false;
        for (index, _) in self
            .states
            .iter()
            .enumerate()
            .filter(|(_, state)| state.vertex == site.cfg_successor_node_id)
        {
            let before = self.before[index].iter().collect();
            let proof = self.availability(index, &before);
            context.check_site_availability(site, &proof)?;
            available |= context.satisfies_before(&site.annotations.guards, &proof);
        }
        Ok(available)
    }

    fn check_alternatives(&self, context: &Context<'_>, site: &Site) -> Result<(), String> {
        let outcomes: BTreeSet<_> = site
            .alternatives
            .iter()
            .map(|alternative| &alternative.outcome)
            .collect();
        if outcomes.len() != site.alternatives.len() || outcomes.len() < 2 {
            return Err("Cyclic merge needs distinct alternatives".into());
        }
        let count = context
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.target == site.pdg_node_id && edge.subkind == "value_alternative")
            .count();
        let live = self
            .states
            .iter()
            .any(|state| state.vertex == site.cfg_successor_node_id);
        if count != if live { site.alternatives.len() } else { 0 } {
            return Err("Cyclic alternative edge count disagrees".into());
        }

        for alternative in &site.alternatives {
            context.check_alternative_shape(alternative)?;
            if live {
                context.check_alternative_edge(site, alternative)?;
            }
            let observed = self.observed_frontier(context, site, alternative);
            let expected: BTreeSet<_> = alternative
                .completion_cfg_node_ids
                .iter()
                .cloned()
                .collect();
            if !observed.is_empty() && observed != expected {
                return Err("Cyclic alternative completion frontier is not exact".into());
            }
        }
        Ok(())
    }

    fn observed_frontier(
        &self,
        context: &Context<'_>,
        site: &Site,
        alternative: &Alternative,
    ) -> BTreeSet<String> {
        let mut observed = BTreeSet::new();
        for &(source, target) in &self.edges {
            if self.states[target].vertex != site.cfg_successor_node_id {
                continue;
            }
            let before = self.before[target].iter().collect();
            if context.satisfies_before(
                &alternative.annotations.guards,
                &self.availability(target, &before),
            ) {
                observed.insert(self.states[source].vertex.clone());
            }
        }
        observed
    }

    fn check_consumers(&self, context: &Context<'_>) -> Result<(), String> {
        for edge in context
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.subkind == "value_operand")
        {
            if context.node(&edge.source)?.category != NodeCategory::Merge {
                continue;
            }
            let consumer = cfg_operation(context, &edge.target)?;
            for (index, _) in self
                .states
                .iter()
                .enumerate()
                .filter(|(_, state)| state.vertex == consumer)
            {
                let before = self.before[index].iter().collect();
                if !context.available(&edge.source, &self.availability(index, &before))? {
                    return Err("Cyclic merge operand is unavailable before consumer".into());
                }
            }
        }
        Ok(())
    }
}

fn transition(
    context: &Context<'_>,
    state: &State,
    edge: &CfgEdge,
    relevant: &BTreeSet<String>,
) -> Result<State, String> {
    let mut outcomes = state.outcomes.clone();
    if matches!(edge.kind, CfgEdgeKind::True | CfgEdgeKind::False) {
        let controller = context
            .operation_at(&edge.source)
            .ok_or("Boolean edge lacks an operation")?;
        if relevant.contains(controller) {
            outcomes.insert(
                controller.into(),
                if edge.kind == CfgEdgeKind::True {
                    "true"
                } else {
                    "false"
                }
                .into(),
            );
        }
    }
    Ok(State {
        vertex: edge.target.clone(),
        outcomes,
    })
}

fn must_evaluate(states: &[State], edges: &BTreeSet<(usize, usize)>) -> Vec<BTreeSet<String>> {
    let all: BTreeSet<_> = states.iter().map(|state| state.vertex.clone()).collect();
    let mut before = vec![all; states.len()];
    before[0].clear();
    let mut pending: BTreeSet<_> = (1..states.len()).collect();

    while let Some(index) = pending.pop_first() {
        let next = predecessor_meet(index, states, edges, &before);
        if next != before[index] {
            before[index] = next;
            pending.extend(
                edges
                    .iter()
                    .filter(|(source, target)| *source == index && *target != 0)
                    .map(|(_, target)| *target),
            );
        }
    }
    before
}

fn predecessor_meet(
    index: usize,
    states: &[State],
    edges: &BTreeSet<(usize, usize)>,
    before: &[BTreeSet<String>],
) -> BTreeSet<String> {
    let mut next = before[index].clone();
    for (source, _) in edges.iter().filter(|(_, target)| *target == index) {
        next.retain(|vertex| before[*source].contains(vertex) || states[*source].vertex == *vertex);
    }
    next
}

fn relevant_controllers(context: &Context<'_>) -> BTreeSet<String> {
    context
        .sites
        .iter()
        .flat_map(|site| {
            site.annotations
                .guards
                .iter()
                .chain(
                    site.alternatives
                        .iter()
                        .flat_map(|alternative| &alternative.annotations.guards),
                )
                .flatten()
                .map(|atom| atom.controller_pdg_node_id.clone())
                .chain(std::iter::once(site.controller_pdg_node_id.clone()))
        })
        .collect()
}

struct Network {
    states: Vec<State>,
    edges: BTreeSet<(usize, usize)>,
    index: BTreeMap<State, usize>,
}

impl Network {
    fn extend(
        &mut self,
        context: &Context<'_>,
        source: usize,
        relevant: &BTreeSet<String>,
    ) -> Result<(), String> {
        let vertex = self.states[source].vertex.clone();
        for edge in context
            .cfg
            .edges
            .iter()
            .filter(|edge| edge.source == vertex)
        {
            let state = transition(context, &self.states[source], edge, relevant)?;
            let target = self.intern(state);
            self.edges.insert((source, target));
        }
        Ok(())
    }

    fn intern(&mut self, state: State) -> usize {
        if let Some(index) = self.index.get(&state) {
            return *index;
        }

        let index = self.states.len();
        self.index.insert(state.clone(), index);
        self.states.push(state);
        index
    }
}

fn cfg_operation<'a>(context: &'a Context<'_>, id: &str) -> Result<&'a str, String> {
    let mut entries = context
        .operations
        .iter()
        .filter(|(_, operation)| operation.as_str() == id);
    match (entries.next(), entries.next()) {
        (Some((cfg, _)), None) => Ok(cfg.as_str()),
        _ => Err("Missing or ambiguous cyclic operation mapping".into()),
    }
}

fn repeats(cfg: &CfgDocument, start: &str) -> bool {
    let mut pending: Vec<_> = cfg
        .edges
        .iter()
        .filter(|edge| edge.source == start)
        .map(|edge| edge.target.as_str())
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(vertex) = pending.pop() {
        if vertex == start {
            return true;
        }
        if visited.insert(vertex) {
            pending.extend(
                cfg.edges
                    .iter()
                    .filter(|edge| edge.source == vertex)
                    .map(|edge| edge.target.as_str()),
            );
        }
    }
    false
}

fn check_nonrepeating(context: &Context<'_>, site: &Site) -> Result<(), String> {
    let controller = cfg_operation(context, &site.controller_pdg_node_id)?;
    if repeats(context.cfg, controller) {
        return Err("Value-merge controller can repeat in a cycle".into());
    }
    for alternative in &site.alternatives {
        let value = context.node(&alternative.value_pdg_node_id)?;
        if value.annotations.get(TYPE).and_then(known_type) == Some("unit") {
            return Err("Unit cyclic alternative value".into());
        }
        if !matches!(
            value.category,
            NodeCategory::Merge | NodeCategory::FormalInput
        ) && repeats(context.cfg, cfg_operation(context, &value.id)?)
        {
            return Err("Cyclic alternative snapshot requires iteration evidence".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
