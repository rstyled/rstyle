//! Resolved call dispatch with a shared work budget for nested activations.
use super::{DefinitionKind, Node, NodeKind, RefStatus, Scalar, Walk};
use crate::cpg::model::{Definition, Dispatch, Function, Ref, TypeKind};

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
        self.resolved_call(node, (reference, def), depth)
    }
    fn resolved_call(
        &mut self,
        node: &'a Node,
        (reference, def): (&'a Ref, &'a Definition),
        depth: usize,
    ) -> Option<Scalar> {
        let callee = self.child(node, 0)?;
        if def.kind == DefinitionKind::Method
            && reference.dispatch == Some(Dispatch::Static)
            && callee.extension.as_deref() == Some(&self.profile.method_value)
        {
            return self.explicit_method(node, callee, depth);
        }
        if def.standard
            && def
                .operation
                .as_deref()
                .is_some_and(|op| self.reflection_operation(op))
        {
            return self.reflection_call(node, def, depth);
        }
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
        // A native constant fact for one of these resolved standard operations
        // establishes that the operand is not evaluated, even if its selector
        // would dereference a nil array pointer when evaluated at run time.
        let ty = self.facts.ty(node)?;
        let extent = (op == self.profile.byte_length || op == self.profile.capacity)
            && self.facts.integer(Some(ty)).is_some_and(|n| n.signed);
        let layout = self
            .profile
            .constant_layout_queries
            .iter()
            .any(|name| name == op)
            && self.uintptr(ty);
        if (extent || layout)
            && node.children.len() == 2
            && let Some(fact) = self.constants.get(node.id.as_str())
            && fact.ty == node.ty
            && fact.value.kind == "integer"
        {
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
        if self.facts.local(callee).is_none() && callee.kind != NodeKind::Closure {
            return None;
        }
        match self.value(callee, depth + 1)? {
            Scalar::ClosureValue(c) => self.invoke_literal(node, &c, depth),
            Scalar::Callable(id) => self.direct(node, self.facts.tables.function(&id)?, depth),
            Scalar::BoundMethod(id, r) => {
                self.invoke_method((node, 1), (self.facts.tables.function(&id)?, *r), depth)
            }
            _ => None,
        }
    }
    fn direct(&mut self, node: &'a Node, func: &'a Function, depth: usize) -> Option<Scalar> {
        let instantiated;
        let func = if func.signature.generics.is_empty() {
            func
        } else {
            instantiated = self.instantiate(func, self.facts.reference(node)?)?;
            &instantiated
        };
        if !self.result_matches(node, &func.signature.result)? {
            return None;
        }
        let inputs = self.run_arguments(node, &func.signature, depth)?;
        self.invoke_frame(func, inputs, (depth, self.write_at == Some(&node.id)))
    }
    fn result_matches(&self, node: &Node, result: &str) -> Option<bool> {
        let ty = self.facts.ty(node)?;
        if ty.id == result {
            return Some(true);
        }
        let declared = self.facts.type_of(result)?;
        // Named result variables can give the call and declaration distinct tuple IDs.
        // A tuple has no independent Go type identity beyond its ordered elements.
        Some(
            ty.kind == TypeKind::Tuple
                && declared.kind == TypeKind::Tuple
                && ty.items.as_ref()? == declared.items.as_ref()?,
        )
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
        let value = self.receiver_value((expr, ty), (&receiver.ty, value), depth)?;
        Some((func, value))
    }

    pub(super) fn receiver_value(
        &mut self,
        (expr, ty): (&'a Node, &crate::cpg::model::Type),
        (expected, value): (&str, Scalar),
        depth: usize,
    ) -> Option<Scalar> {
        if expected == ty.id && self.transferable(&value) {
            return Some(value);
        }
        if ty.target.as_deref() == Some(expected) && ty.kind == TypeKind::Reference {
            let Scalar::Pointer(Some(addr)) = value else {
                return None;
            };
            let v = self.pointed(&addr)?;
            return self.transferable(&v).then_some(v);
        }
        let dst = self.facts.type_of(expected)?;
        if expr.kind == NodeKind::Identifier
            && dst.kind == TypeKind::Reference
            && dst.target.as_deref() == Some(&ty.id)
        {
            let address = self.address(expr, depth + 1)?;
            return Some(Scalar::Pointer(Some(address)));
        }
        None
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

    pub(super) fn invoke_method(
        &mut self,
        (node, offset): (&'a Node, usize),
        (func, receiver): (&Function, Scalar),
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
        self.invoke_frame(func, args, (depth, self.write_at == Some(&node.id)))
    }
}
