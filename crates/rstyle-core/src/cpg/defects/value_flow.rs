//! Where a function's values go, for any frontend that emits its flow and accesses: from each call's
//! result and each parameter, to each call's arguments and what it returns, by what a tree's operands
//! and reaching definitions show.
//!
//! Each step stops at the next call. What a call does with what it's given is a question for
//! whoever reads these, since the answer depends on what a project says of that callee.

use super::super::model::{
    Access, AccessMode, Definition, Dispatch, Node, NodeKind, Scope, TypeKind,
};
use super::super::tree;
use super::Reading;
use super::ignored_result::declared_callee;
use crate::paths::strip_generics;
use std::collections::{BTreeSet, HashMap};

/// A call a flow can start or end at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowCall {
    /// The callee as it's declared, which is what a contract names.
    pub identity: String,
    /// The implementation static selection finds, which is whose body says what it does.
    pub selected: String,
    pub name: String,
    /// What a frontend's contract calls it, with its namespace.
    pub operation: Option<String>,
    pub line: usize,
    /// The text of each argument that's a string literal, by position: a receiver is 0.
    pub literals: Vec<(usize, String)>,
}

/// Where a value starts. Parameters number as a call's arguments do: a receiver is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Start {
    Result(usize),
    Parameter(usize),
}

/// Where a step of a value ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum End {
    Argument { call: usize, position: usize },
    Returned,
}

/// One step of a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hop {
    pub from: Start,
    pub to: End,
}

/// Where a function's values go, in its frontend's namespace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueFlows {
    /// The function, as a call to it names it.
    pub identity: String,
    pub namespace: String,
    pub calls: Vec<FlowCall>,
    pub hops: Vec<Hop>,
}

/// The directed graph values take between a function's tree nodes, each step ending at a call.
struct Graph<'a> {
    reading: &'a Reading<'a>,
    accesses: HashMap<&'a str, Vec<&'a Access>>,
    successors: HashMap<&'a str, Vec<&'a str>>,
    /// Each call's argument nodes, by position, and the nodes it can write.
    arguments: HashMap<&'a str, Vec<(usize, &'a str)>>,
    written_by_calls: HashMap<&'a str, Vec<&'a str>>,
}

/// How many pointers and references a type is looked through. Far more than a type is written.
const POINTER_DEPTH: usize = 8;

/// A number, a float or a truth value can't carry text, whatever it was computed from, though a
/// pointer to one can: C's text is a `char *`. A value of this code's own types might hold some in a
/// field, but which field is what matters: `cfg.client` coming from a request says nothing of
/// `cfg.path`. So they aren't followed through.
fn carries_no_text(ty: Option<&str>, reading: &Reading<'_>) -> bool {
    let tables = reading.tables;
    let mut at = ty.and_then(|id| tables.ty(id).ok());
    let mut behind_a_pointer = false;

    for _ in 0..POINTER_DEPTH {
        let Some(ty) = at else {
            return false;
        };
        match ty.kind {
            TypeKind::Integer | TypeKind::Float | TypeKind::Bool => return !behind_a_pointer,
            TypeKind::Record | TypeKind::Sum | TypeKind::Interface => {
                let definition = tables.named_definition(ty.definition.as_deref());
                return definition.is_some_and(|it| it.scope != Scope::Library);
            }
            TypeKind::Pointer | TypeKind::Reference => {
                behind_a_pointer |= ty.kind == TypeKind::Pointer;
                at = ty.target.as_deref().and_then(|id| tables.ty(id).ok());
            }
            _ => return false,
        }
    }
    false
}

/// The calling convention both sides of a contract use: a receiver is 0, and otherwise the first
/// argument is, after the callee.
fn operands<'a>(reading: &Reading<'a>, call: &Node) -> Result<Vec<(usize, &'a Node)>, String> {
    let dispatch = reading.body.reference(&call.id).and_then(|r| r.dispatch);
    let has_receiver = matches!(dispatch, Some(Dispatch::Method | Dispatch::Dynamic));
    let children = reading.body.children(call)?;
    let arguments = children.into_iter().skip(usize::from(!has_receiver));

    Ok(arguments.enumerate().collect())
}

impl<'a> Graph<'a> {
    fn new(reading: &'a Reading<'a>) -> Self {
        let mut accesses: HashMap<&str, Vec<&Access>> = HashMap::new();
        for access in &reading.function.accesses {
            accesses.entry(&access.node).or_default().push(access);
        }
        Self {
            reading,
            accesses,
            successors: HashMap::new(),
            arguments: HashMap::new(),
            written_by_calls: HashMap::new(),
        }
    }

    fn link(&mut self, from: &'a str, to: &'a str) {
        self.successors.entry(from).or_default().push(to);
    }

    fn writes(&self, node: &str) -> bool {
        self.accesses
            .get(node)
            .is_some_and(|found| found.iter().any(|a| a.mode != AccessMode::Read))
    }

    /// A node a value can't go into: one holding a number, or a binding that does.
    fn holds_no_text(&self, node: &Node) -> Result<bool, String> {
        let body = &self.reading.body;
        let written = self.accesses.get(node.id.as_str()).into_iter().flatten();
        for access in written {
            let symbol = body.symbol(&access.symbol)?;
            if carries_no_text(symbol.ty.as_deref(), self.reading) {
                return Ok(true);
            }
        }
        Ok(carries_no_text(node.ty.as_deref(), self.reading))
    }

    /// The nodes under a target or a pattern that a store writes.
    fn written_within(&self, node: &'a Node) -> Result<Vec<&'a str>, String> {
        let mut found = Vec::new();
        let mut stack = vec![node];
        while let Some(at) = stack.pop() {
            if self.writes(&at.id) {
                found.push(at.id.as_str());
            }
            stack.extend(self.reading.body.children(at)?);
        }
        Ok(found)
    }

    fn store(&mut self, value: &'a Node, target: &'a Node) -> Result<(), String> {
        for written in self.written_within(target)? {
            self.link(&value.id, written);
        }
        Ok(())
    }

    /// What a node's children give it, and what a store does with them.
    fn add_node(&mut self, node: &'a Node) -> Result<(), String> {
        let children = self.reading.body.children(node)?;
        self.add_values(node, &children)?;

        match node.kind {
            NodeKind::Match => self.add_match(node, &children),
            NodeKind::Call => self.add_call(node),
            _ => Ok(()),
        }
    }

    fn add_values(&mut self, node: &'a Node, children: &[&'a Node]) -> Result<(), String> {
        for child in value_children(node, children) {
            self.link(&child.id, &node.id);
        }
        match stored(node, children) {
            Some((value, target)) => self.store(value, target),
            None => Ok(()),
        }
    }

    fn add_match(&mut self, node: &'a Node, children: &[&'a Node]) -> Result<(), String> {
        let Some((scrutinee, arms)) = children.split_first() else {
            return Ok(());
        };
        for arm in arms {
            self.link(&arm.id, &node.id);
            if let Some(pattern) = self.reading.body.children(arm)?.first() {
                self.store(scrutinee, pattern)?;
            }
        }
        Ok(())
    }

    /// A call is where a step ends. What it's handed that it can write is where its value goes as
    /// well as its result: `b.WriteString(x)` leaves `x` in `b`.
    fn add_call(&mut self, node: &'a Node) -> Result<(), String> {
        let operands = operands(self.reading, node)?;
        let mut written = Vec::new();
        for (_, operand) in &operands {
            written.extend(self.written_through(operand)?);
        }

        self.written_by_calls.insert(&node.id, written);
        self.arguments.insert(
            &node.id,
            operands
                .iter()
                .map(|(at, o)| (*at, o.id.as_str()))
                .collect(),
        );
        Ok(())
    }

    /// The node an operand writes, looking through what only points at it.
    fn written_through(&self, operand: &'a Node) -> Result<Option<&'a str>, String> {
        let mut at = operand;
        loop {
            if self.writes(&at.id) {
                return Ok(Some(at.id.as_str()));
            }
            let through = matches!(at.kind, NodeKind::Reference | NodeKind::FieldAccess)
                || (at.kind == NodeKind::Operator && at.operator.as_deref() == Some("deref"));
            if !through || at.children.is_empty() {
                return Ok(None);
            }
            at = self.reading.body.child(at, 0)?;
        }
    }

    /// Each read from each write that reaches it. A read or write off the flow, as a closure's are,
    /// can't be placed before or after anything, so it meets every other of its symbol.
    fn add_reaching(&mut self) {
        let flow = &self.reading.flow;
        let entry = flow.entry();
        let parameters = parameter_symbols(self.reading);
        for &(write, read, symbol) in &self.reading.dependence {
            if write != entry {
                self.link(flow.id(write), flow.id(read));
            } else if parameters.iter().any(|(_, parameter)| *parameter == symbol) {
                self.link(symbol, flow.id(read));
            }
        }

        let accesses = &self.reading.function.accesses;
        let off_flow = accesses.iter().filter(|a| flow.vertex(&a.node).is_none());
        let elsewhere: Vec<(&'a str, &'a str)> = off_flow
            .flat_map(|off| {
                accesses
                    .iter()
                    .filter(move |other| other.symbol == off.symbol && other.node != off.node)
                    .filter_map(move |other| match (other.mode, off.mode) {
                        (AccessMode::Read, AccessMode::Read) => None,
                        (AccessMode::Read, _) => Some((off.node.as_str(), other.node.as_str())),
                        (_, AccessMode::Read) => Some((other.node.as_str(), off.node.as_str())),
                        _ => Some((other.node.as_str(), off.node.as_str())),
                    })
            })
            .collect();
        for (from, to) in elsewhere {
            self.link(from, to);
        }
    }

    /// Every argument this start reaches before the next call takes it over.
    fn reached(&self, starts: &[&'a str]) -> Result<BTreeSet<&'a str>, String> {
        let body = &self.reading.body;
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut stack: Vec<&str> = starts.to_vec();

        while let Some(at) = stack.pop() {
            for &next in self.successors.get(at).into_iter().flatten() {
                if !seen.contains(next) && !self.holds_no_text(body.node(next)?)? {
                    seen.insert(next);
                    stack.push(next);
                }
            }
        }
        Ok(seen)
    }
}

/// The children whose value is part of a node's.
fn value_children<'a>(node: &Node, children: &[&'a Node]) -> Vec<&'a Node> {
    match node.kind {
        NodeKind::FieldAccess
        | NodeKind::IndexAccess
        | NodeKind::Operator
        | NodeKind::Cast
        | NodeKind::Tuple
        | NodeKind::Array
        | NodeKind::RecordConstruction
        | NodeKind::Reference
        | NodeKind::Propagate
        | NodeKind::Extension => children.to_vec(),
        NodeKind::Block | NodeKind::MatchArm => {
            let last = children.last().filter(|last| holds_a_value(last));
            last.into_iter().copied().collect()
        }
        NodeKind::If => {
            let branches = children
                .iter()
                .skip(1)
                .filter(|branch| holds_a_value(branch));
            branches.copied().collect()
        }
        _ => Vec::new(),
    }
}

/// What a node stores, and in what.
fn stored<'a>(node: &Node, children: &[&'a Node]) -> Option<(&'a Node, &'a Node)> {
    match node.kind {
        NodeKind::LocalDeclaration => {
            let (pattern, rest) = children.split_first()?;
            let value = rest.iter().find(|it| it.kind != NodeKind::TypeAnnotation)?;
            Some((*value, *pattern))
        }
        NodeKind::Assignment
        | NodeKind::CompoundAssignment
        | NodeKind::DestructuringAssignment
        | NodeKind::For
        | NodeKind::PatternCondition => match children {
            [target, value, ..] => Some((*value, *target)),
            _ => None,
        },
        _ => None,
    }
}

/// What a function's parameters are bound to, numbered as a call's arguments are.
fn parameter_symbols<'a>(reading: &Reading<'a>) -> Vec<(usize, &'a str)> {
    let signature = &reading.function.signature;
    let receiver = signature.receiver.iter().map(|it| it.symbol.as_str());
    let parameters = signature.parameters.iter().map(|it| it.symbol.as_str());

    receiver.chain(parameters).enumerate().collect()
}

/// The nodes whose value a caller gets: what's returned, what a body ends with, and what's written
/// to a result that has a name.
fn returned_nodes<'a>(reading: &Reading<'a>) -> Result<Vec<&'a str>, String> {
    let function = reading.function;
    let parameters = parameter_symbols(reading);
    let named_result = |symbol: &str| {
        let declared = reading
            .body
            .symbol(symbol)
            .is_ok_and(|it| it.node.is_none());
        declared && !parameters.iter().any(|(_, parameter)| *parameter == symbol)
    };
    let written_to_results = function
        .accesses
        .iter()
        .filter(|access| access.mode != AccessMode::Read && named_result(&access.symbol))
        .map(|access| access.node.as_str());
    let returned = function
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Return)
        .filter_map(|node| node.children.first())
        .map(String::as_str);

    Ok(returned
        .chain(written_to_results)
        .chain(std::iter::once(function.root.as_str()))
        .collect())
}

/// Kinds that are statements or the parts of one, and so give whatever holds them no value.
fn holds_a_value(node: &Node) -> bool {
    !matches!(
        node.kind,
        NodeKind::LocalDeclaration
            | NodeKind::ExpressionStatement
            | NodeKind::ItemDeclaration
            | NodeKind::Return
            | NodeKind::Break
            | NodeKind::Continue
            | NodeKind::While
            | NodeKind::For
            | NodeKind::Loop
            | NodeKind::TypeAnnotation
    )
}

/// A string literal's text, which is all a contract can compare an argument against.
fn literal_text(node: &Node) -> Option<String> {
    let literal = node.value.as_ref()?;
    (literal.kind == "string").then(|| literal.value.clone())?
}

fn flow_call(
    reading: &Reading<'_>,
    node: &Node,
    definition: &Definition,
) -> Result<FlowCall, String> {
    let literals = operands(reading, node)?
        .into_iter()
        .filter_map(|(at, operand)| Some((at, literal_text(operand)?)))
        .collect();
    let selected = reading
        .body
        .reference(&node.id)
        .and_then(|reference| reference.selected.as_ref())
        .and_then(|selected| selected.id.as_deref())
        .and_then(|id| reading.tables.definition(id).ok())
        .unwrap_or(definition);
    Ok(FlowCall {
        identity: definition.stable_identity.clone(),
        selected: selected.stable_identity.clone(),
        name: strip_generics(&definition.name),
        operation: definition.operation.clone(),
        line: tree::line(reading.sources, node)?,
        literals,
    })
}

/// Each call that can start or end a flow, by its node.
fn flow_calls<'a>(
    reading: &Reading<'a>,
) -> Result<(Vec<FlowCall>, HashMap<&'a str, usize>), String> {
    let mut calls = Vec::new();
    let mut positions = HashMap::new();
    let nodes = reading.function.nodes.iter();

    for node in nodes.filter(|node| node.kind == NodeKind::Call) {
        if let Some(definition) = declared_callee(reading, node)? {
            positions.insert(node.id.as_str(), calls.len());
            calls.push(flow_call(reading, node, definition)?);
        }
    }
    Ok((calls, positions))
}

/// Where each call's result can go next. A call whose result holds no text gives nothing on, though
/// it can still write what it's handed.
fn starts_of<'a>(
    graph: &Graph<'a>,
    positions: &HashMap<&'a str, usize>,
) -> Result<Vec<(Start, Vec<&'a str>)>, String> {
    let mut starts = Vec::new();
    for (&node, &call) in positions {
        let written = graph
            .written_by_calls
            .get(node)
            .cloned()
            .unwrap_or_default();
        let result = (!graph.holds_no_text(graph.reading.body.node(node)?)?).then_some(node);
        starts.push((
            Start::Result(call),
            result.into_iter().chain(written).collect(),
        ));
    }
    for (position, symbol) in parameter_symbols(graph.reading) {
        starts.push((Start::Parameter(position), vec![symbol]));
    }
    Ok(starts)
}

/// Every step from one start.
fn hops_from<'a>(
    graph: &Graph<'a>,
    (start, nodes): (Start, &[&'a str]),
    (positions, returned): (&HashMap<&'a str, usize>, &[&'a str]),
) -> Result<Vec<Hop>, String> {
    let reached = graph.reached(nodes)?;
    let holds = |node: &&str| reached.contains(*node) || nodes.contains(node);
    let mut found = Vec::new();

    for (&target, arguments) in &graph.arguments {
        let Some(&call) = positions.get(target) else {
            continue;
        };
        let held = arguments.iter().filter(|(_, argument)| holds(argument));
        for &(position, _) in held.filter(|_| start != Start::Result(call)) {
            let to = End::Argument { call, position };
            found.push(Hop { from: start, to });
        }
    }
    if returned.iter().any(holds) {
        found.push(Hop {
            from: start,
            to: End::Returned,
        });
    }
    Ok(found)
}

/// Where a function's values go.
pub(super) fn of<'a>(reading: &'a Reading<'a>, namespace: &str) -> Result<ValueFlows, String> {
    let mut graph = Graph::new(reading);
    for node in &reading.function.nodes {
        graph.add_node(node)?;
    }
    graph.add_reaching();

    let (calls, positions) = flow_calls(reading)?;
    let returned = returned_nodes(reading)?;
    let mut hops = BTreeSet::new();
    for (start, nodes) in starts_of(&graph, &positions)? {
        hops.extend(hops_from(&graph, (start, &nodes), (&positions, &returned))?);
    }

    Ok(ValueFlows {
        identity: reading
            .tables
            .definition(&reading.function.definition)?
            .stable_identity
            .clone(),
        namespace: namespace.to_owned(),
        calls,
        hops: hops.into_iter().collect(),
    })
}
