use super::{DefinitionSet, Graph, NodeCategory, Operation, Topology, bits::Bits};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) struct Flow<'a> {
    pub pairs: Vec<(&'a str, &'a str)>,
    pub symbols: HashMap<&'a str, Vec<usize>>,
    pub sources: HashMap<&'a str, Option<&'a Operation>>,
    pub incoming: Vec<Bits>,
    pub outgoing: Vec<Bits>,
}

struct Evaluation<'a> {
    topology: &'a Topology,
    inputs: Bits,
    generators: Vec<Option<usize>>,
    root: usize,
}

struct Frontier {
    vertices: Vec<usize>,
    queued: Vec<bool>,
}

impl Frontier {
    fn new(size: usize) -> Self {
        Self {
            vertices: (0..size).collect(),
            queued: vec![true; size],
        }
    }

    fn clear(&mut self) {
        for &vertex in &self.vertices {
            self.queued[vertex] = false;
        }
        self.vertices.clear();
    }

    fn enqueue(&mut self, vertex: usize) {
        if !self.queued[vertex] {
            self.queued[vertex] = true;
            self.vertices.push(vertex);
        }
    }
}

fn formal_inputs(graph: &Graph) -> BTreeMap<&str, &str> {
    graph
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::FormalInput)
        .filter_map(|node| Some((node.symbol.as_deref()?, node.definition.as_deref()?)))
        .collect()
}

impl<'a> Flow<'a> {
    pub fn new(graph: &'a Graph, topology: &Topology, nodes: &[Option<&'a Operation>]) -> Self {
        let inputs = formal_inputs(graph);
        let pairs: BTreeSet<_> =
            inputs
                .iter()
                .map(|(&symbol, &definition)| (symbol, definition))
                .chain(nodes.iter().flatten().filter_map(|node| {
                    Some((node.symbol.as_deref()?, node.definition.as_deref()?))
                }))
                .collect();
        let pairs: Vec<_> = pairs.into_iter().collect();
        let mut result = Self {
            incoming: vec![Bits::empty(pairs.len()); topology.ids.len()],
            outgoing: vec![Bits::empty(pairs.len()); topology.ids.len()],
            pairs,
            symbols: HashMap::new(),
            sources: HashMap::new(),
        };
        result.index(graph);

        let inputs = result.inputs(inputs);
        result.solve(topology, nodes, inputs);
        result
    }

    fn index(&mut self, graph: &'a Graph) {
        for (index, &(symbol, _)) in self.pairs.iter().enumerate() {
            self.symbols.entry(symbol).or_default().push(index);
        }
        self.sources = source_lookup(graph, &self.pairs);
    }

    fn inputs(&self, inputs: BTreeMap<&str, &str>) -> Bits {
        let mut result = Bits::empty(self.pairs.len());
        for pair in inputs {
            result.insert(self.pairs.binary_search(&pair).expect("indexed input"));
        }
        result
    }

    fn generators(&self, nodes: &[Option<&Operation>]) -> Vec<Option<usize>> {
        nodes
            .iter()
            .map(|node| {
                let node = node.as_ref()?;
                let pair = (node.symbol.as_deref()?, node.definition.as_deref()?);
                Some(self.pairs.binary_search(&pair).expect("indexed definition"))
            })
            .collect()
    }

    fn solve(&mut self, topology: &Topology, nodes: &[Option<&Operation>], inputs: Bits) {
        let evaluation = Evaluation {
            topology,
            inputs,
            generators: self.generators(nodes),
            root: topology.indices["E"],
        };
        self.outgoing[evaluation.root] = evaluation.inputs.clone();
        let mut frontier = Frontier::new(topology.ids.len());
        while !frontier.vertices.is_empty() {
            let updates = self.evaluate(&evaluation, &frontier.vertices);
            frontier.clear();
            self.commit(topology, updates, &mut frontier);
        }
    }

    fn evaluate(&mut self, evaluation: &Evaluation<'_>, dirty: &[usize]) -> Vec<(usize, Bits)> {
        let mut updates = Vec::new();
        for &vertex in dirty {
            let incoming = self.join(evaluation, vertex);
            let outgoing = self.transfer(&incoming, evaluation.generators[vertex]);
            self.incoming[vertex] = incoming;
            if self.outgoing[vertex] != outgoing {
                updates.push((vertex, outgoing));
            }
        }
        updates
    }

    fn commit(&mut self, graph: &Topology, updates: Vec<(usize, Bits)>, frontier: &mut Frontier) {
        for (vertex, outgoing) in updates {
            self.outgoing[vertex] = outgoing;
            for &target in &graph.successors[vertex] {
                frontier.enqueue(target);
            }
        }
    }

    fn join(&self, evaluation: &Evaluation<'_>, vertex: usize) -> Bits {
        if vertex == evaluation.root {
            return evaluation.inputs.clone();
        }
        let mut incoming = Bits::empty(self.pairs.len());
        for &source in &evaluation.topology.predecessors[vertex] {
            incoming.union(&self.outgoing[source]);
        }
        incoming
    }

    fn transfer(&self, incoming: &Bits, generator: Option<usize>) -> Bits {
        let mut result = incoming.clone();
        if let Some(generator) = generator {
            for &definition in &self.symbols[self.pairs[generator].0] {
                result.remove(definition);
            }
            result.insert(generator);
        }
        result
    }

    pub fn materialize(
        &self,
        graph: &Topology,
        states: &[Bits],
    ) -> BTreeMap<String, DefinitionSet> {
        graph
            .ids
            .iter()
            .zip(states)
            .map(|(id, state)| {
                let mut definitions = DefinitionSet::new();
                for pair in state.members() {
                    let (symbol, definition) = self.pairs[pair];
                    definitions
                        .entry(symbol.to_owned())
                        .or_default()
                        .insert(definition.to_owned());
                }
                (id.clone(), definitions)
            })
            .collect()
    }
}

fn source_lookup<'a>(
    graph: &'a Graph,
    pairs: &[(&'a str, &'a str)],
) -> HashMap<&'a str, Option<&'a Operation>> {
    let mut sources: HashMap<_, _> = pairs
        .iter()
        .map(|&(_, definition)| (definition, None))
        .collect();
    let mut remaining = sources.len();
    for node in &graph.nodes {
        if remaining == 0 {
            break;
        }
        let Some(slot) = node
            .definition
            .as_deref()
            .and_then(|definition| sources.get_mut(definition))
        else {
            continue;
        };
        if slot.is_none() {
            *slot = Some(node);
            remaining -= 1;
        }
    }
    sources
}
