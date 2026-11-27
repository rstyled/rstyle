//! Conversions between integer code points, byte strings and independent byte or rune slices.
use super::{Node, Scalar, TypeKind, Walk};
use crate::runtime_bounds::Integer;

fn rune(n: i128) -> char {
    u32::try_from(n)
        .ok()
        .and_then(char::from_u32)
        .unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// Go's decoder advances one byte on any malformed encoding, including a truncated prefix.
fn runes(mut bytes: &[u8]) -> Option<Vec<Scalar>> {
    let mut result = Vec::new();
    while let Some(&first) = bytes.first() {
        let width = match first {
            0..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => 0,
        };
        let next = bytes
            .get(..width)
            .and_then(|s| std::str::from_utf8(s).ok())
            .and_then(|s| s.chars().next());
        let (ch, consumed) = next.map_or((char::REPLACEMENT_CHARACTER, 1), |ch| (ch, width));
        result.push(Scalar::Int(i128::from(u32::from(ch))));
        if result.len() > super::storage::MAX_ITEMS {
            return None;
        }
        bytes = &bytes[consumed..];
    }
    Some(result)
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn convert(&mut self, node: &'a Node, depth: usize) -> Option<Scalar> {
        if node.original_kind == self.profile.type_assertion {
            let (v, ok) = self.asserted(node, depth, false)?;
            return ok.then_some(v);
        }
        let arg = self.child(node, 0)?;
        let to = self.facts.ty(node)?;
        let from = self.facts.ty(arg)?;
        let value = self.rounded_argument(arg, to, depth)?;
        if to.kind == TypeKind::Interface {
            return self.interface_value(to, arg, value);
        }
        if let Scalar::Slice(view) = &value
            && to.kind == TypeKind::Reference
        {
            return self.slice_pointer(*view, (from, to));
        }
        if value == Scalar::Nil {
            return self.for_storage(node, value);
        }
        if self.raw_type(to) || self.raw_type(from) {
            return self.pointer_cast(to, from, value);
        }
        if let Some(v) = self.convert_numeric(&value, to) {
            return Some(v);
        }
        if from.kind == to.kind
            && matches!(
                to.kind,
                TypeKind::Reference | TypeKind::Record | TypeKind::Sequence
            )
        {
            return self.preserve_aggregate(value, (from, to));
        }
        match (value, to.kind) {
            (v @ Scalar::Int(_), TypeKind::Integer) if self.facts.integer(Some(to)).is_some() => {
                Some(v)
            }
            (v @ Scalar::Bool(_), TypeKind::Bool) | (v @ Scalar::Text(_), TypeKind::String) => {
                Some(v)
            }
            (Scalar::Int(n), TypeKind::String) => {
                Some(Scalar::Text(rune(n).to_string().into_bytes()))
            }
            (Scalar::Text(bytes), TypeKind::Sequence) if to.fixed == Some(false) => {
                self.text_slice(bytes, to)
            }
            (Scalar::Slice(view), TypeKind::String) => self.slice_text(view, from),
            _ => None,
        }
    }
    fn preserve_aggregate(
        &self,
        value: Scalar,
        (from, to): (&crate::cpg::model::Type, &crate::cpg::model::Type),
    ) -> Option<Scalar> {
        let (a, b) = match &value {
            Scalar::Pointer(_) => {
                let a = self.facts.type_of(from.target.as_deref()?)?;
                let b = self.facts.type_of(to.target.as_deref()?)?;
                if a.id == b.id {
                    return Some(value);
                }
                (a, b)
            }
            Scalar::Record(_) => (from, to),
            Scalar::Slice(view) if from.element == to.element && from.fixed == Some(false) => {
                if to.fixed == Some(false) {
                    return Some(value);
                }
                return self.slice_array(*view, usize::try_from(to.length?).ok()?);
            }
            Scalar::Sequence(_)
                if from.element == to.element
                    && from.fixed == Some(true)
                    && from.length == to.length =>
            {
                return Some(value);
            }
            _ => return None,
        };
        if a.kind != TypeKind::Record || b.kind != TypeKind::Record {
            return None;
        }
        let (xs, ys) = (a.fields.as_deref()?, b.fields.as_deref()?);
        // Tags affect type identity, but do not occupy storage.
        (xs.len() == ys.len()
            && xs
                .iter()
                .zip(ys)
                .all(|(x, y)| x.name == y.name && x.ty == y.ty && x.embedded == y.embedded))
        .then_some(value)
    }
    fn slice_array(&self, view: super::storage::View, length: usize) -> Option<Scalar> {
        if length > view.len {
            return None;
        }
        let items = (0..length)
            .map(|index| self.element(view, index).cloned())
            .collect::<Option<Vec<_>>>()?;
        Some(Scalar::Sequence(items.into()))
    }

    fn slice_pointer(
        &self,
        view: super::storage::View,
        (from, to): (&crate::cpg::model::Type, &crate::cpg::model::Type),
    ) -> Option<Scalar> {
        let array = self.facts.type_of(to.target.as_deref()?)?;
        if from.kind != TypeKind::Sequence
            || from.fixed != Some(false)
            || array.fixed != Some(true)
            || array.element != from.element
        {
            return None;
        }
        view.array_address(usize::try_from(array.length?).ok()?)
    }
    fn rounded_argument(
        &mut self,
        arg: &'a Node,
        to: &crate::cpg::model::Type,
        depth: usize,
    ) -> Option<Scalar> {
        let from = self.facts.ty(arg)?;
        let saved = self.round_at;
        if to.id == from.id && (to.kind == TypeKind::Float || self.complex_width(to).is_some()) {
            self.round_at = Some(&arg.id);
        }
        let value = self.value(arg, depth);
        self.round_at = saved;
        value
    }
    fn text_slice(&mut self, bytes: Vec<u8>, to: &crate::cpg::model::Type) -> Option<Scalar> {
        let element = self
            .facts
            .integer(self.facts.type_of(to.element.as_deref()?))?;
        let items = match element {
            Integer {
                bits: 8,
                signed: false,
            } if bytes.len() <= super::storage::MAX_ITEMS => bytes
                .into_iter()
                .map(|b| Scalar::Int(i128::from(b)))
                .collect(),
            Integer {
                bits: 32,
                signed: true,
            } => runes(&bytes)?,
            _ => return None,
        };
        // Conversion creates a nonnil value even for an empty string. Its capacity may exceed its length.
        self.fresh_slice(items, None)
    }

    fn slice_text(
        &self,
        view: super::storage::View,
        from: &crate::cpg::model::Type,
    ) -> Option<Scalar> {
        let element = self
            .facts
            .integer(self.facts.type_of(from.element.as_deref()?))?;
        let mut bytes = Vec::new();
        for i in 0..view.len {
            let Scalar::Int(n) = self.element(view, i)? else {
                return None;
            };
            match element {
                Integer {
                    bits: 8,
                    signed: false,
                } => bytes.push(u8::try_from(*n).ok()?),
                Integer {
                    bits: 32,
                    signed: true,
                } => bytes.extend_from_slice(rune(*n).encode_utf8(&mut [0; 4]).as_bytes()),
                _ => return None,
            }
        }
        Some(Scalar::Text(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_unicode_advances_one_byte_at_a_time() {
        assert_eq!(
            runes(&[0xe2, 0x82, b'!']),
            Some(vec![
                Scalar::Int(65533),
                Scalar::Int(65533),
                Scalar::Int(33)
            ])
        );
        assert_eq!(rune(0xd800), char::REPLACEMENT_CHARACTER);
    }
}
