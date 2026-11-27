//! Pure resolved calls with known arguments. Share the remaining work budget down the
//! call chain. No expression in a callee can write its caller's state.
use super::{DefinitionKind, Facts, Node, NodeKind, RefStatus, Scalar, Walk};
use crate::cpg::model::{Dispatch, Function, Signature, TypeKind};
use crate::cpg::tree::Body;

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn call(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let reference = self.facts.reference(node)?;
        if reference.arguments.is_some() {
            return None;
        }
        if reference.dispatch == Some(Dispatch::FunctionValue)
            && reference.target.is_none()
            && reference.selected.is_none()
        {
            let callee = self.child(node, 0)?;
            self.facts.local(callee)?;
            let Scalar::Callable(id) = self.value(callee, depth + 1)? else {
                return None;
            };
            return self.direct(node, self.facts.tables.function(&id)?, depth);
        }
        if reference.status != RefStatus::Resolved {
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
            let op = def.operation.as_deref()?;
            if op == self.profile.complex_value
                || op == self.profile.real_component
                || op == self.profile.imaginary_component
            {
                return self.complex_builtin(node, op, depth);
            }
            return self.builtin(node, op, depth);
        }
        if def.standard && reference.dispatch == Some(Dispatch::Static) {
            return self.math_call(node, def.operation.as_deref()?, depth);
        }
        if reference.dispatch != Some(Dispatch::Static)
            || !matches!(def.kind, DefinitionKind::Method | DefinitionKind::Function)
        {
            return None;
        }
        self.direct(node, self.facts.tables.function(&def.id)?, depth)
    }

    fn direct(&mut self, node: &'a Node, func: &'a Function, depth: usize) -> Option<Scalar> {
        let signature = &func.signature;
        if !signature.generics.is_empty()
            || !signature.modifiers.is_empty()
            || signature.parameters.len() + 1 + usize::from(signature.receiver.is_some())
                != node.children.len()
            || func.nodes.len() > 128
            || depth >= 30
        {
            return None;
        }
        let inputs = self.arguments(node, signature, depth)?;
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
        child.returned.filter(|value| {
            matches!(
                value,
                Scalar::Int(_) | Scalar::Text(_) | Scalar::Bool(_) | Scalar::Real(_)
            )
        })
    }
    fn arguments(
        &mut self,
        node: &'a Node,
        signature: &'a Signature,
        depth: usize,
    ) -> Option<Vec<(&'a str, Scalar)>> {
        if signature
            .parameters
            .iter()
            .enumerate()
            .any(|(i, p)| p.position != i)
        {
            return None;
        }
        let receiver = signature.receiver.iter().map(|p| (&p.symbol, &p.ty));
        let parameters = signature.parameters.iter().map(|p| (&p.symbol, &p.ty));
        let mut args = Vec::new();
        for ((symbol, ty), id) in receiver.chain(parameters).zip(&node.children[1..]) {
            let arg = self.facts.body.node(id).ok()?;
            if arg.ty.as_ref() != Some(ty)
                || !matches!(
                    self.facts.ty(arg)?.kind,
                    TypeKind::Integer | TypeKind::Bool | TypeKind::String | TypeKind::Float
                )
            {
                return None;
            }
            args.push((symbol.as_str(), self.value(arg, depth + 1)?));
        }
        Some(args)
    }
}
