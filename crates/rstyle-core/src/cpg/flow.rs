//! What a function's control flow establishes, for any frontend that emits it: dominators,
//! post-dominators, reaching definitions, control dependence and the dependence graph they
//! give. Frontends emit the flow and the accesses; all of this is derived, never emitted.

use super::model::{
    AccessMode, Coverage, Declarations, Function, NodeKind, Origin, PlaceMode, Status,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// A vertex's position in its flow.
pub type Vertex = usize;

/// Vertices times transfers past which termination dependence is not computed: its cost, in
/// the worst case. Far above any function a person wrote.
const TERMINATION_BUDGET: usize = 50_000_000;

/// A function's control flow, restricted to what its entry reaches.
pub struct Flow<'f> {
    ids: Vec<&'f str>,
    indices: HashMap<&'f str, Vertex>,
    /// Each vertex's outgoing transfers: target, kind and loop role.
    edges: Vec<Vec<(Vertex, &'f str, Option<&'f str>)>>,
    forward: Graph,
    entry: Vertex,
    exits: Vec<Vertex>,
}

/// Adjacency, with what a traversal from a root needs.
struct Graph {
    successors: Vec<Vec<Vertex>>,
    predecessors: Vec<Vec<Vertex>>,
}

impl Graph {
    fn new(successors: Vec<Vec<Vertex>>) -> Self {
        let mut predecessors = vec![Vec::new(); successors.len()];
        for (source, targets) in successors.iter().enumerate() {
            for &target in targets {
                predecessors[target].push(source);
            }
        }
        Self {
            successors,
            predecessors,
        }
    }

    // rstyle-allow(mutated_targets): an iterative depth-first walk keeps its stack, marks and output
    /// Reverse postorder from the root: each vertex before its successors, loops aside.
    fn order(&self, root: Vertex) -> Vec<Vertex> {
        let mut seen = vec![false; self.successors.len()];
        let mut finished = Vec::with_capacity(self.successors.len());
        let mut stack = vec![(root, 0)];

        seen[root] = true;
        while let Some((vertex, next)) = stack.last_mut() {
            match self.successors[*vertex].get(*next) {
                Some(&target) => {
                    *next += 1;
                    if !std::mem::replace(&mut seen[target], true) {
                        stack.push((target, 0));
                    }
                }
                None => {
                    finished.push(*vertex);
                    stack.pop();
                }
            }
        }
        finished.reverse();

        finished
    }

    /// Immediate dominators from the root (Cooper, Harvey and Kennedy): each vertex's nearest
    /// strict dominator, and the root its own.
    fn immediate(&self, root: Vertex) -> Vec<Option<Vertex>> {
        let order = self.order(root);
        let mut position = vec![usize::MAX; self.successors.len()];
        for (at, &vertex) in order.iter().enumerate() {
            position[vertex] = at;
        }

        let mut parent = vec![None; self.successors.len()];
        parent[root] = Some(root);
        let mut changed = true;

        while changed {
            changed = false;
            for &vertex in &order[1..] {
                let joined = self.predecessors[vertex]
                    .iter()
                    .copied()
                    .filter(|&predecessor| parent[predecessor].is_some())
                    .reduce(|left, right| meet(&parent, &position, left, right));
                changed |= parent[vertex] != joined;
                parent[vertex] = joined;
            }
        }
        parent
    }
}

fn meet(
    parent: &[Option<Vertex>],
    position: &[usize],
    mut left: Vertex,
    mut right: Vertex,
) -> Vertex {
    while left != right {
        while position[left] > position[right] {
            left = parent[left].unwrap_or(left);
        }
        while position[right] > position[left] {
            right = parent[right].unwrap_or(right);
        }
    }
    left
}

/// Immediate (post-)dominators of a flow's vertices.
pub struct Tree {
    parent: Vec<Option<Vertex>>,
    root: Vertex,
}

impl Tree {
    /// The vertex's nearest strict (post-)dominator. A vertex whose only one is the virtual
    /// exit every path ends at has none.
    pub fn parent(&self, vertex: Vertex) -> Option<Vertex> {
        self.parent[vertex].filter(|&parent| parent != vertex && parent != self.root)
    }

    /// Whether `ancestor` is the vertex or (post-)dominates it.
    pub fn contains(&self, ancestor: Vertex, vertex: Vertex) -> bool {
        let mut at = Some(vertex);
        while let Some(current) = at {
            if current == ancestor {
                return true;
            }
            at = self.parent(current);
        }
        false
    }
}

impl<'f> Flow<'f> {
    // rstyle-allow(mutated_targets, mutation_per_target): a flow is built by interning its vertices and their edges
    /// The function's flow, or `None` when it emits none.
    pub fn of(function: &'f Function) -> Option<Self> {
        let cfg = &function.cfg;
        if cfg.edges.is_empty() {
            return None;
        }
        let mut indices: HashMap<&'f str, Vertex> = HashMap::new();

        let mut ids: Vec<&'f str> = Vec::new();
        let mut index = |id: &'f str| {
            *indices.entry(id).or_insert_with(|| {
                ids.push(id);
                ids.len() - 1
            })
        };
        let entry = index(&cfg.entry);

        let exits: Vec<Vertex> = cfg.exits.iter().map(|exit| index(&exit.id)).collect();
        let mut transfers = Vec::with_capacity(cfg.edges.len());
        for edge in &cfg.edges {
            let (source, target) = (index(&edge.source), index(&edge.target));
            transfers.push((
                source,
                target,
                edge.kind.as_str(),
                edge.loop_role.as_deref(),
            ));
        }

        let mut edges = vec![Vec::new(); ids.len()];
        let mut successors = vec![Vec::new(); ids.len()];
        for (source, target, kind, role) in transfers {
            edges[source].push((target, kind, role));
            successors[source].push(target);
        }

        Some(Self {
            forward: Graph::new(successors),
            ids,
            indices,
            edges,
            entry,
            exits,
        })
    }

    pub fn id(&self, vertex: Vertex) -> &'f str {
        self.ids[vertex]
    }

    pub fn vertex(&self, id: &str) -> Option<Vertex> {
        self.indices.get(id).copied()
    }

    pub fn entry(&self) -> Vertex {
        self.entry
    }

    /// The vertices the entry reaches, each before its successors, loops aside.
    pub fn order(&self) -> Vec<Vertex> {
        self.forward.order(self.entry)
    }

    pub fn dominators(&self) -> Tree {
        Tree {
            parent: self.forward.immediate(self.entry),
            root: self.entry,
        }
    }

    /// Post-dominators against a virtual exit that every path ends at: the exits, a call
    /// that never returns, and a loop nothing leaves.
    pub fn post_dominators(&self) -> Tree {
        let size = self.ids.len();
        let root = size;
        let reachable = self.order();

        let mut ends: Vec<Vertex> = self.exits.clone();
        ends.extend(
            reachable
                .iter()
                .copied()
                .filter(|&vertex| self.forward.successors[vertex].is_empty()),
        );
        loop {
            let reversed = self.reversed(root, &ends);
            let seen: HashSet<Vertex> = reversed.order(root).into_iter().collect();
            // The last vertex in order that nothing ends is where its endless loop repeats.
            match reachable.iter().rev().find(|vertex| !seen.contains(vertex)) {
                Some(&stuck) => ends.push(stuck),
                None => {
                    return Tree {
                        parent: reversed.immediate(root),
                        root,
                    };
                }
            }
        }
    }

    fn reversed(&self, root: Vertex, ends: &[Vertex]) -> Graph {
        let mut successors = vec![Vec::new(); root + 1];
        for (source, targets) in self.forward.successors.iter().enumerate() {
            for &target in targets {
                successors[target].push(source);
            }
        }
        successors[root] = ends.to_vec();
        Graph::new(successors)
    }

    /// Each transfer a branch makes to a vertex that does not always follow it, with
    /// every vertex that runs because of it.
    pub fn control_dependence(&self, post: &Tree) -> Vec<(Vertex, Vertex, &'f str)> {
        let mut found = BTreeSet::new();
        for source in self.order() {
            let stop = post.parent(source);
            for &(target, kind, _) in &self.edges[source] {
                let mut at = Some(target);
                while let Some(current) = at.filter(|&current| Some(current) != stop) {
                    found.insert((source, current, kind));
                    at = post.parent(current);
                }
            }
        }
        found.into_iter().collect()
    }

    /// Whether any path through the function repeats a vertex.
    fn has_cycle(&self) -> bool {
        let order = self.order();
        let mut position = vec![usize::MAX; self.ids.len()];
        for (at, &vertex) in order.iter().enumerate() {
            position[vertex] = at;
        }
        order.iter().any(|&vertex| {
            self.forward.successors[vertex]
                .iter()
                .any(|&target| position[target] <= position[vertex])
        })
    }

    // rstyle-allow(mutated_targets): one propagation per candidate reuses its working sets
    /// Every vertex each vertex is sure to reach: on all paths from it, however it goes,
    /// and with no chance of repeating a loop forever first.
    fn sure_to_reach(&self) -> Vec<Vec<Vertex>> {
        let degrees: Vec<usize> = self.forward.successors.iter().map(Vec::len).collect();
        let mut reached = vec![Vec::new(); self.ids.len()];
        let mut remaining = degrees.clone();

        let mut queue = Vec::with_capacity(self.ids.len());
        for candidate in self.order() {
            remaining.copy_from_slice(&degrees);
            queue.clear();
            queue.push(candidate);
            // The candidate counts as reached however it continues; a repeat must not re-add it.
            remaining[candidate] = 0;
            let mut head = 0;
            while let Some(&vertex) = queue.get(head) {
                head += 1;
                reached[vertex].push(candidate);
                for &predecessor in &self.forward.predecessors[vertex] {
                    if remaining[predecessor] == 0 {
                        continue;
                    }
                    remaining[predecessor] -= 1;
                    if remaining[predecessor] == 0 {
                        queue.push(predecessor);
                    }
                }
            }
        }
        reached
    }

    /// What runs only if a loop before it finishes, beyond what post-dominance says: each
    /// branch, and what one way out of it is sure to reach but the branch itself is not.
    /// `None` when the function is too large for this to be worth computing.
    fn termination_dependence(
        &self,
        known: &BTreeSet<(Vertex, Vertex, &'f str)>,
    ) -> Option<Vec<(Vertex, Vertex, &'f str)>> {
        if !self.has_cycle() {
            return Some(Vec::new());
        }
        let transfers: usize = self.edges.iter().map(Vec::len).sum();
        if self.ids.len().saturating_mul(transfers) > TERMINATION_BUDGET {
            return None;
        }

        let reached = self.sure_to_reach();
        let mut found = BTreeSet::new();
        for source in self.order() {
            let mine: HashSet<Vertex> = reached[source].iter().copied().collect();
            let distinct: HashSet<Vertex> =
                self.forward.successors[source].iter().copied().collect();
            if distinct.len() < 2 {
                continue;
            }
            for &(target, kind, _) in &self.edges[source] {
                for &candidate in &reached[target] {
                    let new = candidate == source || !mine.contains(&candidate);
                    if new && !known.contains(&(source, candidate, kind)) {
                        found.insert((source, candidate, kind));
                    }
                }
            }
        }

        Some(found.into_iter().collect())
    }

    /// Each loop's head and the transfers that repeat it.
    pub fn back_edges(&self) -> Vec<(Vertex, Vertex)> {
        self.order()
            .into_iter()
            .flat_map(|source| {
                self.edges[source]
                    .iter()
                    .filter(|(_, _, role)| *role == Some("back_edge"))
                    .map(move |&(target, _, _)| (source, target))
            })
            .collect()
    }
}

/// A set of definitions.
#[derive(Clone, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn empty(size: usize) -> Self {
        Self(vec![0; size.div_ceil(64)])
    }

    fn insert(&mut self, bit: usize) {
        self.0[bit / 64] |= 1 << (bit % 64);
    }

    fn contains(&self, bit: usize) -> bool {
        self.0[bit / 64] & (1 << (bit % 64)) != 0
    }

    fn union(&mut self, other: &Self) {
        for (word, more) in self.0.iter_mut().zip(&other.0) {
            *word |= more;
        }
    }

    fn remove_all(&mut self, other: &Self) {
        for (word, gone) in self.0.iter_mut().zip(&other.0) {
            *word &= !gone;
        }
    }
}

/// One write of a symbol at a vertex.
#[derive(Clone, Copy)]
pub struct Definition<'f> {
    pub vertex: Vertex,
    pub symbol: &'f str,
}

/// One read of a symbol at a vertex.
struct Read<'f> {
    vertex: Vertex,
    symbol: &'f str,
}

/// Which definitions reach each vertex, and who reads what.
pub struct Reaching<'f> {
    pub definitions: Vec<Definition<'f>>,
    reads: Vec<Read<'f>>,
    before: Vec<Bits>,
}

impl<'f> Reaching<'f> {
    /// Where each symbol is read, written and overwritten. A parameter is written at entry.
    /// A write of part of a place, or through a borrow, keeps what was there.
    pub fn of(function: &'f Function, flow: &Flow<'f>) -> Self {
        let strong = whole_updates(function);
        let mut definitions: Vec<Definition<'f>> = function
            .symbols
            .iter()
            .filter(|symbol| symbol.node.is_none())
            .map(|symbol| Definition {
                vertex: flow.entry(),
                symbol: &symbol.id,
            })
            .collect();
        let mut reads = Vec::new();

        let mut overwrites: BTreeMap<Vertex, BTreeSet<&'f str>> = BTreeMap::new();
        for access in &function.accesses {
            let Some(vertex) = flow.vertex(&access.node) else {
                continue;
            };
            let symbol = access.symbol.as_str();
            if access.mode != AccessMode::Read {
                definitions.push(Definition { vertex, symbol });
            }
            if access.mode != AccessMode::Write {
                reads.push(Read { vertex, symbol });
            }
            if access.mode == AccessMode::Write || strong.contains(access.node.as_str()) {
                overwrites.entry(vertex).or_default().insert(symbol);
            }
        }
        let before = fixed_point(flow, &definitions, &overwrites);

        Self {
            definitions,
            reads,
            before,
        }
    }

    /// Whether the definition reaches the vertex before it runs.
    pub fn reaches(&self, definition: usize, vertex: Vertex) -> bool {
        self.before[vertex].contains(definition)
    }

    /// Each read with each definition that reaches it: definition vertex, read vertex, symbol.
    pub fn data_dependence(&self) -> Vec<(Vertex, Vertex, &'f str)> {
        let mut by_symbol: HashMap<&str, Vec<usize>> = HashMap::new();
        for (index, definition) in self.definitions.iter().enumerate() {
            by_symbol.entry(definition.symbol).or_default().push(index);
        }
        let mut found = BTreeSet::new();

        for read in &self.reads {
            for &definition in by_symbol.get(read.symbol).into_iter().flatten() {
                if self.reaches(definition, read.vertex) {
                    let source = self.definitions[definition].vertex;
                    found.insert((source, read.vertex, read.symbol));
                }
            }
        }
        found.into_iter().collect()
    }
}

/// `x += 1` replaces what `x` held; other reads and writes at one node keep it.
fn whole_updates(function: &Function) -> HashSet<&str> {
    let kinds: HashMap<&str, NodeKind> = function
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.kind))
        .collect();
    function
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::CompoundAssignment)
        .filter_map(|node| node.children.first())
        .map(String::as_str)
        .filter(|target| kinds.get(target) == Some(&NodeKind::Identifier))
        .collect()
}

// rstyle-allow(mutated_targets): reaching definitions iterates its sets until they settle
/// The definitions reaching each vertex, propagated until nothing changes.
fn fixed_point(
    flow: &Flow<'_>,
    definitions: &[Definition<'_>],
    overwrites: &BTreeMap<Vertex, BTreeSet<&str>>,
) -> Vec<Bits> {
    let size = flow.ids.len();
    let count = definitions.len();
    let mut made = vec![Bits::empty(count); size];

    let mut of_symbol: HashMap<&str, Bits> = HashMap::new();
    for (index, definition) in definitions.iter().enumerate() {
        made[definition.vertex].insert(index);
        of_symbol
            .entry(definition.symbol)
            .or_insert_with(|| Bits::empty(count))
            .insert(index);
    }
    let mut removed = vec![Bits::empty(count); size];

    for (&vertex, symbols) in overwrites {
        for symbol in symbols {
            removed[vertex].union(&of_symbol[symbol]);
        }
    }
    let order = flow.order();
    let mut before = vec![Bits::empty(count); size];

    let mut after = made.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for &vertex in &order {
            let mut incoming = Bits::empty(count);
            for &predecessor in &flow.forward.predecessors[vertex] {
                incoming.union(&after[predecessor]);
            }
            let mut leaving = incoming.clone();
            leaving.remove_all(&removed[vertex]);
            leaving.union(&made[vertex]);
            before[vertex] = incoming;
            changed |= after[vertex] != leaving;
            after[vertex] = leaving;
        }
    }

    before
}

/// How far a function's derived graph can be relied on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Follows from what the frontend recorded.
    Known,
    /// Holds except where `reasons` say the recorded facts stop short.
    Approximated,
    /// The frontend records too little to say; coverage can make it known.
    Unknown,
}

/// A data edge: the write at `source` can be what the read at `target` sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DataEdge {
    pub source: String,
    pub target: String,
    pub symbol: String,
    /// The symbol can also change through a borrow or a capture, which no edge shows.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub approximate: bool,
}

/// A control edge: `target` runs because the branch at `source` went this way.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ControlEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
    /// Holds only because a loop between them might never finish; a target that
    /// post-dominates the branch otherwise.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub termination: bool,
}

/// An execution edge: `source` is the nearest operation that always runs before `target`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExecutionEdge {
    pub source: String,
    pub target: String,
}

/// A function's derived flow facts, in terms of the node IDs of its document.
#[derive(Clone, Debug, Serialize)]
pub struct Derived {
    pub standing: Standing,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    /// What the frontend leaves out of every function's flow.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<String>,
    /// Each reached vertex and its immediate dominator.
    pub dominators: BTreeMap<String, String>,
    /// Each reached vertex and its immediate post-dominator, when one is not the virtual exit.
    pub post_dominators: BTreeMap<String, String>,
    /// Absent when the frontend records no accesses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<DataEdge>>,
    pub control: Vec<ControlEdge>,
    pub execution: Vec<ExecutionEdge>,
    /// Each loop head, with the vertices that repeat it.
    pub loops: BTreeMap<String, Vec<String>>,
}

impl Derived {
    fn unknown(reason: String) -> Self {
        Self {
            standing: Standing::Unknown,
            reasons: vec![reason],
            assumptions: Vec::new(),
            dominators: BTreeMap::new(),
            post_dominators: BTreeMap::new(),
            data: None,
            control: Vec::new(),
            execution: Vec::new(),
            loops: BTreeMap::new(),
        }
    }
}

/// The status a document or the function itself declares of a coverage entry.
fn declared((document, function): Entries<'_>, name: &str) -> Option<Status> {
    function
        .and_then(|declarations| declarations.get(name))
        .or_else(|| document.get(name))
        .map(|capability| capability.status)
}

fn usable(status: Option<Status>) -> bool {
    matches!(status, Some(Status::Supported | Status::Approximated))
}

/// Symbols something other than their name can change: borrowed exclusively, reached by a
/// raw pointer, captured to be changed, or written with what was there kept, which is what a
/// read and write at one node says unless it replaces the value. Their data edges say what
/// the names do, not everything that happens. What a lowering does to run the construct it
/// stands for says nothing of this.
pub fn changed_elsewhere<'f>(function: &'f Function, flow: &Flow<'f>) -> BTreeSet<&'f str> {
    let strong = whole_updates(function);
    let written: HashSet<&str> = function
        .nodes
        .iter()
        .filter(|node| node.provenance.origin != Origin::Desugaring)
        .filter(|node| flow.vertex(&node.id).is_some())
        .map(|node| node.id.as_str())
        .collect();
    let borrowed = function
        .places
        .iter()
        .filter(|place| matches!(place.mode, PlaceMode::Exclusive | PlaceMode::Raw))
        .filter(|place| written.contains(place.node.as_str()))
        .filter_map(|place| place.root.symbol.as_deref());
    let captured = function
        .nodes
        .iter()
        .filter(|node| written.contains(node.id.as_str()))
        .flat_map(|node| node.captures.iter().flatten())
        .filter(|capture| matches!(capture.mode.as_str(), "exclusive" | "unique"))
        .map(|capture| capture.symbol.as_str());
    let kept = function
        .accesses
        .iter()
        .filter(|access| access.mode == AccessMode::ReadWrite)
        .filter(|access| written.contains(access.node.as_str()))
        .filter(|access| !strong.contains(access.node.as_str()))
        .map(|access| access.symbol.as_str());

    borrowed.chain(captured).chain(kept).collect()
}

/// Symbols written in part: data edges treat a place as a whole, so a read of one field
/// can show an edge from a write of another.
fn written_in_part<'f>(function: &'f Function, flow: &Flow<'f>) -> BTreeSet<&'f str> {
    function
        .places
        .iter()
        .filter(|place| !place.path.is_empty() && flow.vertex(&place.node).is_some())
        .filter_map(|place| place.root.symbol.as_deref())
        .collect()
}

/// A coverage table and the function's own override of it.
type Entries<'c> = (&'c Declarations, Option<&'c Declarations>);

/// The layers and capabilities a function is covered by.
fn entries<'c>(coverage: &'c Coverage, function: &'c Function) -> (Entries<'c>, Entries<'c>) {
    let own = function.coverage.as_ref();
    let layers = (&coverage.layers, own.and_then(|own| own.layers.as_ref()));
    let capabilities = (
        &coverage.capabilities,
        own.and_then(|own| own.capabilities.as_ref()),
    );
    (layers, capabilities)
}

/// A function's flow, or why it has none to read.
fn declared_flow<'f>(coverage: &Coverage, function: &'f Function) -> Result<Flow<'f>, String> {
    let (layers, _) = entries(coverage, function);
    if !usable(declared(layers, "cfg")) {
        return Err("The frontend does not emit this function's control flow".into());
    }
    Flow::of(function).ok_or_else(|| "The function records no control flow".to_owned())
}

/// Whether a function's accesses can be taken as all of them. A frontend that declares its
/// accesses complete can have none to record.
fn accesses_recorded(coverage: &Coverage, function: &Function) -> bool {
    let (layers, _) = entries(coverage, function);
    !function.accesses.is_empty()
        || function.symbols.is_empty()
        || declared(layers, "symbol") == Some(Status::Supported)
}

/// A function's flow when all of its syntax, what reads and writes each binding and what its calls
/// resolve to are known too, or why they aren't. Whatever isn't in the tree, such as code a
/// compiler couldn't parse or a construct a frontend doesn't model, can do anything, so nothing can
/// be said to be unused.
pub fn with_accesses<'f>(coverage: &Coverage, function: &'f Function) -> Result<Flow<'f>, String> {
    let flow = declared_flow(coverage, function)?;
    let (layers, capabilities) = entries(coverage, function);

    if declared(layers, "ast") != Some(Status::Supported) {
        return Err("The frontend does not hold all of this function's syntax".into());
    }
    if function
        .nodes
        .iter()
        .any(|node| node.kind == NodeKind::Unsupported)
    {
        return Err("The function holds a construct the frontend does not model".into());
    }

    if !accesses_recorded(coverage, function) {
        return Err("The frontend records no accesses for this function".into());
    }
    if !usable(declared(capabilities, "aliasing")) {
        return Err("The frontend does not say what can alias in this function".into());
    }
    if !usable(declared(capabilities, "resolution")) {
        return Err("The frontend does not say what this function's calls resolve to".into());
    }
    Ok(flow)
}

/// Everything derived of a function under this coverage. Unknown where the frontend emits
/// no flow, whatever the reason: the same function becomes known as its coverage grows.
pub fn derive(coverage: &Coverage, function: &Function) -> Derived {
    let (_, capabilities) = entries(coverage, function);
    let flow = match declared_flow(coverage, function) {
        Ok(flow) => flow,
        Err(reason) => return Derived::unknown(reason),
    };
    let (dominators, post) = (flow.dominators(), flow.post_dominators());

    let aliased = changed_elsewhere(function, &flow);
    let data = accesses_recorded(coverage, function).then(|| data_edges(function, &flow, &aliased));

    let (control, too_large) = control_edges(&flow, &post);
    let mut reasons = limits((function, &flow), data.is_some(), capabilities, &aliased);
    reasons.extend(too_large);

    Derived {
        standing: match reasons.is_empty() {
            true => Standing::Known,
            false => Standing::Approximated,
        },
        reasons,
        assumptions: match declared(capabilities, "exceptional-flow") {
            Some(Status::Supported) => Vec::new(),
            _ => vec!["Unwinding and exceptions are not transfers here".into()],
        },
        dominators: tree_map(&flow, &dominators),
        post_dominators: tree_map(&flow, &post),
        data,
        control,
        execution: execution_edges(&flow, &dominators),
        loops: loops(&flow),
    }
}

// rstyle-allow(mutation_per_target): adds each limit that applies
/// Where the recorded facts stop short of the whole function.
fn limits<'f>(
    (function, flow): (&'f Function, &Flow<'f>),
    has_data: bool,
    capabilities: Entries<'_>,
    aliased: &BTreeSet<&str>,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !has_data {
        reasons.push("The frontend records no accesses, so data dependence is unknown".into());
    }
    if !usable(declared(capabilities, "aliasing")) {
        reasons.push("The frontend does not say what can alias".into());
    }

    if !aliased.is_empty() {
        reasons.push(format!(
            "{} can also change through a borrow, a capture or a write to part of it",
            names(function, aliased)
        ));
    }
    let partial = written_in_part(function, flow);
    if !partial.is_empty() {
        reasons.push(format!(
            "{} is written in part, and data edges treat a place as a whole",
            names(function, &partial)
        ));
    }

    reasons
}

fn names(function: &Function, symbols: &BTreeSet<&str>) -> String {
    let written: BTreeSet<&str> = function
        .symbols
        .iter()
        .filter(|symbol| symbols.contains(symbol.id.as_str()))
        .map(|symbol| symbol.name.as_str())
        .collect();
    written.into_iter().collect::<Vec<_>>().join(", ")
}

fn tree_map(flow: &Flow<'_>, tree: &Tree) -> BTreeMap<String, String> {
    flow.order()
        .into_iter()
        .filter_map(|vertex| {
            let parent = tree.parent(vertex)?;
            Some((flow.id(vertex).to_owned(), flow.id(parent).to_owned()))
        })
        .collect()
}

fn data_edges(function: &Function, flow: &Flow<'_>, aliased: &BTreeSet<&str>) -> Vec<DataEdge> {
    Reaching::of(function, flow)
        .data_dependence()
        .into_iter()
        .map(|(source, target, symbol)| DataEdge {
            source: flow.id(source).to_owned(),
            target: flow.id(target).to_owned(),
            symbol: symbol.to_owned(),
            approximate: aliased.contains(symbol),
        })
        .collect()
}

/// A vertex that is not the entry or an exit is an operation of the body.
fn operation(flow: &Flow<'_>, vertex: Vertex) -> bool {
    vertex != flow.entry && !flow.exits.contains(&vertex)
}

/// What each branch decides. A function too large to say what depends on its loops
/// finishing states that, and keeps what post-dominance gives it.
fn control_edges(flow: &Flow<'_>, post: &Tree) -> (Vec<ControlEdge>, Option<String>) {
    let classic: BTreeSet<_> = flow.control_dependence(post).into_iter().collect();
    let extra = flow.termination_dependence(&classic);
    let limit = extra
        .is_none()
        .then(|| "Too large to say what depends on its loops finishing".to_owned());

    let marked = classic.into_iter().map(|edge| (edge, false));
    let edges = marked
        .chain(extra.into_iter().flatten().map(|edge| (edge, true)))
        .filter(|&((source, target, _), _)| operation(flow, source) && operation(flow, target))
        .map(|((source, target, kind), termination)| ControlEdge {
            source: flow.id(source).to_owned(),
            target: flow.id(target).to_owned(),
            kind: kind.to_owned(),
            termination,
        })
        .collect();
    (edges, limit)
}

fn execution_edges(flow: &Flow<'_>, dominators: &Tree) -> Vec<ExecutionEdge> {
    flow.order()
        .into_iter()
        .filter(|&vertex| operation(flow, vertex))
        .filter_map(|vertex| {
            let source = dominators
                .parent(vertex)
                .filter(|&parent| operation(flow, parent))?;
            Some(ExecutionEdge {
                source: flow.id(source).to_owned(),
                target: flow.id(vertex).to_owned(),
            })
        })
        .collect()
}

fn loops(flow: &Flow<'_>) -> BTreeMap<String, Vec<String>> {
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (source, head) in flow.back_edges() {
        found
            .entry(flow.id(head).to_owned())
            .or_default()
            .push(flow.id(source).to_owned());
    }
    found
}
