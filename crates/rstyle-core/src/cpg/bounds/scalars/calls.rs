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
            return self.indirect_call(node, depth);
        }
        if reference.status != RefStatus::Resolved {
            return None;
        }
        let (_, def) = self.facts.resolved(node)?;
        let callee = self.child(node, 0)?;
        if def.kind == DefinitionKind::Method
            && matches!(
                reference.dispatch,
                Some(Dispatch::Method | Dispatch::Dynamic)
            )
        {
            let target = self.method_receiver((callee, def), depth)?;
            return self.invoke_method((node, 1), target, depth);
        }
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
            return self.standard_call(node, def.operation.as_deref()?, depth);
        }
        if def.standard && reference.dispatch == Some(Dispatch::Static) {
            return self.math_call(node, def.operation.as_deref()?, depth);
        }
        if reference.dispatch != Some(Dispatch::Static)
            || !matches!(def.kind, DefinitionKind::Method | DefinitionKind::Function)
        {
            return None;
        }
        self.static_call(node, def, depth)
    }
    fn static_call(
        &mut self,
        node: &'a Node,
        def: &'a crate::cpg::model::Definition,
        depth: usize,
    ) -> Option<Scalar> {
        if def.kind == DefinitionKind::Method
            && def
                .callable
                .as_ref()?
                .receiver
                .as_deref()
                .and_then(|id| self.facts.type_of(id))
                .is_some_and(|r| r.kind == TypeKind::Interface)
        {
            let target = self.method_receiver((self.child(node, 1)?, def), depth)?;
            return self.invoke_method((node, 2), target, depth);
        }
        self.direct(node, self.facts.tables.function(&def.id)?, depth)
    }

    fn standard_call(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        // Go does not evaluate the operand of a compile-time layout query.
        if self
            .profile
            .constant_layout_queries
            .iter()
            .any(|name| name == op)
            && node.children.len() == 2
            && self.uintptr(self.facts.ty(node)?)
        {
            let fact = self.constants.get(node.id.as_str())?;
            if fact.ty != node.ty || fact.value.kind != "integer" {
                return None;
            }
            return self.literal(node, &fact.value);
        }
        if op == self.profile.complex_value
            || op == self.profile.real_component
            || op == self.profile.imaginary_component
        {
            return self.complex_builtin(node, op, depth);
        }
        if [
            self.profile.unsafe_add.as_str(),
            &self.profile.unsafe_slice,
            &self.profile.unsafe_slice_data,
            &self.profile.unsafe_string,
            &self.profile.unsafe_string_data,
        ]
        .contains(&op)
        {
            return self.memory_call(node, op, depth);
        }
        self.builtin(node, op, depth)
    }
    fn indirect_call(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let callee = self.child(node, 0)?;
        self.facts.local(callee)?;
        match self.value(callee, depth + 1)? {
            Scalar::Callable(id) => self.direct(node, self.facts.tables.function(&id)?, depth),
            Scalar::BoundMethod(id, r) => {
                self.invoke_method((node, 1), (self.facts.tables.function(&id)?, *r), depth)
            }
            _ => None,
        }
    }
    fn direct(&mut self, node: &'a Node, func: &'a Function, depth: usize) -> Option<Scalar> {
        let signature = &func.signature;
        if !signature.generics.is_empty()
            || !signature.modifiers.is_empty()
            || signature.parameters.len() + 1 + usize::from(signature.receiver.is_some())
                != node.children.len()
        {
            return None;
        }
        let inputs = self.arguments(node, signature, depth)?;
        self.invoke(func, inputs, depth)
    }

    fn invoke(
        &mut self,
        func: &'a Function,
        inputs: Vec<(&'a str, Scalar)>,
        depth: usize,
    ) -> Option<Scalar> {
        if func.nodes.len() > 128
            || depth >= 30
            || !func.signature.modifiers.is_empty()
            || !func.signature.generics.is_empty()
        {
            return None;
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
        child.returned.filter(|value| {
            matches!(
                value,
                Scalar::Int(_) | Scalar::Text(_) | Scalar::Bool(_) | Scalar::Real(_)
            )
        })
    }
    fn method_receiver(
        &mut self,
        (expr, def): (&'a Node, &'a crate::cpg::model::Definition),
        depth: usize,
    ) -> Option<(&'a Function, Scalar)> {
        if def.kind != DefinitionKind::Method {
            return None;
        }
        let mut ty = self.facts.ty(expr)?;
        let mut value = self.value(expr, depth + 1)?;
        let mut target = def;
        if let Scalar::Interface(Some(pair)) = value {
            if def.callable.as_ref()?.receiver.as_ref()? != &ty.id {
                return None;
            }
            let targets = self.facts.tables.extension(&self.profile.method_targets)?;
            let declared = targets.get(&ty.id)?.as_object()?;
            let mut names = declared
                .iter()
                .filter(|(_, id)| id.as_str() == Some(&def.id));
            let (name, _) = names.next()?;
            if names.next().is_some() {
                return None;
            }
            ty = self.facts.type_of(&pair.0)?;
            target = self
                .facts
                .tables
                .definition(targets.get(&ty.id)?.get(name)?.as_str()?)
                .ok()?;
            value = pair.1;
        }
        let func = self.facts.tables.function(&target.id)?;
        let receiver = func.signature.receiver.as_ref()?;
        if let Scalar::Pointer(Some(addr)) = &value
            && ty.kind == TypeKind::Reference
            && ty.target.as_ref() == Some(&receiver.ty)
        {
            value = self.pointed(addr)?;
            ty = self.facts.type_of(&receiver.ty)?;
        }
        (receiver.ty == ty.id
            && matches!(
                value,
                Scalar::Int(_) | Scalar::Real(_) | Scalar::Bool(_) | Scalar::Text(_)
            ))
        .then_some((func, value))
    }

    pub(super) fn method_value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        if self.facts.reference(node)?.status != RefStatus::Resolved {
            return None;
        }
        let def = self.facts.resolved(node)?.1;
        let (function, receiver) = self.method_receiver((self.child(node, 0)?, def), depth)?;
        Some(Scalar::BoundMethod(
            function.definition.clone(),
            Box::new(receiver),
        ))
    }

    fn invoke_method(
        &mut self,
        (node, offset): (&'a Node, usize),
        (func, receiver): (&'a Function, Scalar),
        depth: usize,
    ) -> Option<Scalar> {
        let sig = &func.signature;
        if sig.parameters.len() + offset != node.children.len()
            || node.ty.as_ref() != Some(&sig.result)
        {
            return None;
        }
        let mut args = vec![(sig.receiver.as_ref()?.symbol.as_str(), receiver)];
        for (i, p) in sig.parameters.iter().enumerate() {
            let arg = self.child(node, offset + i)?;
            if p.position != i || arg.ty.as_ref() != Some(&p.ty) {
                return None;
            }
            let v = self.value(arg, depth + 1)?;
            if !matches!(
                v,
                Scalar::Int(_) | Scalar::Real(_) | Scalar::Bool(_) | Scalar::Text(_)
            ) {
                return None;
            }
            args.push((&p.symbol, v));
        }
        self.invoke(func, args, depth)
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
