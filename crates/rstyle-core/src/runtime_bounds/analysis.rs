use super::*;
use std::collections::{BTreeSet, VecDeque};
mod loops;
mod values;

pub(super) struct Graph {
    successors: Vec<Vec<usize>>,
    predecessors: Vec<Vec<usize>>,
    reachable: Vec<bool>,
    dominators: Vec<Vec<bool>>,
}

pub(super) struct Loop {
    header: usize,
    members: BTreeSet<usize>,
    counter: u32,
    limit: Value,
    increment: usize,
    // Assume mode: why this loop is bounded by an opaque factor.
    opaque: Option<String>,
}

pub(super) enum Failure {
    Unknown(String),
    Budget(String),
}

type Checked<T> = Result<T, Failure>;

pub(super) fn unknown(reason: &str) -> Failure {
    Failure::Unknown(reason.into())
}

pub(super) fn analyze(program: &Program, limits: Limits) -> Outcome {
    match assess(program, limits) {
        Ok(assessment) => Outcome::Proven(assessment),
        Err(Failure::Unknown(reason)) => Outcome::Unknown { reason },
        Err(Failure::Budget(reason)) => Outcome::BudgetExceeded { reason },
    }
}

fn assess(program: &Program, limits: Limits) -> Checked<Assessment> {
    validate(program, limits)?;
    let graph = Graph::new(program)?;
    let graph = graph.dominance(program.entry as usize, limits.iterations)?;
    let mut loops = loops::discover(program, &graph, limits.opaque_loops)?;
    loops::validate(program, &graph, &mut loops, limits.opaque_loops)?;

    let bounds = loops::bounds(program, &graph, &mut loops, limits)?;

    let site = Site {
        program,
        graph: &graph,
        loops: &loops,
    };
    let calls = call_bounds(site, &bounds);
    let loop_body_entries = Bound::add(bounds.clone());
    let expression_size = loop_body_entries.size()
        + bounds.iter().map(Bound::size).sum::<usize>()
        + calls.iter().map(|call| call.count.size()).sum::<usize>();
    if expression_size > limits.expression_nodes {
        return Err(Failure::Budget("Symbolic bound budget exceeded".into()));
    }

    Ok(Assessment {
        cost_scope: "loop-body-entries-and-modeled-call-counts".into(),
        loop_body_entries,
        loops: loops
            .iter()
            .zip(bounds)
            .map(|(item, body_entries)| LoopBound {
                header: item.header as u32,
                source: program.nodes[item.header].source.clone(),
                body_entries,
                opaque: item.opaque.clone(),
            })
            .collect(),
        calls,
        whole_runtime: "unknown: loop and call counts do not bound operation costs".into(),
    })
}

fn validate(program: &Program, limits: Limits) -> Checked<()> {
    if program.nodes.len() > limits.nodes || program.variables.len() > limits.variables {
        return Err(Failure::Budget("Control-flow input budget exceeded".into()));
    }
    if program.nodes.is_empty() || program.entry as usize >= program.nodes.len() {
        return Err(unknown("Missing control-flow entry"));
    }

    let inputs: BTreeSet<_> = program.inputs.iter().collect();
    if inputs.len() != program.inputs.len() || inputs.iter().any(|name| name.is_empty()) {
        return Err(unknown("Input identities must be unique and nonempty"));
    }
    if program
        .variables
        .iter()
        .any(|v| !(1..=64).contains(&v.bits))
    {
        return Err(unknown(
            "Only captured unsigned widths from 1 to 64 are supported",
        ));
    }

    Ok(())
}

impl Graph {
    fn new(program: &Program) -> Checked<Self> {
        let mut successors = Vec::new();
        let mut predecessors = vec![Vec::new(); program.nodes.len()];
        for (index, node) in program.nodes.iter().enumerate() {
            let targets = match &node.flow {
                Flow::Goto(next) => vec![*next as usize],
                Flow::Branch {
                    then_node,
                    else_node,
                    ..
                } => vec![*then_node as usize, *else_node as usize],
                Flow::Return => Vec::new(),
            };
            for &target in &targets {
                if target >= program.nodes.len() {
                    return Err(unknown("Missing CFG successor"));
                }
                predecessors[target].push(index);
            }
            successors.push(targets);
        }
        let reachable = reach(&successors, program.entry as usize);

        Ok(Self {
            successors,
            predecessors,
            reachable,
            dominators: Vec::new(),
        })
    }

    fn dominance(mut self, entry: usize, budget: usize) -> Checked<Self> {
        self.dominators = (0..self.successors.len())
            .map(|row| {
                (0..self.successors.len())
                    .map(|column| {
                        if row == entry {
                            column == entry
                        } else {
                            self.reachable[column]
                        }
                    })
                    .collect()
            })
            .collect();
        let mut pending: VecDeque<_> = (0..self.successors.len())
            .filter(|&i| self.reachable[i] && i != entry)
            .collect();
        let mut visits = 0;
        while let Some(index) = pending.pop_front() {
            visits += 1;
            if visits > budget {
                return Err(Failure::Budget("Dominance work budget exceeded".into()));
            }
            let next = self.intersection(index);
            if next != self.dominators[index] {
                self.dominators[index] = next;
                pending.extend(
                    self.successors[index]
                        .iter()
                        .copied()
                        .filter(|&i| i != entry),
                );
            }
        }

        Ok(self)
    }

    fn intersection(&self, index: usize) -> Vec<bool> {
        let mut result = self.reachable.clone();
        for &predecessor in &self.predecessors[index] {
            if self.reachable[predecessor] {
                for (bit, incoming) in result.iter_mut().zip(&self.dominators[predecessor]) {
                    *bit &= incoming;
                }
            }
        }
        result[index] = true;
        result
    }
}

fn reach(successors: &[Vec<usize>], entry: usize) -> Vec<bool> {
    let mut reached = vec![false; successors.len()];

    let mut pending = vec![entry];
    while let Some(index) = pending.pop() {
        if reached[index] {
            continue;
        }
        reached[index] = true;
        pending.extend(&successors[index]);
    }
    reached
}

fn call_bounds(site: Site<'_>, bounds: &[Bound]) -> Vec<CallBound> {
    site.program
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, _)| site.graph.reachable[*index])
        .filter_map(|(index, node)| call_bound(site, index, node, bounds))
        .collect()
}

fn call_bound(site: Site<'_>, index: usize, node: &Node, bounds: &[Bound]) -> Option<CallBound> {
    let Action::Call {
        callee,
        target,
        arguments,
        ..
    } = &node.action
    else {
        return None;
    };
    let loops = site.loops;
    let enclosing = loops
        .iter()
        .enumerate()
        .filter(|(_, item)| item.members.contains(&index))
        .min_by_key(|(_, item)| item.members.len());

    Some(CallBound {
        node: index as u32,
        callee: callee.clone(),
        target: target.clone(),
        source: node.source.clone(),
        count: enclosing.map_or(Bound::Const(1), |(i, _)| bounds[i].clone()),
        arguments: arguments
            .iter()
            .map(|argument| site.argument(index, argument))
            .collect(),
    })
}

#[derive(Clone, Copy)]
struct Site<'a> {
    program: &'a Program,
    graph: &'a Graph,
    loops: &'a [Loop],
}

impl Site<'_> {
    fn argument(&self, index: usize, argument: &Argument) -> ArgumentBound {
        match argument {
            Argument::Scalar(value) => self
                .scalar(index, value)
                .map_or(ArgumentBound::Unknown, ArgumentBound::Scalar),
            Argument::Place { root, fields } => ArgumentBound::Place {
                root: *root,
                fields: fields.clone(),
            },
            Argument::Opaque => ArgumentBound::Unknown,
        }
    }

    // Inside its cycle a guarded unit-step counter never exceeds the loop limit.
    fn scalar(&self, index: usize, value: &Value) -> Option<Bound> {
        if let Value::Variable(id) = value
            && let Some(item) = self.loops.iter().find(|item| {
                item.opaque.is_none()
                    && item.counter == *id
                    && item.header != index
                    && item.members.contains(&index)
            })
        {
            let bits = self.program.variables.get(*id as usize)?.bits;
            let at_header = values::Values::new(self.program, self.graph, item.header, bits);
            return at_header.bound(&item.limit).ok();
        }
        values::Values::new(self.program, self.graph, index, 64)
            .bound(value)
            .ok()
    }
}
