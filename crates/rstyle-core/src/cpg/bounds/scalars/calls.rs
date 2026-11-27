//! Pure resolved calls with known arguments. Share the remaining work budget down the
//! call chain. No expression in a callee can write its caller's state.
use super::{DefinitionKind, Facts, Node, NodeKind, RefStatus, Scalar, Walk};
use crate::cpg::model::{Dispatch, TypeKind};
use crate::cpg::tree::Body;

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn call(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let reference = self.facts.reference(node)?;
        if reference.status != RefStatus::Resolved || reference.arguments.is_some() {
            return None;
        }
        let (_, def) = self.facts.resolved(node)?;
        let callee = self.child(node, 0)?;
        // A function-valued expression can run arbitrary code before it yields a callee.
        if self.facts.resolved(callee)?.1.id != def.id
            || callee.kind != NodeKind::Identifier
            || !callee.children.is_empty()
        {
            return None;
        }
        if def.standard
            && def.kind == DefinitionKind::Function
            && reference.dispatch == Some(Dispatch::Builtin)
        {
            return self.builtin(node, def.operation.as_deref()?, depth);
        }
        if reference.dispatch != Some(Dispatch::Static) || def.kind != DefinitionKind::Function {
            return None;
        }
        let func = self.facts.tables.function(&def.id)?;
        let signature = &func.signature;
        if !signature.generics.is_empty()
            || signature.receiver.is_some()
            || !signature.modifiers.is_empty()
            || signature.parameters.len() + 1 != node.children.len()
            || func.nodes.len() > 128
            || depth >= 30
        {
            return None;
        }
        let mut inputs = Vec::new();
        for (i, param) in signature.parameters.iter().enumerate() {
            let arg = self.child(node, i + 1)?;
            if param.position != i
                || arg.ty.as_deref() != Some(&param.ty)
                || !matches!(
                    self.facts.ty(arg)?.kind,
                    TypeKind::Integer | TypeKind::Bool | TypeKind::String | TypeKind::Float
                )
            {
                return None;
            }
            inputs.push((param.symbol.as_str(), self.value(arg, depth + 1)?));
        }
        // Indexing another body also consumes work, even when it immediately stops.
        self.remaining = self.remaining.checked_sub(func.nodes.len())?;
        let body = Body::new(func).ok()?;
        let facts = Facts {
            body: &body,
            tables: self.facts.tables,
            sources: self.facts.sources,
            pointer_bits: self.facts.pointer_bits,
            destructors_are_calls: self.facts.destructors_are_calls,
        };
        let mut child = Walk::new(&facts, self.profile, self.remaining);
        child.locals.extend(inputs);
        child.statement(body.root().ok()?, depth + 1);
        self.remaining = child.remaining;
        child
            .returned
            .filter(|value| matches!(value, Scalar::Int(_) | Scalar::Text(_) | Scalar::Bool(_)))
    }
}
