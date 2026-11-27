//! Exact dynamic types of values assigned to interfaces. A copy preserves the
//! dynamic type and nilness of the original interface. A typed nil has a type.
use super::{Node, Scalar, TypeKind, Walk};
use crate::cpg::model::Type;

pub(super) fn primitive(ty: &Type) -> Option<&str> {
    if ty.definition.is_some()
        || !matches!(
            ty.kind,
            TypeKind::Integer | TypeKind::Float | TypeKind::Bool | TypeKind::String
        )
    {
        return None;
    }
    Some(match ty.spelling.as_str() {
        "byte" => "uint8",
        "rune" => "int32",
        other => other,
    })
}

fn same_type(a: &Type, b: &Type) -> Option<bool> {
    if a.id == b.id {
        return Some(true);
    }
    if a.kind != b.kind || a.definition != b.definition {
        return Some(false);
    }
    if a.definition.is_some() {
        // One named generic definition may have multiple instantiations.
        return None;
    }
    Some(primitive(a)? == primitive(b)?)
}

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn item_value(
        &mut self,
        ty: &Type,
        source: &'a Node,
        depth: usize,
    ) -> Option<Scalar> {
        let value = self.value(source, depth)?;
        if ty.kind == TypeKind::Interface {
            self.interface_value(ty, source, value)
        } else {
            Some(value)
        }
    }
    fn implements(&self, dynamic: &Type, target: &Type) -> Option<bool> {
        let rule = self
            .facts
            .tables
            .extension(&self.profile.interface_sets)?
            .get(&target.id)?;
        if rule.get("comparable")?.as_bool()? || !rule.get("elements")?.as_array()?.is_empty() {
            return None;
        }
        let methods = rule.get("methods")?.as_object()?;
        if methods.is_empty() {
            return Some(true);
        }
        // Predeclared scalar types have no methods.
        if primitive(dynamic).is_some() {
            return Some(false);
        }
        let got = self
            .facts
            .tables
            .extension(&self.profile.method_sets)?
            .get(&dynamic.id)?
            .as_object()?;
        for (name, sig) in methods {
            let Some(actual) = got.get(name) else {
                return Some(false);
            };
            // Different frontend IDs can still describe identical signatures.
            if sig.as_str()? != actual.as_str()? {
                return None;
            }
        }
        Some(true)
    }

    pub(super) fn interface_value(
        &self,
        target: &Type,
        source: &Node,
        value: Scalar,
    ) -> Option<Scalar> {
        if value == Scalar::Nil {
            return Some(Scalar::Interface(None));
        }
        if matches!(value, Scalar::ReflectedType(_)) {
            return (self.standard_type(target, &self.profile.reflect.type_object)
                && self.standard_type(self.facts.ty(source)?, &self.profile.reflect.type_object))
            .then_some(value);
        }
        let boxed = match value {
            v @ Scalar::Interface(None) => return Some(v),
            Scalar::Interface(Some(pair)) => pair,
            v => {
                let dtype = self.facts.ty(source)?;
                if matches!(dtype.kind, TypeKind::Interface | TypeKind::Parameter) {
                    return None;
                }
                Box::new((dtype.id.clone(), v))
            }
        };
        self.implements(self.facts.type_of(&boxed.0)?, target)?
            .then_some(Scalar::Interface(Some(boxed)))
    }

    pub(super) fn assert_type(&self, source: Scalar, to: &Type) -> Option<(Scalar, bool)> {
        let Scalar::Interface(contents) = source else {
            return None;
        };
        if let Some(pair) = contents {
            let from = self.facts.type_of(&pair.0)?;
            if to.kind == TypeKind::Interface {
                if self.implements(from, to)? {
                    return Some((Scalar::Interface(Some(pair)), true));
                }
            } else if same_type(from, to)? {
                return Some((pair.1, true));
            }
        }
        Some((self.zero(to)?, false))
    }

    pub(super) fn asserted(
        &mut self,
        node: &'a Node,
        depth: usize,
        paired: bool,
    ) -> Option<(Scalar, bool)> {
        if node.original_kind != self.profile.type_assertion
            || node.children.len() != 1
            || self.facts.reference(node).is_some()
        {
            return None;
        }
        let base = self.child(node, 0)?;
        if self.facts.ty(base)?.kind != TypeKind::Interface {
            return None;
        }
        let mut to = self.facts.ty(node)?;
        if paired {
            let items = to.items.as_deref()?;
            if to.kind != TypeKind::Tuple
                || items.len() != 2
                || self.facts.type_of(&items[1])?.kind != TypeKind::Bool
            {
                return None;
            }
            to = self.facts.type_of(&items[0])?;
        }
        let from = self.value(base, depth)?;
        self.assert_type(from, to)
    }

    pub(super) fn equal_objects(
        &mut self,
        (a, b): (&Scalar, &Scalar),
        ty: &Type,
        depth: usize,
    ) -> Option<bool> {
        use Scalar::*;
        self.remaining = self.remaining.checked_sub(1)?;
        if depth > 48 {
            return None;
        }
        Some(match (a, b) {
            (Int(a), Int(b)) => a == b,
            (Real(a), Real(b)) => a == b,
            (Complex(a, b), Complex(c, d)) => a == c && b == d,
            (Bool(a), Bool(b)) => a == b,
            (Text(a), Text(b)) => a == b,
            (Channel(a), Channel(b)) => a == b,
            (Pointer(a), Pointer(b)) => self.pointer_identity((a, b), ty)?,
            (Sequence(a), Sequence(b)) if ty.fixed == Some(true) && a.len() == b.len() => {
                let elem = self.facts.type_of(ty.element.as_deref()?)?;
                for (x, y) in a.iter().zip(b.iter()) {
                    if !self.equal_objects((x, y), elem, depth + 1)? {
                        return Some(false);
                    }
                }
                true
            }
            (Record(a), Record(b)) => {
                for f in ty.fields.as_ref()?.iter().filter(|f| f.name != "_") {
                    if !self.equal_objects(
                        (a.get(&f.name)?, b.get(&f.name)?),
                        self.facts.type_of(&f.ty)?,
                        depth + 1,
                    )? {
                        return Some(false);
                    }
                }
                true
            }
            (Interface(None), Interface(_)) => matches!(b, Interface(None)),
            (Interface(_), Interface(None)) => false,
            (Interface(Some(a)), Interface(Some(b))) => {
                let at = self.facts.type_of(&a.0)?;
                same_type(at, self.facts.type_of(&b.0)?)?
                    && self.equal_objects((&a.1, &b.1), at, depth + 1)?
            }
            _ => return None,
        })
    }

    fn pointer_identity(
        &self,
        (a, b): (
            &Option<super::pointers::Address>,
            &Option<super::pointers::Address>,
        ),
        ty: &Type,
    ) -> Option<bool> {
        use super::pointers::Address;
        if a == b || a.is_none() || b.is_none() {
            return Some(a == b);
        }
        primitive(self.facts.type_of(ty.target.as_deref()?)?)?;
        match (a.as_ref()?, b.as_ref()?) {
            (Address::Local(x), Address::Local(y)) => Some(x == y),
            (Address::Cell(s, i), Address::Cell(t, j)) if s == t => Some(i == j),
            _ => None,
        }
    }

    pub(super) fn equality(
        &mut self,
        node: &'a Node,
        (a, b): (Scalar, Scalar),
        depth: usize,
    ) -> Option<Scalar> {
        let (x, y) = (self.child(node, 0)?, self.child(node, 1)?);
        let (xt, yt) = (self.facts.ty(x)?, self.facts.ty(y)?);
        let (a, b, ty) = if xt.kind == TypeKind::Interface {
            (a, self.interface_value(xt, y, b)?, xt)
        } else if yt.kind == TypeKind::Interface {
            (self.interface_value(yt, x, a)?, b, yt)
        } else {
            (a, b, xt)
        };
        Some(Scalar::Bool(
            self.equal_objects((&a, &b), ty, depth)? == (node.operator.as_deref() == Some("eq")),
        ))
    }
}
