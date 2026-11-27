//! Monomorphic, read-only Go call frames. A slice argument keeps its backing storage,
//! including when forwarded as the return value. A fresh variadic pack gets a new buffer.
use super::{Node, Scalar, TypeKind, Walk, pointers::Address};
use crate::cpg::model::{Function, Ref, Signature};

pub(super) fn primitive(v: &Scalar) -> bool {
    matches!(
        v,
        Scalar::Int(_)
            | Scalar::Bool(_)
            | Scalar::Real(_)
            | Scalar::Complex(_, _)
            | Scalar::Text(_)
    )
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn transferable(&self, v: &Scalar) -> bool {
        match v {
            Scalar::Pointer(None) => true,
            Scalar::Pointer(Some(a @ Address::Local(_))) => {
                self.pointed(a).is_some_and(|x| primitive(&x))
            }
            Scalar::Record(fields) => fields.values().all(primitive),
            Scalar::Sequence(a) => a.iter().all(primitive),
            Scalar::Slice(view) => view.slot().is_none_or(|slot| {
                self.buffers
                    .get(slot)
                    .is_some_and(|cells| cells.iter().all(primitive))
            }),
            Scalar::Map(slot) => slot.is_none_or(|i| {
                self.maps
                    .get(i)
                    .is_some_and(super::maps::transferable_items)
            }),
            other => primitive(other),
        }
    }

    pub(super) fn run_arguments<'f>(
        &mut self,
        node: &'a Node,
        sig: &'f Signature,
        depth: usize,
    ) -> Option<Vec<(&'f str, Scalar)>> {
        if sig.receiver.is_none()
            && node.children.len() == 2
            && self.child(node, 1)?.kind == super::NodeKind::Call
            && self.facts.ty(self.child(node, 1)?)?.kind == TypeKind::Tuple
        {
            return self.forward_results(node, sig, depth);
        }
        let variadic = sig.modifiers == ["variadic"];
        let fixed = sig.parameters.len().checked_sub(usize::from(variadic))?;
        let receiver = sig.receiver.iter().map(|r| (&r.symbol, &r.ty));
        let formals = sig
            .parameters
            .iter()
            .take(fixed)
            .map(|p| (&p.symbol, &p.ty));
        let offset = 1 + usize::from(sig.receiver.is_some()) + fixed;
        if sig
            .parameters
            .iter()
            .enumerate()
            .any(|(i, p)| i != p.position)
            || node.children.len() < offset
            || (!variadic && node.children.len() != offset)
        {
            return None;
        }
        let mut args = Vec::new();
        for (index, ((sym, ty), id)) in receiver
            .chain(formals)
            .zip(node.children.iter().skip(1))
            .enumerate()
        {
            let child = self.facts.body.node(id).ok()?;
            let mut v = self.value(child, depth + 1)?;
            if sig.receiver.is_some() && index == 0 {
                v = self.receiver_value((child, self.facts.ty(child)?), (ty, v), depth)?;
            } else if child.ty.as_ref() != Some(ty) {
                return None;
            }
            if !self.transferable(&v) {
                return None;
            }
            args.push((sym.as_str(), v));
        }
        if variadic {
            let last = sig.parameters.last()?;
            let packed = self.variadic(node, (&last.ty, offset), depth)?;
            args.push((&last.symbol, packed));
        }
        Some(args)
    }

    pub(super) fn tuple(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let ty = self.facts.ty(node)?;
        let items = ty.items.as_deref()?;
        if ty.kind != TypeKind::Tuple || node.children.len() != items.len() || items.len() > 32 {
            return None;
        }
        let mut out = Vec::new();
        for (id, expected) in node.children.iter().zip(items) {
            let child = self.facts.body.node(id).ok()?;
            if child.ty.as_ref() != Some(expected) {
                return None;
            }
            let x = self.value(child, depth + 1)?;
            if !primitive(&x) {
                return None;
            }
            out.push(x);
        }
        Some(Scalar::Tuple(out))
    }

    fn forward_results<'s>(
        &mut self,
        node: &'a Node,
        sig: &'s Signature,
        depth: usize,
    ) -> Option<Vec<(&'s str, Scalar)>> {
        let arg = self.child(node, 1)?;
        let items = self.facts.ty(arg)?.items.as_deref()?;
        let Scalar::Tuple(results) = self.value(arg, depth + 1)? else {
            return None;
        };
        let variadic = sig.modifiers == ["variadic"];
        let fixed = sig.parameters.len().checked_sub(usize::from(variadic))?;
        if results.len() != items.len()
            || items.len() < fixed
            || (!variadic && items.len() != fixed)
        {
            return None;
        }
        let mut out = Vec::new();
        for (i, parm) in sig.parameters.iter().take(fixed).enumerate() {
            if parm.position != i || items[i] != parm.ty {
                return None;
            }
            out.push((parm.symbol.as_str(), results[i].clone()));
        }
        if variadic {
            let spread = self
                .facts
                .tables
                .extension(&self.profile.spread_arguments)?
                .get(&node.id)?
                .as_array()?;
            let last = sig.parameters.last()?;
            let par = self.facts.type_of(&last.ty)?;
            let tail = &results[fixed..];
            if last.position != fixed
                || !spread.is_empty()
                || par.kind != TypeKind::Sequence
                || par.fixed != Some(false)
                || items[fixed..]
                    .iter()
                    .any(|t| Some(t) != par.element.as_ref())
            {
                return None;
            }
            let pack = if tail.is_empty() {
                Scalar::Slice(super::storage::View::nil())
            } else {
                self.fresh_slice(tail.to_vec(), Some(tail.len()))?
            };
            out.push((&last.symbol, pack));
        }
        Some(out)
    }

    fn variadic(
        &mut self,
        node: &'a Node,
        (ty, offset): (&str, usize),
        depth: usize,
    ) -> Option<Scalar> {
        let spread = self
            .facts
            .tables
            .extension(&self.profile.spread_arguments)?
            .get(&node.id)?
            .as_array()?;
        let tail = node.children.get(offset..)?;
        if !spread.is_empty() {
            if spread.len() != 1 || tail.len() != 1 || spread[0].as_str() != Some(tail[0].as_str())
            {
                return None;
            }
            let arg = self.facts.body.node(&tail[0]).ok()?;
            if arg.ty.as_deref() != Some(ty) {
                return None;
            }
            let v = self.value(arg, depth + 1)?;
            return self.transferable(&v).then_some(v);
        }
        if tail.is_empty() {
            return Some(Scalar::Slice(super::storage::View::nil()));
        }
        let slice = self.facts.type_of(ty)?;
        if slice.kind != TypeKind::Sequence || slice.fixed != Some(false) {
            return None;
        }
        let mut items = Vec::new();
        for id in tail {
            let arg = self.facts.body.node(id).ok()?;
            if arg.ty != slice.element {
                return None;
            }
            let v = self.value(arg, depth + 1)?;
            if !primitive(&v) {
                return None;
            }
            items.push(v);
        }
        self.fresh_slice(items, Some(tail.len()))
    }

    fn type_arg(&self, id: &str, args: &[String]) -> Option<String> {
        let ty = self.facts.type_of(id)?;
        if ty.kind == TypeKind::Parameter {
            return args.get(usize::try_from(ty.index?).ok()?).cloned();
        }
        Some(id.to_owned())
    }

    pub(super) fn instantiate(&mut self, func: &Function, reference: &Ref) -> Option<Function> {
        let args = reference.instance_arguments.as_ref()?;
        let selected = reference.signature.as_ref()?;
        let sig = &func.signature;
        if args.is_empty()
            || args.len() != sig.generics.len()
            || sig.receiver.is_some()
            || !sig.modifiers.is_empty()
            || sig
                .generics
                .iter()
                .enumerate()
                .any(|(i, p)| p.position != Some(i as u32))
            || sig.parameters.len() != selected.parameters.len()
            || func.nodes.len() > 128
        {
            return None;
        }
        self.remaining = self.remaining.checked_sub(func.nodes.len())?;
        let mut frame = func.clone();
        frame.signature.generics.clear();
        for (i, parm) in frame.signature.parameters.iter_mut().enumerate() {
            parm.ty = self.type_arg(&parm.ty, args)?;
            if selected.parameters[i].as_ref() != Some(&parm.ty) {
                return None;
            }
        }
        frame.signature.result = self.type_arg(&sig.result, args)?;
        if selected.result.as_ref() != Some(&frame.signature.result) {
            return None;
        }
        for id in frame
            .nodes
            .iter_mut()
            .filter_map(|n| n.ty.as_mut())
            .chain(frame.symbols.iter_mut().filter_map(|s| s.ty.as_mut()))
        {
            *id = self.type_arg(id, args)?;
        }
        // A fact computed for a generic expression must not be reused as if the
        // compiler had evaluated it at this particular width.
        frame.values.clear();
        Some(frame)
    }

    fn import_pointers(&self, args: &mut [(&str, Scalar)]) -> Option<Vec<(Address, Scalar)>> {
        // Each distinct caller local maps to one cell shared by all corresponding formals.
        // Other aggregate aliases need a common address space across frames.
        let mut cells = Vec::new();
        for (_, value) in args.iter_mut() {
            if let Scalar::Pointer(Some(a)) = value {
                if !matches!(a, Address::Local(_)) {
                    return None;
                }
                let cell = if let Some(i) = cells.iter().position(|(orig, _)| orig == a) {
                    i
                } else {
                    let v = self.pointed(a)?;
                    if !primitive(&v) {
                        return None;
                    }
                    cells.push((a.clone(), v));
                    cells.len() - 1
                };
                *value = Scalar::Pointer(Some(Address::Heap(cell)));
            }
        }
        if !cells.is_empty()
            && args
                .iter()
                .any(|(_, v)| matches!(v, Scalar::Slice(_) | Scalar::Map(_) | Scalar::Sequence(_)))
        {
            return None;
        }
        Some(cells)
    }

    fn export_pointers(
        &mut self,
        cells: &[(Address, Scalar)],
        heap: &[Scalar],
        allow_writes: bool,
    ) -> Option<()> {
        for ((addr, initial), val) in cells.iter().zip(heap) {
            if initial != val {
                if !allow_writes || !primitive(val) {
                    return None;
                }
                self.write_address(addr, val.clone())?;
            }
        }
        Some(())
    }

    pub(super) fn invoke_frame(
        &mut self,
        func: &Function,
        mut args: Vec<(&str, Scalar)>,
        (depth, allow_writes): (usize, bool),
    ) -> Option<Scalar> {
        use super::Facts;
        use crate::cpg::tree::Body;
        if func.nodes.len() > 128
            || depth >= 30
            || !func.signature.generics.is_empty()
            || !(func.signature.modifiers.is_empty() || func.signature.modifiers == ["variadic"])
        {
            return None;
        }
        self.remaining = self.remaining.checked_sub(func.nodes.len())?;
        let body = Body::new(func).ok()?;
        let facts = Facts {
            body: &body,
            tables: self.facts.tables,
            sources: self.facts.sources,
            pointer_bits: self.facts.pointer_bits,
            destructors_are_calls: self.facts.destructors_are_calls,
        };
        let reads_buffers = args.iter().any(|(_, v)| matches!(v, Scalar::Slice(_)));
        let reads_maps = args.iter().any(|(_, v)| matches!(v, Scalar::Map(_)));
        let mut child = Walk::new(&facts, self.profile, self.remaining);
        if reads_buffers {
            child.buffers = self.buffers.clone();
        }
        if reads_maps {
            child.remaining = child
                .remaining
                .checked_sub(self.maps.iter().map(|m| m.len() + 1).sum())?;
            child.maps = self.maps.clone();
        }
        let pointer_cells = self.import_pointers(&mut args)?;
        child
            .heap
            .extend(pointer_cells.iter().map(|(_, v)| v.clone()));
        child.locals.extend(args);
        if child.statement(body.root().ok()?, depth + 1).is_some()
            && self.facts.type_of(&func.signature.result)?.kind == TypeKind::Unit
        {
            child.returned = Some(Scalar::Nil);
        }
        self.remaining = child.remaining;
        let result = self.frame_result(&mut child, (reads_buffers, reads_maps, allow_writes))?;
        self.export_pointers(&pointer_cells, &child.heap, allow_writes)?;
        Some(result)
    }

    fn commit_aggregates(
        &mut self,
        child: &Walk<'_, '_, '_>,
        (buffers, maps): (bool, bool),
        write: bool,
    ) -> Option<()> {
        if buffers {
            for (old, new) in self.buffers.iter_mut().zip(&child.buffers) {
                if !std::sync::Arc::ptr_eq(old, new) {
                    if !write || old.len() != new.len() || !new.iter().all(primitive) {
                        return None;
                    }
                    *old = new.clone();
                }
            }
        }
        if maps {
            for (old, new) in self.maps.iter_mut().zip(&child.maps) {
                if old != new {
                    if !write || !super::maps::transferable_items(new) {
                        return None;
                    }
                    *old = new.clone();
                }
            }
        }
        Some(())
    }

    pub(super) fn frame_result(
        &mut self,
        child: &mut Walk<'_, '_, '_>,
        (reads_buffers, reads_maps, write): (bool, bool, bool),
    ) -> Option<Scalar> {
        let out = child.returned.take()?;
        if reads_buffers || reads_maps {
            self.commit_aggregates(child, (reads_buffers, reads_maps), write)?;
        }
        if primitive(&out) || matches!(&out,Scalar::Tuple(items) if items.iter().all(primitive)) {
            return Some(out);
        }
        match out {
            Scalar::Slice(view) if child.transferable(&Scalar::Slice(view)) => match view.slot() {
                Some(slot) if !reads_buffers || slot >= self.buffers.len() => {
                    let cell = self.allocate(child.buffers.get(slot)?.clone())?;
                    Some(Scalar::Slice(view.relocated(cell)))
                }
                _ => Some(Scalar::Slice(view)),
            },
            Scalar::Nil
                if self
                    .facts
                    .type_of(&child.facts.body.function.signature.result)?
                    .kind
                    == TypeKind::Unit =>
            {
                Some(Scalar::Nil)
            }
            Scalar::ClosureValue(c) if !child.inside_literal => self.save_environment(child, c),
            _ => None,
        }
    }
}
