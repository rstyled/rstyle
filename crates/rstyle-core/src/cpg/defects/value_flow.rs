//! Where a function's values go, for any frontend that emits its flow and accesses: from each call's
//! result and each parameter, to each call's arguments and what it returns, by what a tree's operands
//! and reaching definitions show.
//!
//! Each step stops at the next call. What a call does with what it's given is a question for
//! whoever reads these, since the answer depends on what a project says of that callee.

use super::super::model::{
    Access, AccessMode, Definition, DefinitionKind, Node, NodeKind, RefStatus, Scope, TargetKind,
    TypeKind,
};
use super::super::tree::{self, Argument};
use super::Reading;
use super::ignored_result::{declared_callee, returns_its_receiver};
use crate::paths::strip_generics;
use std::collections::{BTreeSet, HashMap};

/// An argument that's a list written out in a call: its position, and the text of each element
/// that's a string literal.
pub type WrittenList = (usize, Vec<Option<String>>);

/// A call a flow can start or end at, or a library's variable one can start at: reading it is where
/// its value comes from, as a call's result is, and it has no arguments.
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
    /// The literal each argument passes, by the name of the parameter it binds to.
    pub named: Vec<(String, String)>,
    /// Each argument that's a list written out in the call.
    pub lists: Vec<WrittenList>,
    /// The position of each argument whose type is a string, literal or not.
    pub strings: Vec<usize>,
}

/// Where a value starts. Parameters number as a call's arguments do: a receiver is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Start {
    Result(usize),
    Parameter(usize),
}

/// Where a step of a value ends: an argument, and which element of it when it's a list written out
/// in the call, or an argument whose position isn't known, as a spread's isn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum End {
    Argument {
        call: usize,
        position: usize,
        element: Option<usize>,
    },
    Unplaced {
        call: usize,
    },
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

/// Where in a call a value can end: a position, when it's known, the element of a list written out
/// there, and the node that holds it.
type Slot<'a> = (Option<usize>, Option<usize>, &'a str);

/// The directed graph values take between a function's tree nodes, each step ending at a call.
struct Graph<'a> {
    reading: &'a Reading<'a>,
    accesses: HashMap<&'a str, Vec<&'a Access>>,
    successors: HashMap<&'a str, Vec<&'a str>>,
    /// Each call's argument nodes, by position, and the nodes it can write.
    arguments: HashMap<&'a str, Vec<Slot<'a>>>,
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
/// argument is, after the callee. An argument is numbered by the parameter it binds to.
fn operands<'a>(reading: &Reading<'a>, call: &Node) -> Result<Vec<Argument<'a>>, String> {
    reading.body.arguments(call)
}

/// Each argument with a position, by it.
fn placed<'o, 'a>(operands: &'o [Argument<'a>]) -> impl Iterator<Item = (usize, &'a Node)> + 'o {
    operands
        .iter()
        .filter_map(|argument| Some((argument.position?, argument.value)))
}

/// The elements of a list written out where it's passed, as `args(["-c", script])` passes one,
/// looking through a borrow of it.
fn written_out_list<'a>(
    reading: &Reading<'a>,
    passed: &'a Node,
) -> Result<Option<Vec<&'a Node>>, String> {
    let mut at = passed;
    while at.kind == NodeKind::Reference && !at.children.is_empty() {
        at = reading.body.child(at, 0)?;
    }
    if at.kind != NodeKind::Array {
        return Ok(None);
    }
    reading.body.children(at).map(Some)
}

/// Where in each argument a value can end: the argument, or each element of a list written out. An
/// argument with no position has no elements anyone can count.
fn slots<'a>(reading: &Reading<'a>, operands: &[Argument<'a>]) -> Result<Vec<Slot<'a>>, String> {
    let mut slots = Vec::new();
    for argument in operands {
        let position = argument.position;
        let elements = match position {
            Some(_) => written_out_list(reading, argument.value)?,
            None => None,
        };
        let Some(elements) = elements else {
            slots.push((position, None, argument.node.id.as_str()));
            continue;
        };
        let elements = elements.into_iter().enumerate();
        slots.extend(elements.map(|(at, element)| (position, Some(at), element.id.as_str())));
    }
    Ok(slots)
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
        for argument in &operands {
            written.extend(self.written_through(argument.value)?);
        }

        self.written_by_calls.insert(&node.id, written);
        self.arguments
            .insert(&node.id, slots(self.reading, &operands)?);
        Ok(())
    }

    /// Whether a call gives back a reference to what it was called on, as a builder's calls do.
    fn lends_its_receiver(&self, call: &Node) -> Result<bool, String> {
        let Some(ty) = call.ty.as_deref() else {
            return Ok(false);
        };
        let ty = self.reading.tables.ty(ty)?;
        let by_reference = matches!(ty.kind, TypeKind::Reference | TypeKind::Pointer);
        Ok(by_reference && returns_its_receiver(self.reading, call, ty)?)
    }

    /// The node an operand writes, looking through what only points at it, which a call that
    /// lends its receiver does: `cmd.arg("-c").arg(script)` writes `cmd` twice.
    fn written_through(&self, operand: &'a Node) -> Result<Option<&'a str>, String> {
        let mut at = operand;
        loop {
            if self.writes(&at.id) {
                return Ok(Some(at.id.as_str()));
            }
            let through = matches!(at.kind, NodeKind::Reference | NodeKind::FieldAccess)
                || (at.kind == NodeKind::Operator && at.operator.as_deref() == Some("deref"))
                || (at.kind == NodeKind::Call && self.lends_its_receiver(at)?);
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

/// The literal an argument passes to a parameter it names, as Python's `shell=True` does, whether
/// it's passed by keyword or by position.
fn named_literal(argument: &Argument<'_>) -> Option<(String, String)> {
    let value = argument.value;
    let literal = value
        .value
        .as_ref()
        .filter(|_| value.kind == NodeKind::Literal);
    Some((argument.parameter?.to_owned(), literal?.value.clone()?))
}

/// The text of each element of each list written out in a call, where it's a string literal.
fn lists_of(reading: &Reading<'_>, operands: &[Argument<'_>]) -> Result<Vec<WrittenList>, String> {
    let mut lists = Vec::new();
    for (position, passed) in placed(operands) {
        if let Some(elements) = written_out_list(reading, passed)? {
            lists.push((position, elements.into_iter().map(literal_text).collect()));
        }
    }
    Ok(lists)
}

/// The position of each argument a string is passed for, which a contract can tell from a list.
fn strings_of(reading: &Reading<'_>, operands: &[Argument<'_>]) -> Vec<usize> {
    let mut strings = Vec::new();
    for (position, passed) in placed(operands) {
        let ty = passed.ty.as_deref();
        let ty = ty.and_then(|id| reading.tables.ty(id).ok());
        if ty.is_some_and(|ty| ty.kind == TypeKind::String) {
            strings.push(position);
        }
    }
    strings
}

fn flow_call(
    reading: &Reading<'_>,
    node: &Node,
    definition: &Definition,
) -> Result<FlowCall, String> {
    let operands = operands(reading, node)?;
    let literals = placed(&operands)
        .filter_map(|(at, passed)| Some((at, literal_text(passed)?)))
        .collect();
    let named = operands.iter().filter_map(named_literal).collect();

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
        named,
        lists: lists_of(reading, &operands)?,
        strings: strings_of(reading, &operands),
    })
}

/// The library variable a node reads, as `os.environ` is one.
fn library_variable<'r>(
    reading: &Reading<'r>,
    node: &Node,
) -> Result<Option<&'r Definition>, String> {
    let reference = reading.body.reference(&node.id);
    let resolved = reference.filter(|it| it.status == RefStatus::Resolved);
    let target = resolved.and_then(|it| it.target.as_ref());
    let defined = target.filter(|it| it.kind == TargetKind::Definition);
    let Some(id) = defined.and_then(|it| it.id.as_deref()) else {
        return Ok(None);
    };

    let definition = reading.tables.definition(id)?;
    let variable = definition.kind == DefinitionKind::Static && definition.scope == Scope::Library;
    Ok(variable.then_some(definition))
}

/// A read of a library's variable, as a flow sees it: a call with no arguments.
fn variable_read(
    reading: &Reading<'_>,
    node: &Node,
    definition: &Definition,
) -> Result<FlowCall, String> {
    Ok(FlowCall {
        identity: definition.stable_identity.clone(),
        selected: definition.stable_identity.clone(),
        name: strip_generics(&definition.name),
        operation: definition.operation.clone(),
        line: tree::line(reading.sources, node)?,
        literals: Vec::new(),
        named: Vec::new(),
        lists: Vec::new(),
        strings: Vec::new(),
    })
}

/// What a node starts or ends a flow as: a call, or a read of a library's variable.
fn flow_call_at(reading: &Reading<'_>, node: &Node) -> Result<Option<FlowCall>, String> {
    if node.kind == NodeKind::Call {
        let Some(definition) = declared_callee(reading, node)? else {
            return Ok(None);
        };
        return flow_call(reading, node, definition).map(Some);
    }
    let Some(definition) = library_variable(reading, node)? else {
        return Ok(None);
    };
    variable_read(reading, node, definition).map(Some)
}

/// Each call that can start or end a flow, and each read of a library's variable, by its node.
fn flow_calls<'a>(
    reading: &Reading<'a>,
) -> Result<(Vec<FlowCall>, HashMap<&'a str, usize>), String> {
    let mut calls = Vec::new();
    let mut positions = HashMap::new();

    for node in &reading.function.nodes {
        if let Some(call) = flow_call_at(reading, node)? {
            positions.insert(node.id.as_str(), calls.len());
            calls.push(call);
        }
    }
    Ok((calls, positions))
}

/// The nodes a value starts in: what holds it, and what a call writes with it.
struct StartNodes<'a> {
    start: Start,
    value: Vec<&'a str>,
    written: Vec<&'a str>,
}

/// Where each call's result can go next. A call whose result holds no text gives nothing on, though
/// it can still write what it's handed.
fn starts_of<'a>(
    graph: &Graph<'a>,
    positions: &HashMap<&'a str, usize>,
) -> Result<Vec<StartNodes<'a>>, String> {
    let mut starts = Vec::new();
    for (&node, &call) in positions {
        let written = graph
            .written_by_calls
            .get(node)
            .cloned()
            .unwrap_or_default();
        let result = (!graph.holds_no_text(graph.reading.body.node(node)?)?).then_some(node);
        starts.push(StartNodes {
            start: Start::Result(call),
            value: result.into_iter().collect(),
            written,
        });
    }
    for (position, symbol) in parameter_symbols(graph.reading) {
        let value = vec![symbol];
        let start = Start::Parameter(position);
        starts.push(StartNodes {
            start,
            value,
            written: Vec::new(),
        });
    }
    Ok(starts)
}

/// Every step from one start. What a call writes is under its own operands, as the receiver of a
/// call it's chained on can be, so no call is handed it until something reads it.
fn hops_from<'a>(
    graph: &Graph<'a>,
    from: &StartNodes<'a>,
    (positions, returned): (&HashMap<&'a str, usize>, &[&'a str]),
) -> Result<Vec<Hop>, String> {
    let (start, written) = (from.start, &from.written);
    let reached = graph.reached(&[&from.value[..], written].concat())?;
    let given = |node: &&str| reached.contains(*node) || from.value.contains(node);
    let mut found = Vec::new();

    for (&target, slots) in &graph.arguments {
        let Some(&call) = positions.get(target) else {
            continue;
        };
        let held = slots.iter().filter(|(_, _, node)| given(node));
        for &(position, element, _) in held.filter(|_| start != Start::Result(call)) {
            let to = match position {
                Some(position) => End::Argument {
                    call,
                    position,
                    element,
                },
                None => End::Unplaced { call },
            };
            found.push(Hop { from: start, to });
        }
    }
    if returned
        .iter()
        .any(|node| given(node) || written.contains(node))
    {
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
    for from in starts_of(&graph, &positions)? {
        hops.extend(hops_from(&graph, &from, (&positions, &returned))?);
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
