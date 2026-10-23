//! Finite, alias-conservative memory flow over the producer's ordered CFG.
use super::Graph;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
mod source;
pub use source::from_graph;

const MAX_NODES: usize = 4096;
const MAX_ROOTS: usize = 64;
const MAX_STEPS: usize = 2_000_000;
const MAX_DEPENDENCIES: usize = 262_144;
const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessKind {
    Load,
    Store,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAccess {
    pub operation: String,
    pub location: Location,
    pub kind: AccessKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitKind {
    Normal,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryExit {
    pub operation: String,
    pub kind: ExitKind,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Definition {
    Entry(Location),
    Operation(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReachingDefinition {
    pub definition: Definition,
    /// A path from this definition to this use crosses a declared back edge.
    pub loop_carried: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependenceKind {
    Raw,
    War,
    Waw,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryDependence {
    pub source: Definition,
    pub target: String,
    pub location: Location,
    pub kind: DependenceKind,
    pub loop_carried: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitState {
    pub kind: ExitKind,
    pub reachable: bool,
    pub reaching: BTreeMap<Location, BTreeSet<ReachingDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAnalysis {
    pub dependencies: Vec<MemoryDependence>,
    pub exits: BTreeMap<String, ExitState>,
}

/// Borrow the checked locations, effects, alias uncertainty and exit policy
/// together; none of these inventories is meaningful independently.
#[derive(Clone, Copy)]
pub struct AccessInventory<'a> {
    pub roots: &'a [Location],
    pub accesses: &'a [MemoryAccess],
    pub aliases: &'a [(Location, Location)],
    pub exits: &'a [MemoryExit],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct State {
    writes: BTreeMap<Location, BTreeSet<ReachingDefinition>>,
    reads: BTreeMap<Location, BTreeSet<ReachingDefinition>>,
}

struct Inputs<'a> {
    accesses: BTreeMap<&'a str, &'a MemoryAccess>,
    peers: BTreeMap<Location, BTreeSet<Location>>,
    successors: BTreeMap<String, Vec<(String, bool)>>,
}

/// Each access must have its own operation. Aliases are symmetric possible
/// overlap, not equivalence classes: their transitive closure is not inferred.
/// Failure exits observe exactly the state on their incoming CFG paths.
pub fn analyze(graph: &Graph, inventory: AccessInventory<'_>) -> Result<MemoryAnalysis, String> {
    let inputs = inputs(graph, inventory)?;
    let (states, dependencies) = propagate(&inputs, inventory.roots)?;
    let exits = inventory
        .exits
        .iter()
        .map(|exit| {
            let state = states.get(&exit.operation);
            (
                exit.operation.clone(),
                ExitState {
                    kind: exit.kind,
                    reachable: state.is_some(),
                    reaching: state.map(|state| state.writes.clone()).unwrap_or_default(),
                },
            )
        })
        .collect();
    Ok(MemoryAnalysis {
        dependencies: dependencies.into_iter().collect(),
        exits,
    })
}

fn inputs<'a>(graph: &Graph, inventory: AccessInventory<'a>) -> Result<Inputs<'a>, String> {
    let AccessInventory {
        roots,
        accesses,
        aliases,
        exits,
    } = inventory;
    graph_budget(graph)?;

    if roots.len() > MAX_ROOTS
        || exits.len() > MAX_NODES
        || accesses.len() > MAX_NODES
        || aliases.len() > MAX_ROOTS * MAX_ROOTS
        || exits.iter().any(|exit| exit.operation.len() > MAX_ID_BYTES)
    {
        return Err("Memory CFG or roots exceed their budget".into());
    }

    let roots_set: BTreeSet<_> = roots.iter().copied().collect();
    if roots_set.len() != roots.len() {
        return Err("Duplicate or over-budget memory roots".into());
    }

    let mut ids = BTreeSet::from(["E".to_owned(), "X".to_owned()]);
    for node in &graph.nodes {
        if !ids.insert(node.id.clone()) {
            return Err("Duplicate memory CFG operation identity".into());
        }
    }

    let mut exit_ids = BTreeSet::new();
    for exit in exits {
        if exit.operation == "E" || !exit_ids.insert(exit.operation.clone()) {
            return Err("Invalid or duplicate memory exit".into());
        }
        ids.insert(exit.operation.clone());
    }
    if ids.len() > MAX_NODES {
        return Err("Memory CFG exceeds its budget".into());
    }

    let accesses = access_index(accesses, &ids, &roots_set, &exit_ids)?;
    let peers = alias_index(roots, aliases, &roots_set)?;
    let successors = successors(graph, &ids, &exit_ids)?;
    Ok(Inputs {
        accesses,
        peers,
        successors,
    })
}

fn graph_budget(graph: &Graph) -> Result<(), String> {
    if graph.nodes.len() > MAX_NODES
        || graph.transfers.len() > MAX_NODES * 4
        || graph.nodes.iter().any(|node| node.id.len() > MAX_ID_BYTES)
        || graph
            .transfers
            .iter()
            .any(|edge| edge.source.len() > MAX_ID_BYTES || edge.target.len() > MAX_ID_BYTES)
    {
        return Err("Memory CFG exceeds its node, edge or identity budget".into());
    }
    Ok(())
}

fn access_index<'a>(
    accesses: &'a [MemoryAccess],
    ids: &BTreeSet<String>,
    roots: &BTreeSet<Location>,
    exits: &BTreeSet<String>,
) -> Result<BTreeMap<&'a str, &'a MemoryAccess>, String> {
    if accesses.len() > MAX_NODES {
        return Err("Memory accesses exceed their budget".into());
    }
    let mut result = BTreeMap::new();
    for access in accesses {
        if !ids.contains(&access.operation)
            || !roots.contains(&access.location)
            || access.operation == "E"
            || access.operation == "X"
            || exits.contains(&access.operation)
            || result.insert(access.operation.as_str(), access).is_some()
        {
            return Err("Unknown root/operation or duplicate memory access".into());
        }
    }
    Ok(result)
}

fn alias_index(
    roots: &[Location],
    aliases: &[(Location, Location)],
    known: &BTreeSet<Location>,
) -> Result<BTreeMap<Location, BTreeSet<Location>>, String> {
    if aliases.len() > MAX_ROOTS * MAX_ROOTS {
        return Err("Memory aliases exceed their budget".into());
    }
    let mut result: BTreeMap<_, _> = roots
        .iter()
        .map(|root| (*root, BTreeSet::from([*root])))
        .collect();
    for &(left, right) in aliases {
        if !known.contains(&left) || !known.contains(&right) {
            return Err("Memory alias names an unknown root".into());
        }
        result.get_mut(&left).unwrap().insert(right);
        result.get_mut(&right).unwrap().insert(left);
    }
    Ok(result)
}

fn successors(
    graph: &Graph,
    ids: &BTreeSet<String>,
    exits: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<(String, bool)>>, String> {
    let mut result: BTreeMap<String, Vec<(String, bool)>> = BTreeMap::new();
    for transfer in &graph.transfers {
        if !ids.contains(&transfer.source)
            || !ids.contains(&transfer.target)
            || transfer.target == "E"
            || exits.contains(&transfer.source)
            || transfer.source == "X"
            || transfer
                .loop_role
                .as_deref()
                .is_some_and(|role| !matches!(role, "back_edge" | "loop_exit"))
        {
            return Err("Invalid memory CFG transfer or exit continuation".into());
        }
        result.entry(transfer.source.clone()).or_default().push((
            transfer.target.clone(),
            transfer.loop_role.as_deref() == Some("back_edge"),
        ));
    }
    Ok(result)
}

fn initial_state(roots: &[Location]) -> State {
    let mut state = State::default();
    for &root in roots {
        state
            .writes
            .entry(root)
            .or_default()
            .insert(ReachingDefinition {
                definition: Definition::Entry(root),
                loop_carried: false,
            });
    }
    state
}

struct Worklist {
    states: BTreeMap<String, State>,
    queue: VecDeque<String>,
    queued: BTreeSet<String>,
    dependencies: BTreeSet<MemoryDependence>,
    steps: usize,
}

impl Worklist {
    fn new(roots: &[Location]) -> Self {
        Self {
            states: BTreeMap::from([("E".to_owned(), initial_state(roots))]),
            queue: VecDeque::from(["E".to_owned()]),
            queued: BTreeSet::from(["E".to_owned()]),
            dependencies: BTreeSet::new(),
            steps: 0,
        }
    }

    fn process(&mut self, id: &str, inputs: &Inputs<'_>) -> Result<(), String> {
        let mut state = self.prepare(id)?;

        if let Some(access) = inputs.accesses.get(id) {
            apply(&mut state, access, &inputs.peers, &mut self.dependencies)?;
        }
        for (target, back_edge) in inputs.successors.get(id).into_iter().flatten() {
            self.transfer(target, *back_edge, &state)?;
        }
        Ok(())
    }

    fn prepare(&mut self, id: &str) -> Result<State, String> {
        let cost = 1 + state_size(&self.states[id]);
        self.queued.remove(id);
        charge(&mut self.steps, cost)?;
        Ok(self.states[id].clone())
    }

    fn transfer(&mut self, target: &str, back_edge: bool, state: &State) -> Result<(), String> {
        charge(
            &mut self.steps,
            1 + state_size(state) + self.states.get(target).map_or(0, state_size),
        )?;
        let outgoing = if back_edge {
            carried(state)
        } else {
            state.clone()
        };
        let incoming = self.states.get(target);
        let combined = incoming.map_or_else(|| outgoing.clone(), |state| merged(state, &outgoing));

        if incoming != Some(&combined) {
            self.states.insert(target.to_owned(), combined);
            self.enqueue(target);
        }
        Ok(())
    }

    fn enqueue(&mut self, target: &str) {
        if self.queued.contains(target) {
            return;
        }
        let target = target.to_owned();
        self.queued.insert(target.clone());
        self.queue.push_back(target);
    }
}

fn propagate(
    inputs: &Inputs<'_>,
    roots: &[Location],
) -> Result<(BTreeMap<String, State>, BTreeSet<MemoryDependence>), String> {
    let mut work = Worklist::new(roots);
    while let Some(id) = work.queue.pop_front() {
        work.process(&id, inputs)?;
    }
    Ok((work.states, work.dependencies))
}

fn state_size(state: &State) -> usize {
    state
        .writes
        .values()
        .chain(state.reads.values())
        .map(BTreeSet::len)
        .sum()
}

fn charge(steps: &mut usize, amount: usize) -> Result<(), String> {
    *steps = steps.saturating_add(amount);
    if *steps > MAX_STEPS {
        return Err("Memory dataflow exceeds its work budget".into());
    }
    Ok(())
}

fn apply(
    state: &mut State,
    access: &MemoryAccess,
    peers: &BTreeMap<Location, BTreeSet<Location>>,
    dependencies: &mut BTreeSet<MemoryDependence>,
) -> Result<(), String> {
    let kind = if access.kind == AccessKind::Load {
        DependenceKind::Raw
    } else {
        DependenceKind::Waw
    };
    depend(
        state.writes.get(&access.location),
        access,
        kind,
        dependencies,
    )?;
    if access.kind == AccessKind::Store {
        depend(
            state.reads.get(&access.location),
            access,
            DependenceKind::War,
            dependencies,
        )?;
    }

    update_state(state, access, &peers[&access.location]);
    Ok(())
}

fn update_state(state: &mut State, access: &MemoryAccess, peers: &BTreeSet<Location>) {
    let generated = ReachingDefinition {
        definition: Definition::Operation(access.operation.clone()),
        loop_carried: false,
    };

    if access.kind == AccessKind::Load {
        record_peers(&mut state.reads, peers, &generated);
    } else {
        state.reads.remove(&access.location);
        record_store(&mut state.writes, access.location, peers, &generated);
    }
}

fn record_store(
    writes: &mut BTreeMap<Location, BTreeSet<ReachingDefinition>>,
    location: Location,
    peers: &BTreeSet<Location>,
    generated: &ReachingDefinition,
) {
    record_peers(writes, peers, generated);
    writes.insert(location, BTreeSet::from([generated.clone()]));
}

fn record_peers(
    definitions: &mut BTreeMap<Location, BTreeSet<ReachingDefinition>>,
    peers: &BTreeSet<Location>,
    generated: &ReachingDefinition,
) {
    for peer in peers {
        definitions
            .entry(*peer)
            .or_default()
            .insert(generated.clone());
    }
}

fn depend(
    sources: Option<&BTreeSet<ReachingDefinition>>,
    access: &MemoryAccess,
    kind: DependenceKind,
    dependencies: &mut BTreeSet<MemoryDependence>,
) -> Result<(), String> {
    for source in sources.into_iter().flatten() {
        dependencies.insert(MemoryDependence {
            source: source.definition.clone(),
            target: access.operation.clone(),
            location: access.location,
            kind,
            loop_carried: source.loop_carried,
        });
        if dependencies.len() > MAX_DEPENDENCIES {
            return Err("Memory dependencies exceed their budget".into());
        }
    }
    Ok(())
}

fn carried(state: &State) -> State {
    State {
        writes: carried_definitions(&state.writes),
        reads: carried_definitions(&state.reads),
    }
}

fn carried_definitions(
    definitions: &BTreeMap<Location, BTreeSet<ReachingDefinition>>,
) -> BTreeMap<Location, BTreeSet<ReachingDefinition>> {
    definitions
        .iter()
        .map(|(location, definitions)| {
            let carried = definitions
                .iter()
                .map(|definition| ReachingDefinition {
                    definition: definition.definition.clone(),
                    loop_carried: true,
                })
                .collect();
            (*location, carried)
        })
        .collect()
}

fn merged(target: &State, source: &State) -> State {
    State {
        writes: merged_definitions(&target.writes, &source.writes),
        reads: merged_definitions(&target.reads, &source.reads),
    }
}

fn merged_definitions(
    target: &BTreeMap<Location, BTreeSet<ReachingDefinition>>,
    source: &BTreeMap<Location, BTreeSet<ReachingDefinition>>,
) -> BTreeMap<Location, BTreeSet<ReachingDefinition>> {
    target
        .keys()
        .chain(source.keys())
        .map(|location| {
            let definitions = target
                .get(location)
                .into_iter()
                .flatten()
                .chain(source.get(location).into_iter().flatten())
                .cloned()
                .collect();
            (*location, definitions)
        })
        .collect()
}

#[cfg(test)]
mod tests;
