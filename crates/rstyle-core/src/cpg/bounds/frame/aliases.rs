//! Provenance for simple immutable aliases of a shared formal's places.
//!
//! The frame must select `source_for` alongside every selected alias extent. That
//! subjects both names to its existing whole-body write, borrow and capture checks,
//! including accesses before the alias is initialized.

use super::super::super::model::{Node, NodeKind, RefStatus, Type, TypeKind};
use super::super::extent::{Place, place};
use super::super::facts::{Facts, is_shared_reference};
use std::collections::{BTreeMap, BTreeSet};

const MAX_PATH: usize = 64;
const MAX_WORK: usize = 256;

/// The frame's formal roots and stable symbol numbering, in their fact context.
pub(super) struct SharedFormals<'f, 'b, 'a> {
    facts: &'f Facts<'b, 'a>,
    symbols: &'f BTreeSet<&'a str>,
    numbers: &'f BTreeMap<&'a str, u32>,
}

#[derive(Clone)]
pub(super) struct Alias<'a> {
    pub local: &'a str,
    source: Place<'a>,
    source_name: &'a str,
    source_number: u32,
    target: &'a Type,
}

impl<'f, 'b, 'a> SharedFormals<'f, 'b, 'a> {
    pub fn new(
        facts: &'f Facts<'b, 'a>,
        symbols: &'f BTreeSet<&'a str>,
        numbers: &'f BTreeMap<&'a str, u32>,
    ) -> Self {
        Self {
            facts,
            symbols,
            numbers,
        }
    }

    /// Frame has certified an immutable simple value binding outside loops,
    /// without a `let-else` alternative.
    /// This deliberately does not follow aliases of other locals or destructuring.
    pub fn for_local(
        &self,
        pattern: &'a Node,
        initializer: &'a Node,
    ) -> Result<Option<Alias<'a>>, String> {
        let Some((local, target)) = binding(self.facts, pattern, initializer) else {
            return Ok(None);
        };
        if !pure_path(self.facts, initializer)? {
            return Ok(None);
        }
        let Some(source) = place(self.facts, initializer, self.numbers)? else {
            return Ok(None);
        };
        if !self.symbols.contains(source.root) {
            return Ok(None);
        }
        let formal = self.facts.body.symbol(source.root)?;
        let ty = formal.ty.as_deref().and_then(|ty| self.facts.type_of(ty));
        if formal.mutable || !is_shared_reference(ty) {
            return Ok(None);
        }
        // Both maps come from this body's symbol roster, and the root was resolved above.
        let source_number = self.numbers[source.root];
        Ok(Some(Alias {
            local,
            source,
            source_name: formal.name.as_str(),
            source_number,
            target,
        }))
    }
}

impl<'a> Alias<'a> {
    /// The corresponding formal-rooted place, including fields measured below
    /// the alias. Its label names that actual formal path, not an intermediate record.
    pub fn source_for(
        &self,
        facts: &Facts<'_, 'a>,
        selected: &Place<'a>,
        expression: &'a Node,
    ) -> Result<Option<Place<'a>>, String> {
        if selected.root != self.local
            || self.source.fields.len() + selected.fields.len() > MAX_PATH
            || selected.fields.len() != selected.path.len()
            || !pure_path(facts, expression)?
        {
            return Ok(None);
        }
        let mut ty = self.target;
        for field in &selected.path {
            let Some(field) = field_type(facts, ty, field.index, &field.name) else {
                return Ok(None);
            };
            ty = field;
        }
        let mut source = self.source.clone();
        source.fields.extend_from_slice(&selected.fields);
        source.path.extend_from_slice(&selected.path);
        let mut label = self.source_name.to_owned();
        for field in &source.path {
            label.push('.');
            label.push_str(&field.name);
        }
        source.label = format!("len({label})#{}", self.source_number);
        Ok(Some(source))
    }
}

fn binding<'a>(
    facts: &Facts<'_, 'a>,
    pattern: &Node,
    initializer: &Node,
) -> Option<(&'a str, &'a Type)> {
    let symbol = facts
        .body
        .function
        .symbols
        .iter()
        .find(|symbol| symbol.node.as_deref() == Some(pattern.id.as_str()))?;
    let ty = symbol.ty.as_deref().and_then(|ty| facts.type_of(ty));
    let ty = ty.filter(|ty| {
        is_shared_reference(Some(ty))
            && pattern.ty.as_deref() == Some(ty.id.as_str())
            && facts
                .adjusted(initializer)
                .is_some_and(|init| init.id == ty.id)
    })?;
    Some((symbol.id.as_str(), ty))
}

/// A single path has no additional expression children whose evaluation a place
/// lookup could hide. `place` separately validates every type adjustment.
fn pure_path<'a>(facts: &Facts<'_, 'a>, mut node: &'a Node) -> Result<bool, String> {
    let mut work = 0;
    for _ in 0..MAX_PATH {
        work += 1 + node.adjustments.as_ref().map_or(0, Vec::len);
        if work > MAX_WORK {
            return Ok(false);
        }
        if node.adjustments.as_ref().is_some_and(|adjustments| {
            adjustments.iter().any(|adjustment| {
                adjustment
                    .target
                    .as_deref()
                    .and_then(|ty| facts.type_of(ty))
                    .is_none()
            })
        }) {
            return Ok(false);
        }
        if node.kind == NodeKind::Identifier {
            let Some(local) = facts.local(node) else {
                return Ok(false);
            };
            let symbol = facts.body.symbol(local)?;
            let resolved = facts
                .reference(node)
                .is_some_and(|r| r.status == RefStatus::Resolved);
            return Ok(node.children.is_empty()
                && resolved
                && node.ty.is_some()
                && node.ty == symbol.ty);
        }
        if node.children.len() != 1 {
            return Ok(false);
        }
        let base = facts.body.child(node, 0)?;
        if !native_projection(facts, node, base) {
            return Ok(false);
        }
        node = base;
    }
    Ok(false)
}

fn native_projection(facts: &Facts<'_, '_>, node: &Node, base: &Node) -> bool {
    let (Some(ty), Some(base_ty)) = (facts.ty(node), facts.adjusted(base)) else {
        return false;
    };
    match node.kind {
        NodeKind::FieldAccess => {
            let (Some(index), Some(name)) = (node.member_index, node.member.as_deref()) else {
                return false;
            };
            facts.reference(node).is_none()
                && field_type(facts, base_ty, index, name).is_some_and(|field| field.id == ty.id)
        }
        NodeKind::Reference => {
            node.modifiers.is_empty()
                && is_shared_reference(Some(ty))
                && ty.target.as_deref() == Some(base_ty.id.as_str())
        }
        NodeKind::Operator if node.operator.as_deref() == Some("deref") => {
            facts.reference(node).is_none()
                && is_shared_reference(Some(base_ty))
                && base_ty.target.as_deref() == Some(ty.id.as_str())
        }
        _ => false,
    }
}

fn field_type<'a>(
    facts: &Facts<'_, 'a>,
    mut ty: &'a Type,
    index: u32,
    name: &str,
) -> Option<&'a Type> {
    for _ in 0..MAX_PATH {
        if ty.kind != TypeKind::Reference {
            let id = match ty.kind {
                TypeKind::Record => {
                    let field = ty.fields.as_ref()?.get(index as usize)?;
                    (!name.is_empty() && field.name == name && !field.embedded)
                        .then_some(field.ty.as_str())?
                }
                TypeKind::Tuple => {
                    (name == index.to_string()).then_some(())?;
                    ty.items.as_ref()?.get(index as usize)?.as_str()
                }
                _ => return None,
            };
            return facts.type_of(id);
        }
        if !is_shared_reference(Some(ty)) {
            return None;
        }
        ty = facts.type_of(ty.target.as_deref()?)?;
    }
    None
}
