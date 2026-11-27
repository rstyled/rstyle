//! Addressable storage within a single invocation. Returning or passing a pointer to
//! a separate frame requires an alias summary and stays unknown.
use super::{Node, NodeKind, Scalar, TypeKind, Walk};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::cpg::bounds) enum Address {
    Local(String),
    Heap(usize),
    Array(super::storage::View),
    Cell(usize, usize),
    Field(Box<Address>, String),
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn fresh_pointer(&mut self, value: Scalar) -> Option<Scalar> {
        if let Scalar::Sequence(items) = value {
            let view = self.array_view(items)?;
            return Some(Scalar::Pointer(Some(Address::Array(view))));
        }
        if self.heap.len() >= super::storage::MAX_ITEMS {
            return None;
        }
        let slot = self.heap.len();
        self.heap.push(value);
        Some(Scalar::Pointer(Some(Address::Heap(slot))))
    }

    pub(super) fn address(&mut self, place: &'a Node, depth: usize) -> Option<Address> {
        self.enter(place, depth)?;
        match (place.kind, place.children.len()) {
            (NodeKind::Identifier, 0) => {
                let symbol = self.facts.local(place)?;
                self.locals
                    .contains_key(symbol)
                    .then(|| Address::Local(symbol.to_owned()))
            }
            (NodeKind::IndexAccess, 2) => {
                let base = self.view(self.child(place, 0)?, depth + 1)?;
                let i = self.index(self.child(place, 1)?, depth + 1)?;
                let (slot, at) = base.position(i)?;
                Some(Address::Cell(slot, at))
            }
            (NodeKind::FieldAccess, 1) if self.facts.reference(place).is_none() => {
                let n = self.child(place, 0)?;
                let base = if self.facts.ty(n)?.kind == TypeKind::Reference {
                    let Scalar::Pointer(Some(addr)) = self.value(n, depth + 1)? else {
                        return None;
                    };
                    addr
                } else {
                    self.address(n, depth + 1)?
                };
                let field = place.member.as_ref()?;
                let Scalar::Record(fields) = self.pointed(&base)? else {
                    return None;
                };
                fields
                    .contains_key(field)
                    .then(|| Address::Field(Box::new(base), field.clone()))
            }
            (NodeKind::Operator, 1)
                if place.operator.as_deref() == Some("deref")
                    && self.facts.reference(place).is_none() =>
            {
                let Scalar::Pointer(addr) = self.value(self.child(place, 0)?, depth + 1)? else {
                    return None;
                };
                addr
            }
            (NodeKind::RecordConstruction | NodeKind::Array, _) => {
                let value = self.value(place, depth + 1)?;
                let Scalar::Pointer(addr) = self.fresh_pointer(value)? else {
                    return None;
                };
                addr
            }
            _ => None,
        }
    }

    pub(super) fn pointed(&self, addr: &Address) -> Option<Scalar> {
        match addr {
            Address::Array(view) => {
                let items = (0..view.len)
                    .map(|i| self.element(*view, i).cloned())
                    .collect::<Option<Vec<_>>>()?;
                Some(Scalar::Sequence(items.into()))
            }
            Address::Local(id) => self.local_value(id),
            Address::Heap(i) => self.heap.get(*i).cloned(),
            Address::Cell(slot, i) => self.buffers.get(*slot)?.get(*i).cloned(),
            Address::Field(base, name) => {
                let Scalar::Record(fields) = self.pointed(base)? else {
                    return None;
                };
                fields.get(name).cloned()
            }
        }
    }

    pub(super) fn write_address(&mut self, addr: &Address, value: Scalar) -> Option<()> {
        match addr {
            Address::Array(view) => {
                let Scalar::Sequence(items) = value else {
                    return None;
                };
                if view.len != items.len() {
                    return None;
                }
                for (i, x) in items.iter().enumerate() {
                    self.set(*view, i, x.clone())?;
                }
                Some(())
            }
            Address::Local(id) => {
                let (&symbol, _) = self.locals.get_key_value(id.as_str())?;
                self.store(symbol, value)
            }
            Address::Heap(i) => {
                *self.heap.get_mut(*i)? = value;
                Some(())
            }
            Address::Cell(slot, i) => {
                *std::sync::Arc::make_mut(self.buffers.get_mut(*slot)?).get_mut(*i)? = value;
                Some(())
            }
            Address::Field(base, name) => {
                let Scalar::Record(mut fields) = self.pointed(base)? else {
                    return None;
                };
                *std::sync::Arc::make_mut(&mut fields).get_mut(name)? = value;
                self.write_address(base, Scalar::Record(fields))
            }
        }
    }

    pub(super) fn field_value(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        let receiver = self.facts.ty(self.child(node, 0)?)?;
        let record = if receiver.kind == TypeKind::Reference {
            self.facts.type_of(receiver.target.as_deref()?)?
        } else {
            receiver
        };
        let name = node.member.as_deref()?;
        let field = record.fields.as_ref()?.iter().find(|f| f.name == name)?;
        if Some(field.ty.as_str()) != node.ty.as_deref() {
            return None;
        }
        let base = self.value(self.child(node, 0)?, depth)?;
        let v = match base {
            Scalar::Pointer(Some(addr)) => self.pointed(&addr)?,
            v => v,
        };
        let Scalar::Record(fields) = v else {
            return None;
        };
        fields.get(node.member.as_deref()?).cloned()
    }
}
