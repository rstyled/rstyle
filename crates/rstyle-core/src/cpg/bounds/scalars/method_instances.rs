//! Specialize a direct call of an explicitly instantiated Go method value.
//! Receiver and method parameters have separate numbering, so key the map by
//! the source parameter's type identity rather than its position alone.
use super::{Node, Scalar, TypeKind, Walk};
use crate::cpg::model::{Function, Ref, Type};
use std::collections::BTreeMap;

type Substitutions = BTreeMap<String, String>;

fn receiver_args(formal: &Type, actual: &Type) -> Option<Substitutions> {
    if formal.definition.is_none() || formal.definition != actual.definition {
        return None;
    }
    let fs = formal.arguments.as_deref().unwrap_or_default();
    let xs = actual.arguments.as_deref().unwrap_or_default();
    if fs.len() != xs.len() {
        return None;
    }
    let mut map: Substitutions = fs.iter().cloned().zip(xs.iter().cloned()).collect();
    map.insert(formal.id.clone(), actual.id.clone());
    Some(map)
}

impl<'a> Walk<'_, '_, 'a> {
    fn method_args(&self, func: &Function, call: &Ref, subst: &mut Substitutions) -> Option<()> {
        let xs = call.instance_arguments.as_ref()?;
        if xs.len() != func.signature.generics.len() {
            return None;
        }
        let decls = self.facts.tables.extension("go:lexical-bindings")?;
        for (i, (generic, actual)) in func.signature.generics.iter().zip(xs).enumerate() {
            let span = generic.span.as_ref()?;
            let decl = decls.get(format!("{}:{}", span.file, span.start))?;
            let ty = decl.get("type")?.as_str()?;
            if decl.get("kind")?.as_str()? != "type_parameter"
                || generic.position != Some(i as u32)
                || self.facts.type_of(ty)?.kind != TypeKind::Parameter
                || subst.insert(ty.to_owned(), actual.clone()).is_some()
            {
                return None;
            }
        }
        Some(())
    }

    fn concrete_method(&mut self, func: &Function, call: &Ref, actual: &Type) -> Option<Function> {
        let sig = &func.signature;
        let recv = self.facts.type_of(&sig.receiver.as_ref()?.ty)?;
        if func.nodes.len() > 128 || sig.generics.is_empty() {
            return None;
        }
        self.remaining = self.remaining.checked_sub(func.nodes.len())?;
        let mut subst = receiver_args(recv, actual)?;
        self.method_args(func, call, &mut subst)?;
        let signature = call.signature.as_ref()?;
        let mut frame = func.clone();
        frame.signature.generics.clear();
        frame.values.clear();
        let recv = &mut frame.signature.receiver.as_mut()?.ty;
        *recv = actual.id.clone();
        frame.signature.owner_type = Some(actual.id.clone());
        let s = &mut frame.signature;
        if s.parameters.len() != signature.parameters.len() {
            return None;
        }
        for id in s
            .parameters
            .iter_mut()
            .map(|p| &mut p.ty)
            .chain(std::iter::once(&mut s.result))
            .chain(frame.nodes.iter_mut().filter_map(|n| n.ty.as_mut()))
            .chain(frame.symbols.iter_mut().filter_map(|s| s.ty.as_mut()))
        {
            if let Some(value) = subst.get(id) {
                id.clone_from(value);
            } else if self.facts.type_of(id)?.kind == TypeKind::Parameter {
                return None;
            }
        }
        if signature.result.as_ref() != Some(&s.result)
            || !s
                .parameters
                .iter()
                .zip(&signature.parameters)
                .all(|(p, t)| Some(&p.ty) == t.as_ref())
        {
            return None;
        }
        Some(frame)
    }

    pub(super) fn explicit_method(
        &mut self,
        node: &'a Node,
        sel: &'a Node,
        depth: usize,
    ) -> Option<Scalar> {
        let (_, def) = self.facts.resolved(node)?;
        let call = self.facts.reference(node)?;
        let (_, target) = self.facts.resolved(sel)?;
        let method = self.facts.reference(sel)?;
        if target.id != def.id
            || method.instance_arguments != call.instance_arguments
            || sel.children.len() != 1
        {
            return None;
        }
        let recv = self.child(sel, 0)?;
        let func = self.facts.tables.function(&def.id)?;
        let frame = self.concrete_method(func, call, self.facts.ty(recv)?)?;
        let value = self.value(recv, depth + 1)?;
        if !self.transferable(&value) {
            return None;
        }
        self.invoke_method((node, 1), (&frame, value), depth)
    }
}
