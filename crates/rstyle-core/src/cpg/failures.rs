//! Where the error a function returns can come from, read from its tree and the reaching
//! definitions its flow gives: a constructor's message, a call that returns an error, a variable
//! that holds one, or something it can't follow. A call stays a call, so a comparison can follow it
//! into a function of the same codebase.
//!
//! Only Go's errors are read so far: an `error` result, made by `fmt.Errorf` or `errors.New`.

use super::flow::Derived;
use super::model::{Dispatch, Function, Node, NodeKind, Ref, TargetKind, TypeKind};
use super::tree::Tables;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Calls that make an error from a message, its first string argument, by identity.
const CONSTRUCTORS: [&str; 2] = ["go::fmt.Errorf", "go::errors.New"];
/// What returning no error reads as.
const NO_ERROR: &str = "go::nil";
/// The verb that wraps another error into a message.
const WRAPS: &str = "%w";

/// Where an error can come from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Source {
    /// Made here, with this message; empty when the message is computed.
    Message(String),
    /// Returned by a call to this callee.
    Call(String),
    /// Held by this variable, as a sentinel error is.
    Variable(String),
    /// Through something that can't be followed: a function value, or a value the flow doesn't
    /// record a definition of. A call through an interface is a call to its method.
    Unknown,
}

/// A source and how many messages wrap it on the way out.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Failure {
    pub source: Source,
    pub wrapped: usize,
}

/// Where the error `function` returns can come from, or nothing when it returns none.
pub fn of(function: &Function, flow: &Derived, tables: &Tables<'_>) -> Option<Vec<Failure>> {
    if !is_error(tables, &function.signature.result) {
        return None;
    }
    let Some(data) = &flow.data else {
        return Some(vec![Failure {
            source: Source::Unknown,
            wrapped: 0,
        }]);
    };

    let mut walk = Walk::new(function, data, tables);
    let found: BTreeSet<Failure> = walk
        .returns(function)
        .into_iter()
        .flat_map(|operand| walk.of_node(operand, 0))
        .collect();
    Some(found.into_iter().collect())
}

/// Whether a type is an error, or a tuple whose last item is one.
fn is_error(tables: &Tables<'_>, id: &str) -> bool {
    let Ok(ty) = tables.ty(id) else {
        return false;
    };
    match (&ty.kind, ty.items.as_deref()) {
        (TypeKind::Tuple, Some([.., last])) => is_error(tables, last),
        _ => ty.spelling == "error",
    }
}

struct Walk<'f> {
    nodes: HashMap<&'f str, &'f Node>,
    parents: HashMap<&'f str, &'f str>,
    refs: HashMap<&'f str, &'f Ref>,
    /// Each use, and the definitions of its symbol that reach it.
    reaching: HashMap<(&'f str, &'f str), Vec<&'f str>>,
    tables: &'f Tables<'f>,
    seen: HashSet<&'f str>,
}

impl<'f> Walk<'f> {
    fn new(
        function: &'f Function,
        data: &'f [super::flow::DataEdge],
        tables: &'f Tables<'f>,
    ) -> Self {
        let mut reaching: HashMap<(&str, &str), Vec<&str>> = HashMap::new();
        for edge in data {
            let use_of = (edge.target.as_str(), edge.symbol.as_str());
            reaching.entry(use_of).or_default().push(&edge.source);
        }
        let parents = function
            .nodes
            .iter()
            .flat_map(|node| {
                node.children
                    .iter()
                    .map(|child| (child.as_str(), node.id.as_str()))
            })
            .collect();
        Self {
            nodes: function
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect(),
            parents,
            refs: function
                .refs
                .iter()
                .map(|found| (found.node.as_str(), found))
                .collect(),
            reaching,
            tables,
            seen: HashSet::new(),
        }
    }

    /// The error operand of each return that isn't inside a closure: the last of the values it
    /// returns.
    fn returns(&self, function: &'f Function) -> Vec<&'f str> {
        let returns = function
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Return && !self.in_closure(&node.id));
        let operands = returns.filter_map(|node| self.nodes.get(node.children.first()?.as_str()));
        operands.map(|operand| error_operand(operand)).collect()
    }

    fn in_closure(&self, node: &str) -> bool {
        let mut at = node;
        while let Some(&parent) = self.parents.get(at) {
            if self
                .nodes
                .get(parent)
                .is_some_and(|node| node.kind == NodeKind::Closure)
            {
                return true;
            }
            at = parent;
        }
        false
    }

    fn of_node(&mut self, id: &'f str, wrapped: usize) -> Vec<Failure> {
        let Some(&node) = self.nodes.get(id) else {
            return unknown(wrapped);
        };
        if !self.seen.insert(id) {
            return Vec::new();
        }
        if node.kind == NodeKind::Call {
            return self.of_call(node, wrapped);
        }

        let target = self.refs.get(id).and_then(|found| found.target.as_ref());
        match target.map(|target| (target.kind, target.id.as_deref())) {
            Some((TargetKind::Symbol, Some(symbol))) => self.of_symbol(id, symbol, wrapped),
            Some((TargetKind::Definition, id)) => self.of_definition(id, wrapped),
            _ => unknown(wrapped),
        }
    }

    /// What each definition of a symbol that reaches this use assigns it. A parameter's comes from
    /// the caller, which is the caller's to say.
    fn of_symbol(&mut self, id: &'f str, symbol: &'f str, wrapped: usize) -> Vec<Failure> {
        let definitions = self
            .reaching
            .get(&(id, symbol))
            .cloned()
            .unwrap_or_default();
        let values: Vec<&str> = definitions
            .into_iter()
            .filter(|definition| self.nodes.contains_key(definition))
            .map(|definition| self.assigned(definition))
            .collect();
        values
            .into_iter()
            .flat_map(|value| self.of_node(value, wrapped))
            .collect()
    }

    fn of_definition(&self, id: Option<&str>, wrapped: usize) -> Vec<Failure> {
        match self
            .tables
            .named_definition(id)
            .map(|found| found.stable_identity.as_str())
        {
            Some(NO_ERROR) => Vec::new(),
            Some(identity) => vec![failure(Source::Variable(identity.to_owned()), wrapped)],
            None => unknown(wrapped),
        }
    }

    fn of_call(&mut self, call: &'f Node, wrapped: usize) -> Vec<Failure> {
        let found = self.refs.get(call.id.as_str()).copied();
        let through_value = found
            .and_then(|found| found.dispatch)
            .is_some_and(|dispatch| dispatch == Dispatch::FunctionValue);
        let callee = found
            .and_then(|found| found.target.as_ref())
            .and_then(|target| self.tables.named_definition(target.id.as_deref()))
            .map(|definition| definition.stable_identity.as_str());
        let Some(callee) = callee.filter(|_| !through_value) else {
            return unknown(wrapped);
        };

        if CONSTRUCTORS.contains(&callee) {
            return self.made(call, wrapped);
        }
        let returns_error = call
            .ty
            .as_deref()
            .is_some_and(|ty| is_error(self.tables, ty));
        match returns_error {
            true => vec![failure(Source::Call(callee.to_owned()), wrapped)],
            false => Vec::new(),
        }
    }

    /// An error a constructor makes, and the errors its message wraps.
    fn made(&mut self, call: &'f Node, wrapped: usize) -> Vec<Failure> {
        let message = self.message(call);
        let made = failure(Source::Message(message.clone()), wrapped);
        if !message.contains(WRAPS) {
            return vec![made];
        }
        let arguments: Vec<&'f str> = call
            .children
            .iter()
            .skip(2)
            .map(String::as_str)
            .filter(|&argument| self.is_error_node(argument))
            .collect();
        let inner = arguments
            .into_iter()
            .flat_map(|argument| self.of_node(argument, wrapped + 1));
        std::iter::once(made)
            .chain(inner.collect::<Vec<_>>())
            .collect()
    }

    fn is_error_node(&self, id: &str) -> bool {
        let ty = self.nodes.get(id).and_then(|node| node.ty.as_deref());
        ty.is_some_and(|ty| is_error(self.tables, ty))
    }

    /// A constructor's first string argument, or nothing when it's computed.
    fn message(&self, call: &Node) -> String {
        let literals = call.children.iter().skip(1).filter_map(|child| {
            let value = self.nodes.get(child.as_str())?.value.as_ref()?;
            (value.kind == "string")
                .then(|| value.value.clone())
                .flatten()
        });
        literals.into_iter().next().unwrap_or_default()
    }

    /// The value a definition binds: the last child of the declaration or assignment it's in,
    /// through any tuple of bindings.
    fn assigned(&self, binding: &'f str) -> &'f str {
        let mut at = binding;
        while let Some(&parent) = self.parents.get(at) {
            let grouping = self
                .nodes
                .get(parent)
                .is_some_and(|node| matches!(node.kind, NodeKind::PatternTuple | NodeKind::Tuple));
            if !grouping {
                break;
            }
            at = parent;
        }
        let holder = self
            .parents
            .get(at)
            .and_then(|parent| self.nodes.get(parent));
        let value = holder
            .and_then(|holder| holder.children.last())
            .map(String::as_str);
        value.filter(|&value| value != at).unwrap_or(binding)
    }
}

/// The error a return hands back: the last of the values it returns.
fn error_operand(returned: &Node) -> &str {
    match returned.kind {
        NodeKind::Tuple => returned.children.last().unwrap_or(&returned.id),
        _ => &returned.id,
    }
}

fn failure(source: Source, wrapped: usize) -> Failure {
    Failure { source, wrapped }
}

fn unknown(wrapped: usize) -> Vec<Failure> {
    vec![failure(Source::Unknown, wrapped)]
}
