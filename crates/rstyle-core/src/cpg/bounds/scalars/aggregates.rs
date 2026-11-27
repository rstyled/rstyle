//! Direct tuple bindings, densely initialized sequences, and by-value record fields.
use super::{Scalar, Walk};
use crate::cpg::model::{Node, NodeKind, TypeKind};
use std::collections::BTreeMap;
use std::sync::Arc;

impl<'a> Walk<'_, '_, 'a> {
    fn binding(&self, place: &Node) -> Option<&'a str> {
        if place.kind == NodeKind::PatternBinding
            && place.children.is_empty()
            && place.binding_mode.as_deref().unwrap_or("value") == "value"
        {
            self.bindings.get(place.id.as_str()).copied()
        } else {
            None
        }
    }

    pub(super) fn assign(
        &mut self,
        place: &'a Node,
        rhs: &'a Node,
        kind: NodeKind,
        depth: usize,
    ) -> Option<()> {
        if place.kind == NodeKind::PatternTuple
            && kind == NodeKind::LocalDeclaration
            && rhs.kind == NodeKind::Tuple
            && place.children.len() == rhs.children.len()
        {
            // Evaluate all right-hand expressions against the incoming state before storing any.
            let mut bindings = Vec::new();
            for (p, v) in place.children.iter().zip(&rhs.children) {
                let p = self.facts.body.node(p).ok()?;
                let value = self.value(self.facts.body.node(v).ok()?, depth)?;
                bindings.push((self.binding(p)?, self.for_storage(p, value)?));
            }
            self.locals.extend(bindings);
            return Some(());
        }
        let value = self.value(rhs, depth)?;
        let value = self.for_storage(place, value)?;
        if place.kind == NodeKind::FieldAccess
            && kind == NodeKind::Assignment
            && place.children.len() == 1
        {
            let symbol = self.facts.local(self.child(place, 0)?)?;
            let Scalar::Record(fields) = self.locals.get_mut(symbol)? else {
                return None;
            };
            *Arc::make_mut(fields).get_mut(place.member.as_deref()?)? = value;
            return Some(());
        }
        let symbol = if kind == NodeKind::LocalDeclaration {
            self.binding(place)?
        } else {
            self.facts
                .local(place)
                .filter(|s| self.locals.contains_key(s))?
        };
        self.locals.insert(symbol, value);
        Some(())
    }

    pub(super) fn record(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let fields = self.facts.ty(node)?.fields.as_deref()?;
        if fields.len() > 128 || node.children.len() > fields.len() {
            return None;
        }
        let mut items = BTreeMap::new();
        if node
            .members
            .as_ref()
            .is_some_and(|members| members.len() != node.children.len())
        {
            return None;
        }
        for (i, child) in node.children.iter().enumerate() {
            let name = match &node.members {
                Some(names) => names.get(i)?,
                None => &fields.get(i)?.name,
            };
            let def = fields.iter().find(|field| &field.name == name)?;
            let value = self.value(self.facts.body.node(child).ok()?, depth)?;
            if items.insert(def.name.clone(), value).is_some() {
                return None;
            }
        }
        for field in fields {
            if !items.contains_key(&field.name) {
                let zero = match self.facts.type_of(&field.ty)?.kind {
                    TypeKind::Integer => Scalar::Int(0),
                    TypeKind::Bool => Scalar::Bool(false),
                    TypeKind::String => Scalar::Text(Vec::new()),
                    _ => return None,
                };
                items.insert(field.name.clone(), zero);
            }
        }
        Some(Scalar::Record(Arc::new(items)))
    }
}
