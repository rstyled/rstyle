//! The compiler judgments a runtime-bound lowering asks about a node, answered from
//! the document instead of the compiler.

use super::super::model::{
    Definition, DefinitionKind, Node, NodeKind, Ref, RefStatus, TargetKind, Type, TypeKind,
};
use super::super::source::Sources;
use super::super::tree::{Body, Tables, anchor};
use crate::runtime_bounds::{Integer, Source};
use std::collections::BTreeMap;

pub struct Facts<'b, 'a> {
    pub body: &'b Body<'a>,
    pub tables: &'b Tables<'a>,
    pub sources: &'b Sources<'a>,
    /// Width of a pointer-sized integer on the target.
    pub pointer_bits: u8,
}

impl<'a> Facts<'_, 'a> {
    /// Children that are source code or wrap it, without a lowering's own nodes.
    pub fn sources(&self, node: &Node) -> Result<Vec<&'a Node>, String> {
        self.body.source_children(node)
    }

    pub fn ty(&self, node: &Node) -> Option<&'a Type> {
        node.ty.as_deref().and_then(|id| self.tables.ty(id).ok())
    }

    pub fn type_of(&self, id: &str) -> Option<&'a Type> {
        self.tables.ty(id).ok()
    }

    /// The type after the node's implicit adjustments.
    pub fn adjusted(&self, node: &Node) -> Option<&'a Type> {
        let last = node
            .adjustments
            .as_ref()
            .and_then(|adjustments| adjustments.last());
        match last.and_then(|adjustment| adjustment.target.as_deref()) {
            Some(target) => self.type_of(target),
            None => self.ty(node),
        }
    }

    pub fn adjusted_of(&self, node: &Node) -> bool {
        node.adjustments
            .as_ref()
            .is_some_and(|adjustments| !adjustments.is_empty())
    }

    /// A primitive integer with an exact width the analysis models.
    pub fn integer(&self, ty: Option<&Type>) -> Option<Integer> {
        let ty = ty?;
        if ty.kind != TypeKind::Integer {
            return None;
        }

        let signed = ty.signed?;
        let bits = match ty.bits {
            _ if ty.pointer_sized == Some(true) => self.pointer_bits,
            Some(bits @ (8 | 16 | 32 | 64)) => bits as u8,
            _ => return None,
        };

        Some(Integer { bits, signed })
    }

    /// Width of an unsigned integer the analysis models, for ranges and iteration.
    pub fn unsigned_bits(&self, ty: Option<&Type>) -> Option<u8> {
        self.integer(ty)
            .filter(|integer| !integer.signed)
            .map(|integer| integer.bits)
    }

    pub fn reference(&self, node: &Node) -> Option<&'a Ref> {
        self.body.reference(&node.id)
    }

    /// The symbol an identifier resolves to.
    pub fn local(&self, node: &Node) -> Option<&'a str> {
        if node.kind != NodeKind::Identifier {
            return None;
        }
        let target = self.reference(node)?.target.as_ref()?;
        (target.kind == TargetKind::Symbol)
            .then_some(target.id.as_deref())
            .flatten()
    }

    /// A stable number for each symbol, standing in for the frontend's own.
    pub fn symbol_numbers(&self) -> BTreeMap<&'a str, u32> {
        let symbols = &self.body.function.symbols;
        symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (symbol.id.as_str(), index as u32))
            .collect()
    }

    /// What an identifier resolves to: a definition or constructor, with its kind.
    pub fn resolved(&self, node: &Node) -> Option<(TargetKind, &'a Definition)> {
        let target = self.reference(node)?.target.as_ref()?;
        let definition = self.tables.definition(target.id.as_deref()?).ok()?;
        matches!(
            target.kind,
            TargetKind::Definition | TargetKind::Constructor
        )
        .then_some((target.kind, definition))
    }

    /// The member a call or operator declares: its target, or `via` when dependent.
    pub fn declared(&self, reference: &Ref) -> Option<&'a Definition> {
        let id = match reference.status {
            RefStatus::Dependent => reference.via.as_deref()?,
            _ => reference.target.as_ref()?.id.as_deref()?,
        };
        self.tables.definition(id).ok()
    }

    /// A member of an interface (Rust trait) rather than of an implementation.
    pub fn interface_member(&self, definition: &Definition) -> bool {
        self.tables
            .named_definition(definition.owner.as_deref())
            .is_some_and(|owner| owner.kind == DefinitionKind::Trait)
    }

    /// The node's location as the program records it.
    pub fn source(&self, node: &Node, evidence: &str) -> Option<Source> {
        let span = anchor(node).ok()?;
        let (line, column) = self.sources.location(span).ok()?;
        Some(Source {
            file: self.sources.path(span).ok()?.to_string(),
            line,
            column,
            evidence: evidence.into(),
        })
    }

    /// Source text of a node, as written.
    pub fn snippet(&self, node: &Node) -> String {
        node.span
            .as_ref()
            .and_then(|span| self.sources.between(span, span).ok())
            .unwrap_or_default()
            .to_string()
    }

    /// A standard type's name in its contract namespace, such as `Vec` for `rust:Vec`.
    pub fn type_key<'t>(&self, ty: &'t Type) -> Option<&'t str> {
        ty.key.as_deref()?.split_once(':').map(|(_, name)| name)
    }

    /// Whether `ty` or a type it is built from, within `depth` levels, satisfies `test`.
    pub fn any_part(&self, ty: &Type, depth: usize, test: &dyn Fn(&Type) -> bool) -> bool {
        let Some(depth) = depth.checked_sub(1) else {
            return false;
        };
        test(ty)
            || super::calls::components(ty)
                .filter_map(|id| self.type_of(id))
                .any(|part| self.any_part(part, depth, test))
    }

    pub fn standard_definition(&self, id: Option<&str>) -> bool {
        id.and_then(|id| self.tables.definition(id).ok())
            .is_some_and(|definition| definition.standard)
    }
}

/// A shared reference, through any number of shared references.
pub fn shared_target<'t>(facts: &Facts<'_, 't>, mut ty: &'t Type) -> Option<&'t Type> {
    while ty.kind == TypeKind::Reference {
        if ty.mode.as_deref() != Some("shared") {
            return None;
        }
        ty = facts.type_of(ty.target.as_deref()?)?;
    }
    Some(ty)
}

pub fn is_shared_reference(ty: Option<&Type>) -> bool {
    ty.is_some_and(|ty| ty.kind == TypeKind::Reference && ty.mode.as_deref() == Some("shared"))
}
