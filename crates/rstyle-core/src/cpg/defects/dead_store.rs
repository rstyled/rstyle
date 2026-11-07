//! A value stored in a binding that nothing can read, because every path from the store
//! replaces it or leaves the function first. The value was computed for a reason, and the most
//! common one is an error that was never checked:
//!
//! ```go
//! x, err := f()
//! y, err := g(x) // the first err is gone
//! ```

use super::super::flow::{Reaching, Vertex, changed_elsewhere};
use super::super::model::{AccessMode, Function, Node, NodeKind, Origin, Symbol, TypeKind};
use super::super::tree::{self, Body, Tables};
use super::{Defect, Reading};
use std::collections::{BTreeSet, HashMap, HashSet};

pub(super) const RULE: &str = "dead_store";

/// How deep a type's arguments are searched for a destructor. Far more than a type is written.
const ARGUMENT_DEPTH: usize = 16;

pub(super) fn find(reading: &Reading<'_>) -> Result<Vec<Defect>, String> {
    let function = reading.function;
    let body = Body::new(function)?;
    let reached: HashSet<(Vertex, &str)> = Reaching::of(function, &reading.flow)
        .data_dependence()
        .into_iter()
        .map(|(store, _, symbol)| (store, symbol))
        .collect();
    let parents = parents(function);
    let skipped = unreliable(function, reading);
    let mut found = Vec::new();

    for access in &function.accesses {
        let Some(vertex) = reading.flow.vertex(&access.node) else {
            continue;
        };
        let symbol = access.symbol.as_str();
        if access.mode != AccessMode::Write
            || reached.contains(&(vertex, symbol))
            || skipped.contains(symbol)
        {
            continue;
        }
        let node = body.node(&access.node)?;
        if node.provenance.origin == Origin::Source && stores_a_value(node, &parents, &body)? {
            found.push(defect(reading, &body, node, symbol)?);
        }
    }
    Ok(found)
}

fn defect(
    reading: &Reading<'_>,
    body: &Body<'_>,
    node: &Node,
    symbol: &str,
) -> Result<Defect, String> {
    Ok(Defect {
        rule: RULE,
        line: tree::line(reading.sources, node)?,
        message: format!(
            "the value stored in `{}` here is never read: every path replaces it or leaves first",
            body.symbol(symbol)?.name
        ),
    })
}

fn parents(function: &Function) -> HashMap<&str, &Node> {
    function
        .nodes
        .iter()
        .flat_map(|node| {
            node.children
                .iter()
                .map(move |child| (child.as_str(), node))
        })
        .collect()
}

/// Symbols a read of the name doesn't account for, or whose stores aren't only that value. A
/// closure reads what it captures when it's called, whenever that is, a named result is read by
/// the return, and a read off the flow can't be placed before or after anything. A destructor is
/// what running out of scope is for, as a lock guard's is. A type the frontend can't describe
/// might do anything when written, as a volatile int does, and so can a type parameter. A leading
/// underscore says that it's deliberate.
fn unreliable<'f>(function: &'f Function, reading: &Reading<'f>) -> BTreeSet<&'f str> {
    let signature = declared_by_signature(function);
    let captured = function
        .nodes
        .iter()
        .flat_map(|node| node.captures.iter().flatten())
        .map(|capture| capture.symbol.as_str());
    let not_for_their_value = function
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.name.starts_with('_')
                || (symbol.node.is_none() && !signature.contains(symbol.id.as_str()))
                || writing_does_more(symbol, reading.tables)
        })
        .map(|symbol| symbol.id.as_str());

    changed_elsewhere(function, &reading.flow)
        .into_iter()
        .chain(captured)
        .chain(reads_off_the_flow(function, reading))
        .chain(not_for_their_value)
        .collect()
}

fn reads_off_the_flow<'r, 'f: 'r>(
    function: &'f Function,
    reading: &'r Reading<'f>,
) -> impl Iterator<Item = &'f str> + 'r {
    function
        .accesses
        .iter()
        .filter(|access| access.mode != AccessMode::Write)
        .filter(|access| reading.flow.vertex(&access.node).is_none())
        .map(|access| access.symbol.as_str())
}

fn declared_by_signature(function: &Function) -> HashSet<&str> {
    let signature = &function.signature;
    signature
        .parameters
        .iter()
        .map(|parameter| parameter.symbol.as_str())
        .chain(signature.receiver.iter().map(|it| it.symbol.as_str()))
        .collect()
}

fn writing_does_more(symbol: &Symbol, tables: &Tables<'_>) -> bool {
    symbol
        .ty
        .as_deref()
        .is_none_or(|ty| type_does_more(ty, tables, ARGUMENT_DEPTH))
}

fn type_does_more(id: &str, tables: &Tables<'_>, depth: usize) -> bool {
    let Ok(ty) = tables.ty(id) else {
        return true;
    };
    let destructor = tables
        .named_definition(ty.definition.as_deref())
        .is_some_and(|definition| definition.destructor);
    destructor
        || matches!(ty.kind, TypeKind::Opaque | TypeKind::Parameter)
        || depth > 0
            && ty
                .arguments
                .iter()
                .flatten()
                .any(|argument| type_does_more(argument, tables, depth - 1))
}

/// Whether a write puts a computed value into a binding: an assignment's target, or a binding a
/// declaration's initializer fills. A `for` or a match binds a part of what it's looking at, and
/// a declaration with no initializer holds nothing yet. Nor is `count := 0` a loss: giving a
/// binding a starting value is a choice of style, where overwriting a result drops something.
fn stores_a_value(
    node: &Node,
    parents: &HashMap<&str, &Node>,
    body: &Body<'_>,
) -> Result<bool, String> {
    let mut at = node;
    while let Some(parent) = parents.get(at.id.as_str()) {
        match parent.kind {
            NodeKind::Assignment | NodeKind::DestructuringAssignment => {
                return Ok(parent.children.first() == Some(&at.id));
            }
            NodeKind::LocalDeclaration => return computed_initializer(parent, body),
            NodeKind::Tuple
            | NodeKind::Array
            | NodeKind::RecordConstruction
            | NodeKind::PatternTuple
            | NodeKind::PatternConstructor
            | NodeKind::PatternSequence
            | NodeKind::PatternAlias => at = parent,
            _ => return Ok(false),
        }
    }
    Ok(false)
}

/// The profile's child order for a declaration: its pattern, an optional type annotation, then
/// the initializer. A diverging `else` can only follow an initializer.
fn computed_initializer(declaration: &Node, body: &Body<'_>) -> Result<bool, String> {
    let children = body.children(declaration)?;
    let initializer = children
        .iter()
        .skip(1)
        .find(|child| child.kind != NodeKind::TypeAnnotation);
    Ok(initializer.is_some_and(|value| value.kind != NodeKind::Literal))
}
