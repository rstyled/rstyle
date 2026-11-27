//! A closure's captured variables retain identity across calls. A returned closure can
//! carry a bounded set of private scalar locals from an activation that has finished.
use super::{Facts, Node, NodeKind, Scalar, TypeKind, Walk, frames::primitive};
use crate::cpg::tree::Body;
use std::collections::BTreeMap;

pub(super) type Environment = BTreeMap<String, Scalar>;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::cpg::bounds) struct Closure {
    owner: String,
    node: String,
    environment: Option<usize>,
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn closure_value(&self, node: &Node) -> Scalar {
        Scalar::ClosureValue(Closure {
            owner: self.facts.body.function.definition.clone(),
            node: node.id.clone(),
            environment: None,
        })
    }

    pub(super) fn save_environment(
        &mut self,
        child: &Walk<'_, '_, '_>,
        mut c: Closure,
    ) -> Option<Scalar> {
        if c.environment.is_some()
            || c.owner != child.facts.body.function.definition
            || self.environments.len() >= super::storage::MAX_ITEMS
        {
            return None;
        }
        let node = child.facts.body.node(&c.node).ok()?;
        let mut env = Environment::new();
        for capture in node.captures.as_deref().unwrap_or_default() {
            let value = child.local_value(&capture.symbol)?;
            if !primitive(&value)
                || !capture.path.is_empty()
                || env.insert(capture.symbol.clone(), value).is_some()
            {
                return None;
            }
        }
        c.environment = Some(self.environments.len());
        self.environments.push(env);
        Some(Scalar::ClosureValue(c))
    }

    fn capture_input(&self, c: &Closure, symbol: &str) -> Option<Scalar> {
        match c.environment {
            Some(slot) => self.environments.get(slot)?.get(symbol).cloned(),
            None if c.owner == self.facts.body.function.definition => self.local_value(symbol),
            _ => None,
        }
    }

    fn start_literal(&mut self, literal: &'a Node) -> Option<()> {
        self.inside_literal = true;
        let Some(results) = self
            .facts
            .body
            .function
            .extensions
            .as_ref()
            .and_then(|ext| ext.get(&self.profile.named_results))
            .and_then(|by_closure| by_closure.get(&literal.id))
        else {
            return Some(());
        };
        let [id] = results.as_array()?.as_slice() else {
            return None;
        };
        let sym = self.facts.body.symbol(id.as_str()?).ok()?;
        if sym.ty != self.facts.ty(literal)?.result {
            return None;
        }
        self.named_return = Some(&sym.id);
        self.locals
            .insert(&sym.id, self.zero(self.facts.type_of(sym.ty.as_deref()?)?)?);
        Some(())
    }

    fn literal_locals<'n>(
        &mut self,
        (node, literal): (&'a Node, &'n Node),
        child: &mut Walk<'_, '_, 'n>,
        depth: usize,
    ) -> Option<()> {
        let shape = child.facts.ty(literal)?;
        let params = shape.parameters.as_deref()?;
        if shape.kind != TypeKind::Function
            || shape.result != node.ty
            || params.len() + 1 != node.children.len()
            || node.children.len() != literal.children.len()
        {
            return None;
        }
        for (i, expected) in params.iter().enumerate() {
            let bind = child.child(literal, i)?;
            let arg = self.child(node, i + 1)?;
            if bind.ty.as_ref() != Some(expected) || arg.ty != bind.ty {
                return None;
            }
            let v = self.value(arg, depth + 1)?;
            if !primitive(&v) {
                return None;
            }
            child.locals.insert(child.binding(bind)?, v);
        }
        Some(())
    }
    fn bind_captures<'n>(
        &self,
        (literal, c): (&'n Node, &Closure),
        child: &mut Walk<'_, '_, 'n>,
    ) -> Option<()> {
        for capture in literal.captures.as_deref().unwrap_or_default() {
            let sym = child.facts.body.symbol(&capture.symbol).ok()?;
            let v = self.capture_input(c, &sym.id)?;
            if sym.ty.as_ref() != Some(&capture.ty)
                || !capture.path.is_empty()
                || !primitive(&v)
                || child.locals.insert(&sym.id, v).is_some()
            {
                return None;
            }
        }
        Some(())
    }

    fn update_captures(
        &mut self,
        child: &Walk<'_, '_, '_>,
        (literal, c): (&Node, &Closure),
        writes: bool,
    ) -> Option<()> {
        for capture in literal.captures.as_deref().unwrap_or_default() {
            let current = self.capture_input(c, &capture.symbol)?;
            let value = child.local_value(&capture.symbol)?;
            if value != current {
                if !writes || !primitive(&value) || capture.mode != "exclusive" {
                    return None;
                }
                match c.environment {
                    Some(slot) => {
                        *self.environments.get_mut(slot)?.get_mut(&capture.symbol)? = value
                    }
                    None => {
                        let (&sym, _) = self.locals.get_key_value(capture.symbol.as_str())?;
                        self.store(sym, value)?;
                    }
                }
            }
        }
        Some(())
    }

    pub(super) fn invoke_literal(
        &mut self,
        node: &'a Node,
        c: &Closure,
        depth: usize,
    ) -> Option<Scalar> {
        let owner = self.facts.tables.function(&c.owner)?;
        if owner.nodes.len() > 256 || !owner.signature.generics.is_empty() || depth >= 30 {
            return None;
        }
        self.remaining = self.remaining.checked_sub(owner.nodes.len())?;
        let body = Body::new(owner).ok()?;
        let literal = body.node(&c.node).ok()?;
        let block = body.node(literal.children.last()?).ok()?;
        if literal.kind != NodeKind::Closure || block.kind != NodeKind::Block {
            return None;
        }
        let facts = Facts {
            body: &body,
            tables: self.facts.tables,
            sources: self.facts.sources,
            pointer_bits: self.facts.pointer_bits,
            destructors_are_calls: self.facts.destructors_are_calls,
        };
        let mut child = Walk::new(&facts, self.profile, self.remaining);
        self.literal_locals((node, literal), &mut child, depth)?;
        self.bind_captures((literal, c), &mut child)?;
        child.remaining = self.remaining;
        child.start_literal(literal)?;
        child.statement(block, depth + 1);
        self.remaining = child.remaining;
        let result = self.frame_result(&mut child, (false, false, false))?;
        self.update_captures(&child, (literal, c), self.write_at == Some(&node.id))?;
        Some(result)
    }
}
