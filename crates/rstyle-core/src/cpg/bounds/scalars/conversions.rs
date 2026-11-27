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
        let arg = self.child(node, 0)?;
        let to = self.facts.ty(node)?;
        let from = self.facts.ty(arg)?;
        let value = self.value(arg, depth)?;
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
            (Scalar::Slice(view), TypeKind::String) => {
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
            (Scalar::Slice(view), TypeKind::Sequence)
                if to.fixed == Some(true) && from.element == to.element =>
            {
                let length = usize::try_from(to.length?).ok()?;
                if length > view.len {
                    return None;
                }
                let items = (0..length)
                    .map(|i| self.element(view, i).cloned())
                    .collect::<Option<Vec<_>>>()?;
                Some(Scalar::Sequence(items.into()))
            }
            _ => None,
        }
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
