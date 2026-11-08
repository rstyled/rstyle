//! A call whose result is thrown away where the rest of the run uses it. Nothing in a signature
//! says a result matters, so a callee's calls decide, as in "Bugs as Deviant Behavior" (Engler et
//! al., SOSP 2001): when nearly all of them use it, the few that don't stand out.
//!
//! ```go
//! json.Unmarshal(raw, &out) // nearly every other call of this checks the error
//! ```

use super::super::model::{
    Definition, DefinitionKind, Dispatch, Node, NodeKind, Origin, RefStatus, TargetKind, Type,
    TypeKind,
};
use super::super::tree::{self, Tables};
use super::Reading;
use crate::config::IgnoredResultConfig;
use std::collections::BTreeSet;

pub const RULE: &str = "ignored_result";

/// What one call did with its result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultUse {
    /// The callee's identity, which a run's calls are weighed by.
    pub callee: String,
    /// The callee as a message names it.
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub dropped: bool,
}

/// Whether `used` of a callee's `total` calls are enough that the rest stand out.
pub fn stands_out(used: usize, total: usize, limits: &IgnoredResultConfig) -> bool {
    total >= limits.min_calls && used * 100 >= total * limits.min_used_percent.0
}

pub fn message(name: &str, used: usize, total: usize) -> String {
    format!("the result of `{name}` is dropped here, where {used} of its {total} calls use it")
}

pub(super) fn uses(reading: &Reading<'_>) -> Result<Vec<ResultUse>, String> {
    let mut found = Vec::new();
    for node in &reading.function.nodes {
        if let Some(call) = call_that_counts(reading, node)? {
            found.push(call);
        }
    }
    Ok(found)
}

fn call_that_counts(reading: &Reading<'_>, node: &Node) -> Result<Option<ResultUse>, String> {
    if node.kind != NodeKind::Call || node.provenance.origin != Origin::Source {
        return Ok(None);
    }
    let (Some(dropped), Some(span)) = (dropped(reading, node), node.span.as_ref()) else {
        return Ok(None);
    };
    let Some(callee) = declared_callee(reading, node)? else {
        return Ok(None);
    };
    if !gives_a_result(reading, node)? {
        return Ok(None);
    }

    let (line, column) = reading.sources.location(span)?;
    Ok(Some(ResultUse {
        callee: callee.stable_identity.clone(),
        name: callee.name.clone(),
        line,
        column,
        dropped,
    }))
}

/// Whether a call's place drops its result, or nothing when it can't say: an extension says how
/// a call runs, as a `defer` does, and a statement a lowering made up isn't one anyone wrote.
/// Anything else takes the value, even to discard it by name, which says that it's deliberate.
fn dropped(reading: &Reading<'_>, node: &Node) -> Option<bool> {
    let Some(parent) = reading.parents.get(node.id.as_str()) else {
        return Some(false);
    };
    match parent.kind {
        NodeKind::Extension => None,
        NodeKind::ExpressionStatement if tree::generated(parent) => None,
        NodeKind::ExpressionStatement => Some(true),
        _ => Some(false),
    }
}

/// The function or method a call names, whichever implementation it reaches: what's expected of
/// a callee is expected of everything that implements it. A call of a value names nothing.
pub(super) fn declared_callee<'r>(
    reading: &Reading<'r>,
    node: &Node,
) -> Result<Option<&'r Definition>, String> {
    let Some(reference) = reading.body.reference(&node.id) else {
        return Ok(None);
    };
    let declared = match reference.status {
        RefStatus::Unresolved => None,
        RefStatus::Dependent => reference.via.as_deref(),
        RefStatus::Resolved if reference.dispatch == Some(Dispatch::FunctionValue) => None,
        RefStatus::Resolved => reference
            .target
            .as_ref()
            .filter(|target| {
                matches!(
                    target.kind,
                    TargetKind::Definition | TargetKind::LibraryOperation
                )
            })
            .and_then(|target| target.id.as_deref()),
    };
    let Some(declared) = declared else {
        return Ok(None);
    };

    let definition = reading.tables.definition(declared)?;
    Ok(matches!(
        definition.kind,
        DefinitionKind::Function | DefinitionKind::Method
    )
    .then_some(definition))
}

/// Whether a call hands back something of its own: not nothing, not what the frontend can't
/// describe, and not the receiver it was called on, which is only there so calls can chain.
fn gives_a_result(reading: &Reading<'_>, node: &Node) -> Result<bool, String> {
    let Some(ty) = node.ty.as_deref() else {
        return Ok(false);
    };
    let ty = reading.tables.ty(ty)?;
    if matches!(
        ty.kind,
        TypeKind::Unit | TypeKind::Bottom | TypeKind::Opaque | TypeKind::Parameter
    ) {
        return Ok(false);
    }
    Ok(!returns_its_receiver(reading, node, ty)?)
}

pub(super) fn returns_its_receiver(
    reading: &Reading<'_>,
    node: &Node,
    result: &Type,
) -> Result<bool, String> {
    let on_a_receiver = reading.body.reference(&node.id).is_some_and(|reference| {
        matches!(
            reference.dispatch,
            Some(Dispatch::Method | Dispatch::Dynamic)
        )
    });
    let Some(receiver) = node.children.first().filter(|_| on_a_receiver) else {
        return Ok(false);
    };
    let Some(receiver) = reading.body.node(receiver)?.ty.as_deref() else {
        return Ok(false);
    };

    let tables = reading.tables;
    let receiver = pointed_at(tables, tables.ty(receiver)?)?;
    Ok(receiver.spelling == pointed_at(tables, result)?.spelling)
}

/// What a chain of references and pointers ends at, or where it starts repeating. No frontend
/// writes a chain that repeats, but a document is only data.
pub(super) fn pointed_at<'t>(tables: &Tables<'t>, ty: &'t Type) -> Result<&'t Type, String> {
    let (mut at, mut seen) = (ty, BTreeSet::new());
    while let (TypeKind::Reference | TypeKind::Pointer, Some(target)) =
        (at.kind, at.target.as_deref())
    {
        if !seen.insert(target) {
            break;
        }
        at = tables.ty(target)?;
    }
    Ok(at)
}

#[cfg(test)]
mod tests;
