//! Retain a symbolic base and byte displacement when Go converts a live object
//! address to uintptr. Absolute addresses and differences between unrelated
//! allocations remain unknown.
use super::pointers::Address;
use super::{Scalar, TypeKind, Walk};
use crate::cpg::model::Type;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::cpg::bounds) struct Word {
    base: Address,
    offset: i128,
    alignment: i128,
}

impl Walk<'_, '_, '_> {
    pub(super) fn uintptr(&self, ty: &Type) -> bool {
        ty.kind == TypeKind::Integer
            && ty.pointer_sized == Some(true)
            && ty.signed == Some(false)
            && ty.definition.is_none()
            && ty.spelling == self.profile.uintptr
    }

    pub(super) fn address_word(&self, addr: &Address, origin: &str) -> Option<Scalar> {
        let (word, ty) = self.location(addr)?;
        (ty.id == origin).then_some(Scalar::AddressWord(word))
    }

    fn location(&self, addr: &Address) -> Option<(Word, &Type)> {
        let layouts = self.facts.tables.extension(&self.profile.address_layouts)?;
        match addr {
            Address::Local(id) => {
                let sym = self
                    .facts
                    .body
                    .function
                    .symbols
                    .iter()
                    .find(|s| s.id == *id)?;
                let ty = self.facts.type_of(sym.ty.as_deref()?)?;
                let align = layouts.get(&ty.id)?.get("align")?.as_u64()?;
                if !align.is_power_of_two() {
                    return None;
                }
                Some((
                    Word {
                        base: addr.clone(),
                        offset: 0,
                        alignment: i128::from(align),
                    },
                    ty,
                ))
            }
            Address::Field(base, name) => {
                let (mut word, ty) = self.location(base)?;
                if ty.kind != TypeKind::Record {
                    return None;
                }
                let fields = ty.fields.as_deref()?;
                let (i, field) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| field.name == *name)?;
                let layout = layouts.get(&ty.id)?;
                let offsets = layout.get("offsets")?.as_array()?;
                let at = offsets.get(i)?.as_u64()?;
                if offsets.len() != fields.len() || at > layout.get("size")?.as_u64()? {
                    return None;
                }
                word.offset = word.offset.checked_add(i128::from(at))?;
                Some((word, self.facts.type_of(&field.ty)?))
            }
            _ => None,
        }
    }

    pub(super) fn word_operation(&self, op: &str, a: Scalar, b: Scalar) -> Option<Scalar> {
        use Scalar::{AddressWord, Bool, Int};
        let modulus = 1i128 << self.facts.pointer_bits;
        match (a, b) {
            (AddressWord(mut word), Int(delta)) if op == "add" || op == "sub" => {
                let delta = delta.rem_euclid(modulus);
                word.offset =
                    (word.offset + if op == "add" { delta } else { -delta }).rem_euclid(modulus);
                Some(AddressWord(word))
            }
            (Int(delta), AddressWord(word)) if op == "add" => {
                self.word_operation(op, AddressWord(word), Int(delta))
            }
            (AddressWord(word), Int(n)) if op == "rem" && n > 0 && word.alignment % n == 0 => {
                Some(Int(word.offset.rem_euclid(n)))
            }
            (AddressWord(a), AddressWord(b)) if a.base == b.base => match op {
                "eq" | "ne" => Some(Bool((a.offset == b.offset) == (op == "eq"))),
                "sub" => Some(Int((a.offset - b.offset).rem_euclid(modulus))),
                _ => None,
            },
            _ => None,
        }
    }
}
