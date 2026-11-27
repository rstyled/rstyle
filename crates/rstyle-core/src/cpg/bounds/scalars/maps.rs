//! Map headers refer to shared storage, including after a local assignment. Only keys
//! with a completely known, non-panicking equality operation participate in the model.
use super::{Node, NodeKind, Scalar, TypeKind, Walk};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Int(i128),
    Bool(bool),
    Bytes(Vec<u8>),
    Array(Vec<Key>),
    Dynamic(String, Box<Key>),
    NilInterface,
    Record(Vec<(String, Key)>),
}

type Items = BTreeMap<Key, Scalar>;
pub(super) type Maps = Vec<Items>;

impl Key {
    pub(super) fn of(v: Scalar) -> Option<Self> {
        let mut remaining = super::storage::MAX_ITEMS;
        Self::part(&v, &mut remaining)
    }

    fn part(v: &Scalar, remaining: &mut usize) -> Option<Self> {
        *remaining = remaining.checked_sub(1)?;
        match v {
            Scalar::Int(n) => Some(Self::Int(*n)),
            Scalar::Bool(b) => Some(Self::Bool(*b)),
            Scalar::Text(s) => Some(Self::Bytes(s.clone())),
            Scalar::Interface(None) => Some(Self::NilInterface),
            // Any dynamic key's type must be canonicalized by map_key first.
            Scalar::Sequence(a) => Some(Self::Array(
                a.iter()
                    .map(|x| Self::part(x, remaining))
                    .collect::<Option<_>>()?,
            )),
            Scalar::Record(fields) => Some(Self::Record(
                fields
                    .iter()
                    .filter(|(name, _)| name.as_str() != "_")
                    .map(|(n, v)| Some((n.clone(), Self::part(v, remaining)?)))
                    .collect::<Option<_>>()?,
            )),
            _ => None,
        }
    }
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn map_key(&mut self, map: &Node, node: &'a Node, depth: usize) -> Option<Key> {
        let declared = self
            .facts
            .type_of(self.facts.ty(map)?.key_type.as_deref()?)?;
        let v = self.value(node, depth)?;
        if declared.kind != TypeKind::Interface {
            return Key::of(v);
        }
        let Scalar::Interface(pair) = self.interface_value(declared, node, v)? else {
            return None;
        };
        let Some(pair) = pair else {
            return Some(Key::NilInterface);
        };
        let ty = self.facts.type_of(&pair.0)?;
        // Structural identities (including type aliases in a composite) require a
        // canonical identity before they can be used as a dynamic map key.
        if !matches!(
            ty.kind,
            TypeKind::Integer | TypeKind::String | TypeKind::Bool
        ) {
            return None;
        }
        let identity = if ty.definition.is_some() {
            ty.id.as_str()
        } else {
            super::interfaces::primitive(ty)?
        };
        Some(Key::Dynamic(
            identity.to_owned(),
            Box::new(Key::of(pair.1)?),
        ))
    }

    fn map(&mut self, items: Items) -> Option<Scalar> {
        if self.maps.len() >= super::storage::MAX_ITEMS {
            return None;
        }
        self.remaining = self.remaining.checked_sub(1 + items.len())?;
        let slot = self.maps.len();
        self.maps.push(items);
        Some(Scalar::Map(Some(slot)))
    }

    pub(super) fn map_literal(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        if node.children.len() > super::storage::MAX_ITEMS {
            return None;
        }
        let mut items = Items::new();
        for id in &node.children {
            let pair = self.facts.body.node(id).ok()?;
            if pair.kind != NodeKind::Tuple || pair.children.len() != 2 {
                return None;
            }
            let key = self.map_key(node, self.child(pair, 0)?, depth)?;
            let value = self.item_value(
                self.facts
                    .type_of(self.facts.ty(node)?.value_type.as_deref()?)?,
                self.child(pair, 1)?,
                depth,
            )?;
            // The specification leaves the order of conflicting map literal stores open.
            if items.insert(key, value).is_some() {
                return None;
            }
        }
        self.map(items)
    }

    pub(super) fn lookup(
        &self,
        base: &'a Node,
        slot: Option<usize>,
        key: Key,
    ) -> Option<(Scalar, bool)> {
        if let Some(slot) = slot
            && let Some(val) = self.maps.get(slot)?.get(&key)
        {
            return Some((val.clone(), true));
        }
        let elem = self.facts.ty(base)?.value_type.as_deref()?;
        Some((self.zero(self.facts.type_of(elem)?)?, false))
    }

    pub(super) fn map_builtin(&mut self, node: &'a Node, op: &str, depth: usize) -> Option<Scalar> {
        let arg = self.child(node, 1)?;
        if op == self.profile.make
            && arg.kind == NodeKind::TypeAnnotation
            && (2..=3).contains(&node.children.len())
        {
            if node.children.len() == 3 {
                self.index(self.child(node, 2)?, depth)?;
            }
            return self.map(Items::new());
        }
        let Scalar::Map(slot) = self.value(arg, depth)? else {
            return None;
        };
        if op == self.profile.byte_length && node.children.len() == 2 {
            let len = match slot {
                Some(slot) => self.maps.get(slot)?.len(),
                None => 0,
            };
            return Some(Scalar::Int(len as i128));
        }
        if self.write_at != Some(node.id.as_str()) {
            return None;
        }
        if op == self.profile.clear && node.children.len() == 2 {
            if let Some(i) = slot {
                self.maps.get_mut(i)?.clear();
            }
            return Some(Scalar::Nil);
        }
        if op == self.profile.delete && node.children.len() == 3 {
            let key = self.map_key(arg, self.child(node, 2)?, depth)?;
            if let Some(i) = slot {
                self.maps.get_mut(i)?.remove(&key);
            }
            return Some(Scalar::Nil);
        }
        None
    }

    pub(super) fn map_bindings(
        &mut self,
        pattern: &'a Node,
        rhs: &'a Node,
        depth: usize,
    ) -> Option<()> {
        if rhs.children.len() != 2
            || pattern.children.len() != 2
            || self.facts.ty(rhs)?.kind != TypeKind::Tuple
        {
            return None;
        }
        let base = self.child(rhs, 0)?;
        let Scalar::Map(slot) = self.value(base, depth)? else {
            return None;
        };
        let key = self.map_key(base, self.child(rhs, 1)?, depth)?;
        let (item, hit) = self.lookup(base, slot, key)?;
        self.pair_bindings(pattern, item, hit)
    }

    pub(super) fn pair_bindings(
        &mut self,
        pattern: &'a Node,
        item: Scalar,
        hit: bool,
    ) -> Option<()> {
        if pattern.children.len() != 2 {
            return None;
        }
        for (i, v) in [item, Scalar::Bool(hit)].into_iter().enumerate() {
            let p = self.child(pattern, i)?;
            if p.kind != NodeKind::PatternWildcard {
                self.store(self.binding(p)?, self.for_storage(p, v)?)?;
            }
        }
        Some(())
    }
}
