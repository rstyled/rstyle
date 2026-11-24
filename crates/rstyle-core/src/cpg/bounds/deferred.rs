//! Lexical ownership of one immutable iterator local, independently of stage semantics.
//! The caller proves root-block adjacency and validates the deferred pipeline itself.

use super::super::model::{
    AccessMode, DefinitionKind, Dispatch, Node, NodeKind, Place, PlaceMode, RefStatus, Symbol,
    TargetKind,
};
use super::facts::Facts;
use crate::advisory::runtime_bounds::MAX_NODES;
use std::collections::BTreeSet;

const MAX_SCAN: usize = MAX_NODES * 4;
const MAX_SPINE: usize = 32;

pub(super) struct Binding<'a> {
    pub symbol: &'a str,
    pub init: &'a Node,
    pub use_node: &'a Node,
}

struct Local<'a> {
    symbol: &'a Symbol,
    pattern: &'a Node,
    init: &'a Node,
}

/// The caller supplies an adjacent declaration and terminal tail in the root block.
/// This certificate authorizes neither constructor erasure nor a traversal bound.
pub(super) fn binding<'a>(
    facts: &Facts<'_, 'a>,
    declaration: &'a Node,
    tail: &'a Node,
) -> Result<Option<Binding<'a>>, String> {
    if !bounded(facts) {
        return Ok(None);
    }
    let Some(local) = local(facts, declaration)? else {
        return Ok(None);
    };
    let Some(use_node) = receiver_use(facts, tail, local.symbol)? else {
        return Ok(None);
    };
    if use_node.ty != local.symbol.ty || !private(facts, &local, use_node) {
        return Ok(None);
    }
    Ok(Some(Binding {
        symbol: local.symbol.id.as_str(),
        init: local.init,
        use_node,
    }))
}

fn local<'a>(facts: &Facts<'_, 'a>, declaration: &'a Node) -> Result<Option<Local<'a>>, String> {
    if declaration.kind != NodeKind::LocalDeclaration || declaration.children.len() != 2 {
        return Ok(None);
    }
    let pattern = facts.body.child(declaration, 0)?;
    let init = facts.body.child(declaration, 1)?;
    if pattern.kind != NodeKind::PatternBinding
        || !pattern.children.is_empty()
        || !pattern.modifiers.is_empty()
        || facts.adjusted_of(pattern)
        || pattern.binding_mode.as_deref().unwrap_or("value") != "value"
    {
        return Ok(None);
    }
    let mut symbols = facts
        .body
        .function
        .symbols
        .iter()
        .filter(|symbol| symbol.node.as_deref() == Some(pattern.id.as_str()));
    let Some(symbol) = symbols.next() else {
        return Ok(None);
    };
    if symbols.next().is_some()
        || symbol.kind != "local"
        || symbol.mutable
        || symbol.ty.is_none()
        || symbol.ty != pattern.ty
        || symbol.ty != init.ty
        || symbol
            .ty
            .as_deref()
            .is_none_or(|id| facts.type_of(id).is_none())
        || facts.adjusted_of(init)
    {
        return Ok(None);
    }
    Ok(Some(Local {
        symbol,
        pattern,
        init,
    }))
}

fn receiver_use<'a>(
    facts: &Facts<'_, 'a>,
    mut call: &'a Node,
    symbol: &Symbol,
) -> Result<Option<&'a Node>, String> {
    let mut seen = BTreeSet::new();
    for _ in 0..MAX_SPINE {
        if !seen.insert(call.id.as_str()) || !resolved_method(facts, call) {
            return Ok(None);
        }
        let Some(receiver) = call.children.first() else {
            return Ok(None);
        };
        let receiver = facts.body.node(receiver)?;
        if receiver.kind == NodeKind::Identifier {
            let reference = facts.reference(receiver);
            let exact = reference.is_some_and(|reference| {
                reference.status == RefStatus::Resolved
                    && reference.target.as_ref().is_some_and(|target| {
                        target.kind == TargetKind::Symbol
                            && target.id.as_deref() == Some(symbol.id.as_str())
                    })
            });
            return Ok(
                (exact && receiver.children.is_empty() && !facts.adjusted_of(receiver))
                    .then_some(receiver),
            );
        }
        call = receiver;
    }
    Ok(None)
}

fn resolved_method(facts: &Facts<'_, '_>, call: &Node) -> bool {
    if call.kind != NodeKind::Call {
        return false;
    }
    facts.reference(call).is_some_and(|reference| {
        reference.status == RefStatus::Resolved
            && reference.dispatch == Some(Dispatch::Method)
            && reference.arguments.is_none()
            && reference.target.as_ref().is_some_and(|target| {
                target.kind == TargetKind::Definition
                    && target.id.as_deref().is_some_and(|id| {
                        facts
                            .tables
                            .definition(id)
                            .is_ok_and(|definition| definition.kind == DefinitionKind::Method)
                    })
            })
    })
}

fn private(facts: &Facts<'_, '_>, local: &Local<'_>, use_node: &Node) -> bool {
    let function = facts.body.function;
    let captured = function.nodes.iter().any(|node| {
        (matches!(node.kind, NodeKind::Closure | NodeKind::ItemDeclaration)
            && node.captures.is_none())
            || node
                .captures
                .iter()
                .flatten()
                .any(|capture| capture.symbol == local.symbol.id)
    });
    !captured
        && structural(facts, local, use_node)
        && references(facts, &local.symbol.id, &use_node.id)
        && accesses(facts, local, use_node)
        && function.places.iter().all(|place| {
            place.root.symbol.as_deref() != Some(local.symbol.id.as_str())
                || inert_place(place, local.pattern, use_node)
        })
}

fn structural(facts: &Facts<'_, '_>, local: &Local<'_>, use_node: &Node) -> bool {
    [
        local.pattern.id.as_str(),
        local.init.id.as_str(),
        use_node.id.as_str(),
    ]
    .iter()
    .all(|id| {
        facts
            .body
            .function
            .nodes
            .iter()
            .flat_map(|node| &node.children)
            .filter(|child| child.as_str() == *id)
            .count()
            == 1
    })
}

fn references(facts: &Facts<'_, '_>, symbol: &str, use_node: &str) -> bool {
    let mut reads = 0;
    for reference in &facts.body.function.refs {
        if reference
            .selected
            .as_ref()
            .is_some_and(|target| target.id.as_deref() == Some(symbol))
        {
            return false;
        }
        let Some(target) = reference
            .target
            .as_ref()
            .filter(|target| target.id.as_deref() == Some(symbol))
        else {
            continue;
        };
        if reference.node != use_node
            || reference.status != RefStatus::Resolved
            || target.kind != TargetKind::Symbol
        {
            return false;
        }
        reads += 1;
    }
    reads == 1
}

fn accesses(facts: &Facts<'_, '_>, local: &Local<'_>, use_node: &Node) -> bool {
    let mut reads = 0;
    let mut writes = 0;
    for access in &facts.body.function.accesses {
        if access.symbol != local.symbol.id {
            continue;
        }
        match (access.mode, access.node.as_str()) {
            (AccessMode::Read, id) if id == use_node.id => reads += 1,
            (AccessMode::Write, id) if id == local.pattern.id => writes += 1,
            _ => return false,
        }
    }
    reads == 1 && writes == 1
}

fn inert_place(place: &Place, pattern: &Node, use_node: &Node) -> bool {
    let expected = match place.access {
        AccessMode::Read => place.node == use_node.id,
        AccessMode::Write => place.node == pattern.id,
        AccessMode::ReadWrite => false,
    };
    place.mode == PlaceMode::Value
        && place.escapes == Some(false)
        && place.path.is_empty()
        && place.adjustments.as_ref().is_none_or(Vec::is_empty)
        && expected
}

fn bounded(facts: &Facts<'_, '_>) -> bool {
    let function = facts.body.function;
    let rosters = [
        function.nodes.len(),
        function.refs.len(),
        function.symbols.len(),
        function.accesses.len(),
        function.places.len(),
    ];
    let Some(left) = rosters.into_iter().try_fold(MAX_SCAN, usize::checked_sub) else {
        return false;
    };
    function
        .nodes
        .iter()
        .try_fold(left, |left, node| {
            let left = left
                .checked_sub(node.children.len())?
                .checked_sub(node.adjustments.as_ref().map_or(0, Vec::len))?
                .checked_sub(node.captures.as_ref().map_or(0, Vec::len))?;
            node.captures
                .iter()
                .flatten()
                .try_fold(left, |left, capture| left.checked_sub(capture.path.len()))
        })
        .is_some()
}

#[cfg(test)]
mod tests;
