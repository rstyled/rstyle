//! Resolved Go memory builtins within an existing, bounded allocation. A raw
//! conversion retains its element type. Reinterpretation and an unknown
//! allocation boundary do not yield a scalar fact.
use super::pointers::Address;
use super::storage::View;
use super::{Node, Scalar, TypeKind, Walk};
use crate::cpg::model::Type;

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn raw_type(&self, ty: &Type) -> bool {
        ty.kind == TypeKind::Pointer
            && ty.mode.as_deref() == Some("raw")
            && ty
                .target
                .as_deref()
                .and_then(|id| self.facts.type_of(id))
                .and_then(|t| t.key.as_ref())
                == Some(&self.profile.arbitrary_pointee)
            && ty
                .definition
                .as_deref()
                .and_then(|id| self.facts.tables.definition(id).ok())
                .is_some_and(|d| d.standard && d.kind == super::DefinitionKind::Type)
    }

    pub(super) fn pointer_cast(&self, to: &Type, from: &Type, value: Scalar) -> Option<Scalar> {
        match value {
            Scalar::RawPointer(Some(addr), Some(origin)) if self.uintptr(to) => {
                self.address_word(&addr, &origin)
            }
            Scalar::Pointer(addr) if from.kind == TypeKind::Reference && self.raw_type(to) => {
                Some(Scalar::RawPointer(addr, Some(from.target.clone()?)))
            }
            Scalar::RawPointer(addr, orig)
                if to.kind == TypeKind::Reference
                    && (addr.is_none() || (orig.is_some() && orig == to.target)) =>
            {
                Some(Scalar::Pointer(addr))
            }
            Scalar::RawPointer(Some(addr), Some(origin)) if to.kind == TypeKind::Reference => {
                let array = self.facts.type_of(to.target.as_deref()?)?;
                if array.kind != TypeKind::Sequence
                    || array.fixed != Some(true)
                    || array.element.as_ref() != Some(&origin)
                {
                    return None;
                }
                let n = usize::try_from(array.length?).ok()?;
                self.memory_view(Some(addr), n)?.array_address(n)
            }
            v @ Scalar::RawPointer(_, _) if self.raw_type(to) && self.raw_type(from) => Some(v),
            _ => None,
        }
    }

    pub(super) fn read_string(&self, value: Scalar) -> Option<Scalar> {
        match value {
            v @ Scalar::Text(_) => Some(v),
            Scalar::StringView(view) => {
                let mut bytes = Vec::with_capacity(view.len);
                for i in 0..view.len {
                    let Scalar::Int(n) = self.element(view, i)? else {
                        return None;
                    };
                    bytes.push(u8::try_from(*n).ok()?);
                }
                Some(Scalar::Text(bytes))
            }
            _ => None,
        }
    }

    fn element_width(&self, id: &str) -> Option<usize> {
        let ty = self.facts.type_of(id)?;
        let bits = match ty.kind {
            TypeKind::Integer => u32::from(self.facts.integer(Some(ty))?.bits),
            TypeKind::Float => ty.bits?,
            _ => return None,
        };
        matches!(bits, 8 | 16 | 32 | 64).then_some((bits / 8) as usize)
    }

    fn byte_pointer(&self, ty: &Type) -> bool {
        ty.kind == TypeKind::Reference
            && ty
                .target
                .as_deref()
                .and_then(|id| self.facts.type_of(id))
                .and_then(|elem| self.facts.integer(Some(elem)))
                .is_some_and(|integer| integer.bits == 8 && !integer.signed)
    }

    fn memory_view(&self, addr: Option<Address>, len: usize) -> Option<View> {
        match addr {
            None if len == 0 => Some(View::nil()),
            Some(Address::Cell(slot, first))
                if first.checked_add(len)? <= self.buffers.get(slot)?.len() =>
            {
                Some(View::window(slot, first, len))
            }
            _ => None,
        }
    }

    fn raw_add(&self, value: Scalar, delta: i128) -> Option<Scalar> {
        let Scalar::RawPointer(addr, origin) = value else {
            return None;
        };
        if delta == 0 {
            return Some(Scalar::RawPointer(addr, origin));
        }
        let Address::Cell(slot, index) = addr? else {
            return None;
        };
        let width = i128::try_from(self.element_width(origin.as_deref()?)?).ok()?;
        if delta % width != 0 {
            return None;
        }
        let next = usize::try_from((index as i128).checked_add(delta / width)?).ok()?;
        self.buffers.get(slot)?.get(next)?;
        Some(Scalar::RawPointer(Some(Address::Cell(slot, next)), origin))
    }

    pub(super) fn memory_call(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        let source = self.facts.ty(arg)?;
        let target = self.facts.ty(node)?;
        if op == self.profile.unsafe_add
            && node.children.len() == 3
            && self.raw_type(source)
            && self.raw_type(target)
        {
            let ptr = self.value(arg, depth)?;
            let Scalar::Int(offset) = self.value(self.child(node, 2)?, depth)? else {
                return None;
            };
            return self.raw_add(ptr, offset);
        }
        if (op == self.profile.unsafe_slice || op == self.profile.unsafe_string)
            && node.children.len() == 3
        {
            let is_slice = op == self.profile.unsafe_slice
                && target.fixed == Some(false)
                && target.kind == TypeKind::Sequence
                && source.kind == TypeKind::Reference
                && source.target.is_some()
                && source.target == target.element;
            let is_string = op == self.profile.unsafe_string
                && target.kind == TypeKind::String
                && self.byte_pointer(source);
            if !(is_slice || is_string) {
                return None;
            }
            let Scalar::Pointer(addr) = self.value(arg, depth)? else {
                return None;
            };
            let len = self.index(self.child(node, 2)?, depth)?;
            if is_string && len == 0 {
                return Some(Scalar::Text(Vec::new()));
            }
            let view = self.memory_view(addr, len)?;
            return Some(if is_slice {
                Scalar::Slice(view)
            } else {
                Scalar::StringView(view)
            });
        }
        self.data_pointer(node, op, depth)
    }

    fn data_pointer(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        let source = self.facts.ty(arg)?;
        let target = self.facts.ty(node)?;
        if op == self.profile.unsafe_slice_data
            && node.children.len() == 2
            && source.kind == TypeKind::Sequence
            && source.fixed == Some(false)
            && target.kind == TypeKind::Reference
            && target.target.is_some()
            && target.target == source.element
        {
            let Scalar::Slice(view) = self.value(arg, depth)? else {
                return None;
            };
            return Some(Scalar::Pointer(
                view.data()?.map(|(slot, at)| Address::Cell(slot, at)),
            ));
        }
        if op == self.profile.unsafe_string_data
            && node.children.len() == 2
            && source.kind == TypeKind::String
            && self.byte_pointer(target)
        {
            let Scalar::StringView(view) = self.value(arg, depth)? else {
                return None;
            };
            let (slot, at) = view.position(0)?;
            return Some(Scalar::Pointer(Some(Address::Cell(slot, at))));
        }
        None
    }
}
