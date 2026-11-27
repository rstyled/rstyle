//! Known, bounded aggregate storage. A slice is a view into one backing buffer; copying
//! a fixed array creates independent storage. An append with insufficient capacity has
//! an implementation-chosen new capacity, which remains unknown.
use super::{Node, NodeKind, Scalar, Walk};
use crate::cpg::model::{Type, TypeKind};
use std::sync::Arc;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(in crate::cpg::bounds) struct View {
    slot: Option<usize>,
    first: usize,
    pub(super) len: usize,
    capacity: Option<usize>,
}

impl View {
    pub(super) fn position(self, index: usize) -> Option<(usize, usize)> {
        (index < self.len).then_some((self.slot?, self.first + index))
    }
    pub(super) fn nil() -> Self {
        Self {
            slot: None,
            first: 0,
            len: 0,
            capacity: Some(0),
        }
    }
}

pub(super) const MAX_ITEMS: usize = 256;

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn zero(&self, ty: &Type) -> Option<Scalar> {
        let mut remaining = MAX_ITEMS;
        self.zero_at(ty, 0, &mut remaining)
    }

    fn zero_at(&self, ty: &Type, depth: usize, remaining: &mut usize) -> Option<Scalar> {
        *remaining = remaining.checked_sub(1)?;
        if depth > 32 {
            return None;
        }
        Some(match ty.kind {
            TypeKind::Integer => Scalar::Int(0),
            TypeKind::Bool => Scalar::Bool(false),
            TypeKind::Float if matches!(ty.bits, Some(32 | 64)) => Scalar::Real(0.),
            TypeKind::Opaque if self.complex_width(ty).is_some() => Scalar::Complex(0., 0.),
            TypeKind::String => Scalar::Text(Vec::new()),
            TypeKind::Map => Scalar::Map(None),
            TypeKind::Reference => Scalar::Pointer(None),
            TypeKind::Opaque if self.channel_element(ty).is_some() => Scalar::Channel(None),
            TypeKind::Sequence if ty.fixed == Some(false) => Scalar::Slice(View::nil()),
            TypeKind::Sequence if ty.fixed == Some(true) => {
                let len = usize::try_from(ty.length?).ok()?;
                *remaining = remaining.checked_sub(len)?;
                let element = self.facts.type_of(ty.element.as_deref()?)?;
                let item = self.zero_at(element, depth + 1, remaining)?;
                Scalar::Sequence(vec![item; len].into())
            }
            TypeKind::Record => {
                let mut values = std::collections::BTreeMap::new();
                for f in ty.fields.as_ref()? {
                    let z = self.zero_at(self.facts.type_of(&f.ty)?, depth + 1, remaining)?;
                    if values.insert(f.name.clone(), z).is_some() {
                        return None;
                    }
                }
                Scalar::Record(Arc::new(values))
            }
            _ => return None,
        })
    }

    fn allocate(&mut self, items: Arc<[Scalar]>) -> Option<usize> {
        if items.len() > MAX_ITEMS || self.buffers.len() > MAX_ITEMS {
            return None;
        }
        self.remaining = self.remaining.checked_sub(items.len())?;
        let slot = self.buffers.len();
        self.buffers.push(items);
        Some(slot)
    }

    pub(super) fn fresh_slice(
        &mut self,
        items: Vec<Scalar>,
        capacity: Option<usize>,
    ) -> Option<Scalar> {
        let len = items.len();
        let slot = self.allocate(items.into())?;
        Some(Scalar::Slice(View {
            slot: Some(slot),
            first: 0,
            len,
            capacity,
        }))
    }

    pub(super) fn store(&mut self, symbol: &'a str, value: Scalar) -> Option<()> {
        if let Scalar::Sequence(items) = &value {
            if let Some(slot) = self.array_slots.get(symbol) {
                if self.buffers[*slot].len() != items.len() {
                    return None;
                }
                self.buffers[*slot] = items.clone();
            } else {
                let slot = self.allocate(items.clone())?;
                self.array_slots.insert(symbol, slot);
            }
        }
        self.locals.insert(symbol, value);
        Some(())
    }

    pub(super) fn index(&mut self, node: &'a Node, depth: usize) -> Option<usize> {
        let Scalar::Int(i) = self.value(node, depth)? else {
            return None;
        };
        usize::try_from(i).ok()
    }

    pub(super) fn array(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let ty = self.facts.ty(node)?;
        let fixed = ty.length.and_then(|n| usize::try_from(n).ok());
        if fixed.is_some_and(|n| n > MAX_ITEMS) || node.children.len() > MAX_ITEMS {
            return None;
        }
        let mut entries = std::collections::BTreeMap::new();
        let mut next = 0;
        for id in &node.children {
            let child = self.facts.body.node(id).ok()?;
            let value = if child.kind == NodeKind::Tuple && child.children.len() == 2 {
                next = self.index(self.child(child, 0)?, depth)?;
                self.child(child, 1)?
            } else {
                child
            };
            if next >= fixed.unwrap_or(MAX_ITEMS)
                || entries.insert(next, self.value(value, depth)?).is_some()
            {
                return None;
            }
            next += 1;
        }
        let size = fixed.unwrap_or_else(|| entries.last_key_value().map_or(0, |(i, _)| i + 1));
        let mut a = Vec::with_capacity(size);
        for i in 0..size {
            let value = match entries.remove(&i) {
                Some(value) => value,
                None => self.zero(self.facts.type_of(ty.element.as_deref()?)?)?,
            };
            a.push(value);
        }
        if ty.fixed == Some(true) {
            return Some(Scalar::Sequence(a.into()));
        }
        if ty.fixed != Some(false) {
            return None;
        }
        let slot = self.allocate(a.into())?;
        Some(Scalar::Slice(View {
            slot: Some(slot),
            first: 0,
            len: size,
            capacity: Some(size),
        }))
    }

    pub(super) fn view(&mut self, node: &'a Node, depth: usize) -> Option<View> {
        match self.value(node, depth)? {
            Scalar::Slice(view) => Some(view),
            Scalar::Sequence(_) => {
                let slot = *self.array_slots.get(self.facts.local(node)?)?;
                // Without a declared length, trailing zero-value array elements might be missing.
                let size = usize::try_from(self.facts.ty(node)?.length?).ok()?;
                (self.buffers[slot].len() == size).then_some(View {
                    slot: Some(slot),
                    first: 0,
                    len: size,
                    capacity: Some(size),
                })
            }
            _ => None,
        }
    }

    pub(super) fn element(&self, view: View, index: usize) -> Option<&Scalar> {
        (index < view.len).then_some(())?;
        self.buffers.get(view.slot?)?.get(view.first + index)
    }

    pub(super) fn set(&mut self, view: View, index: usize, value: Scalar) -> Option<()> {
        if index >= view.len {
            return None;
        }
        *Arc::make_mut(self.buffers.get_mut(view.slot?)?).get_mut(view.first + index)? = value;
        Some(())
    }

    pub(super) fn slice(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let roles = self
            .facts
            .tables
            .extension(&self.profile.slice_bounds)?
            .get(&node.id)?
            .as_array()?;
        if roles.len() + 1 != node.children.len() {
            return None;
        }
        let view = self.view(self.child(node, 0)?, depth)?;
        let (mut low, mut high, mut max) = (0, view.len, view.capacity);
        let mut previous = 0;
        for (i, role) in roles.iter().enumerate() {
            let n = self.index(self.child(node, i + 1)?, depth)?;
            let rank = match role.as_str()? {
                "low" => {
                    low = n;
                    1
                }
                "high" => {
                    high = n;
                    2
                }
                "max" => {
                    max = Some(n);
                    3
                }
                _ => return None,
            };
            if rank <= previous {
                return None;
            }
            previous = rank;
        }
        if low > high
            || max.is_some_and(|m| high > m)
            || view.capacity.is_some_and(|c| max.is_some_and(|m| m > c))
        {
            return None;
        }
        // A grown slice can expose a known initialized prefix even when its allocation policy is opaque.
        if view.capacity.is_none() && (high > view.len || max.is_some_and(|m| m > view.len)) {
            return None;
        }
        Some(Scalar::Slice(View {
            first: view.first + low,
            len: high - low,
            capacity: max.map(|m| m - low),
            ..view
        }))
    }

    pub(super) fn builtin(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        if op == self.profile.new_variable && node.children.len() == 2 {
            let ptr = self.facts.ty(node)?;
            if ptr.kind != TypeKind::Reference || ptr.target != arg.ty || ptr.target.is_none() {
                return None;
            }
            let value = if arg.kind == NodeKind::TypeAnnotation {
                self.zero(self.facts.ty(arg)?)?
            } else {
                self.value(arg, depth)?
            };
            return self.fresh_pointer(value);
        }
        if self.channel_element(self.facts.ty(arg)?).is_some() {
            return self.channel_builtin(node, op, depth);
        }
        if self.facts.ty(arg)?.kind == TypeKind::Map {
            return self.map_builtin(node, op, depth);
        }
        if op == self.profile.make && self.facts.ty(node)?.kind == TypeKind::Sequence {
            return self.make_slice(node, depth);
        }
        if (op == self.profile.byte_length || op == self.profile.capacity)
            && node.children.len() == 2
        {
            return self.size(arg, op, depth);
        }
        if op == self.profile.minimum || op == self.profile.maximum {
            return self.extreme(node, op, depth);
        }
        if self.write_at != Some(node.id.as_str()) {
            return None;
        }
        let dest = self.view(arg, depth)?;
        if op == self.profile.append {
            let extra = self.append_arguments(node, depth)?;
            return self.append_slice(dest, extra);
        }
        if op == self.profile.clear && node.children.len() == 2 {
            let z = self.zero(
                self.facts
                    .type_of(self.facts.ty(arg)?.element.as_deref()?)?,
            )?;
            for i in 0..dest.len {
                self.set(dest, i, z.clone())?;
            }
            return Some(Scalar::Nil);
        }
        if op == self.profile.copy && node.children.len() == 3 {
            return self.copy_slice(node, dest, depth);
        }
        None
    }

    fn size(&mut self, arg: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let size = match self.value(arg, depth)? {
            Scalar::Text(bytes) if op == self.profile.byte_length => bytes.len(),
            Scalar::Slice(v) => {
                if op == self.profile.capacity {
                    v.capacity?
                } else {
                    v.len
                }
            }
            Scalar::Sequence(_) => usize::try_from(self.facts.ty(arg)?.length?).ok()?,
            Scalar::Pointer(_) => {
                let array = self.facts.type_of(self.facts.ty(arg)?.target.as_deref()?)?;
                if array.kind != TypeKind::Sequence || array.fixed != Some(true) {
                    return None;
                }
                usize::try_from(array.length?).ok()?
            }
            _ => return None,
        };
        Some(Scalar::Int(size as i128))
    }

    fn make_slice(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        let t = self.facts.ty(node)?;
        if t.fixed != Some(false)
            || arg.kind != NodeKind::TypeAnnotation
            || !(3..=4).contains(&node.children.len())
        {
            return None;
        }
        let len = self.index(self.child(node, 2)?, depth)?;
        let cap = if node.children.len() == 4 {
            self.index(self.child(node, 3)?, depth)?
        } else {
            len
        };
        if cap > MAX_ITEMS || len > cap {
            return None;
        }
        let z = self.zero(self.facts.type_of(t.element.as_deref()?)?)?;
        let slot = self.allocate(vec![z; cap].into())?;
        Some(Scalar::Slice(View {
            slot: Some(slot),
            first: 0,
            len,
            capacity: Some(cap),
        }))
    }

    fn extreme(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let mut best = self.value(self.child(node, 1)?, depth)?;
        for id in &node.children[2..] {
            let next = self.value(self.facts.body.node(id).ok()?, depth)?;
            let order = match (&best, &next) {
                (Scalar::Int(a), Scalar::Int(b)) => a.cmp(b),
                (Scalar::Text(a), Scalar::Text(b)) => a.cmp(b),
                _ => return None,
            };
            if (op == self.profile.minimum && order.is_gt())
                || (op == self.profile.maximum && order.is_lt())
            {
                best = next
            }
        }
        Some(best)
    }

    fn copy_slice(&mut self, node: &'a Node, dest: View, depth: usize) -> Option<Scalar> {
        let source = self.value(self.child(node, 2)?, depth)?;
        // Materialize the source before writing any destination, also for overlapping views.
        let items = match source {
            Scalar::Slice(v) => (0..v.len.min(dest.len))
                .map(|i| self.element(v, i).cloned())
                .collect::<Option<Vec<_>>>()?,
            Scalar::Text(s) => s
                .into_iter()
                .take(dest.len)
                .map(|b| Scalar::Int(i128::from(b)))
                .collect(),
            _ => return None,
        };
        let count = items.len();
        for (i, x) in items.into_iter().enumerate() {
            self.set(dest, i, x)?;
        }
        Some(Scalar::Int(count as i128))
    }

    fn append_slice(&mut self, dest: View, extra: Vec<Scalar>) -> Option<Scalar> {
        if extra.is_empty() {
            return Some(Scalar::Slice(dest));
        }
        let n = dest.len.checked_add(extra.len())?;
        if n > MAX_ITEMS {
            return None;
        }
        if n <= dest.capacity? {
            let v = View { len: n, ..dest };
            for (i, x) in extra.into_iter().enumerate() {
                self.set(v, dest.len + i, x)?;
            }
            return Some(Scalar::Slice(v));
        }
        let mut items = (0..dest.len)
            .map(|i| self.element(dest, i).cloned())
            .collect::<Option<Vec<_>>>()?;
        items.extend(extra);
        let slot = self.allocate(items.into())?;
        Some(Scalar::Slice(View {
            slot: Some(slot),
            first: 0,
            len: n,
            capacity: None,
        }))
    }

    fn append_arguments(&mut self, node: &'a Node, depth: usize) -> Option<Vec<Scalar>> {
        let elem = self.facts.ty(self.child(node, 1)?)?.element.as_deref()?;
        let spread = self
            .facts
            .tables
            .extension(&self.profile.spread_arguments)
            .and_then(|m| m.get(&node.id))
            .and_then(|v| v.as_array());
        let mut extra = Vec::new();
        if let Some([last]) = spread.map(|args| args.as_slice()) {
            if node.children.len() != 3 || Some(self.child(node, 2)?.id.as_str()) != last.as_str() {
                return None;
            }
            let last = self.child(node, 2)?;
            extra = match self.value(last, depth)? {
                Scalar::Slice(view) if self.facts.ty(last)?.element.as_deref() == Some(elem) => (0
                    ..view.len)
                    .map(|i| self.element(view, i).cloned())
                    .collect::<Option<Vec<_>>>()?,
                Scalar::Text(bytes)
                    if self.facts.integer(self.facts.type_of(elem))
                        == Some(crate::runtime_bounds::Integer::unsigned(8))
                        && bytes.len() <= MAX_ITEMS =>
                {
                    bytes
                        .into_iter()
                        .map(|b| Scalar::Int(i128::from(b)))
                        .collect()
                }
                _ => return None,
            };
        } else {
            if spread.is_some_and(|list| !list.is_empty()) {
                return None;
            }
            for id in &node.children[2..] {
                let child = self.facts.body.node(id).ok()?;
                // Older producers did not distinguish a recursive slice passed as one element
                // from one expanded into individual elements.
                if child.ty.as_deref() != Some(elem)
                    || (spread.is_none() && self.facts.ty(child)?.element.as_deref() == Some(elem))
                {
                    return None;
                }
                extra.push(self.value(child, depth)?);
            }
        }
        Some(extra)
    }
}
