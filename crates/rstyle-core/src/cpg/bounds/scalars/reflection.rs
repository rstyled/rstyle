//! Observe ordered field metadata from a concrete reflect.TypeFor instantiation.
//! Unknown tag syntax, missing metadata and a nonstandard receiver stay unknown.
use super::{DefinitionKind, Node, NodeKind, Scalar, TypeKind, Walk};
use crate::cpg::model::{Definition, Dispatch, Type};

impl<'a> Walk<'_, '_, 'a> {
    pub(super) fn standard_type(&self, ty: &Type, operation: &str) -> bool {
        ty.definition
            .as_deref()
            .and_then(|id| self.facts.tables.definition(id).ok())
            .is_some_and(|d| d.standard && d.operation.as_deref() == Some(operation))
    }

    pub(super) fn same_reflected_type(&self, a: &str, b: &str) -> Option<bool> {
        if a == b {
            return Some(true);
        }
        let at = self.facts.type_of(a)?;
        let bt = self.facts.type_of(b)?;
        if at.definition != bt.definition {
            return Some(false);
        }
        let tags = self.facts.tables.extension(&self.profile.reflect.tags)?;
        (tags.get(a)? != tags.get(b)?).then_some(false)
    }

    pub(super) fn reflection_operation(&self, key: &str) -> bool {
        let p = &self.profile.reflect;
        key == p.type_for || key == p.type_field || key == p.tag_get
    }

    pub(super) fn reflection_call(
        &mut self,
        node: &'a Node,
        def: &Definition,
        depth: usize,
    ) -> Option<Scalar> {
        let p = &self.profile.reflect;
        let op = def.operation.as_deref()?;
        let receiver = self.child(node, 0)?;
        let r = self.facts.reference(node)?;
        if op == p.type_for {
            let [instance] = r.instance_arguments.as_deref()? else {
                return None;
            };
            if def.kind != DefinitionKind::Function
                || r.dispatch != Some(Dispatch::Static)
                || node.children.len() != 1
                || receiver.kind != NodeKind::Identifier
                || !receiver.children.is_empty()
                || self.facts.resolved(receiver)?.1.id != def.id
                || !self.standard_type(self.facts.ty(node)?, &p.type_object)
                || self.facts.type_of(instance)?.kind != TypeKind::Record
            {
                return None;
            }
            return Some(Scalar::ReflectedType(instance.to_owned()));
        }
        self.reflect_method(node, def, depth)
    }

    fn reflect_method(&mut self, node: &'a Node, def: &Definition, depth: usize) -> Option<Scalar> {
        let p = &self.profile.reflect;
        let op = def.operation.as_deref()?;
        let receiver = self.child(node, 0)?;
        let r = self.facts.reference(node)?;
        if def.kind != DefinitionKind::Method
            || node.children.len() != 2
            || def.callable.as_ref()?.receiver.as_ref() != receiver.ty.as_ref()
        {
            return None;
        }
        if op == p.type_field
            && r.dispatch == Some(Dispatch::Dynamic)
            && self.standard_type(self.facts.ty(node)?, &p.struct_field)
        {
            let Scalar::ReflectedType(id) = self.value(receiver, depth)? else {
                return None;
            };
            let index = self.index(self.child(node, 1)?, depth)?;
            return self.metadata_field(&id, index);
        }
        if op == p.tag_get
            && r.dispatch == Some(Dispatch::Method)
            && self.standard_type(self.facts.ty(receiver)?, &p.struct_tag)
            && self.facts.ty(node)?.kind == TypeKind::String
        {
            let Scalar::Text(tag) = self.value(receiver, depth)? else {
                return None;
            };
            let Scalar::Text(key) = self.value(self.child(node, 1)?, depth)? else {
                return None;
            };
            return Some(Scalar::Text(tag_get(&tag, &key)?.to_vec()));
        }
        None
    }

    fn metadata_field(&self, id: &str, index: usize) -> Option<Scalar> {
        self.facts.type_of(id)?.fields.as_ref()?.get(index)?;
        let tags = self
            .facts
            .tables
            .extension(&self.profile.reflect.tags)?
            .get(id)?
            .as_object()?;
        let hex = match tags.get(&index.to_string()) {
            Some(value) => value.as_str()?,
            None => "",
        };
        if hex.len() > 8192 || !hex.len().is_multiple_of(2) || !hex.is_ascii() {
            return None;
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect::<Option<Vec<_>>>()?;
        // Keep only the field whose value is established. Other metadata members
        // of the returned struct cannot be fetched from this partial record.
        Some(Scalar::Record(std::sync::Arc::new(
            std::collections::BTreeMap::from([("Tag".to_owned(), Scalar::Text(bytes))]),
        )))
    }
}

// Follow the valid, unescaped subset of Go's reflect.StructTag.Lookup syntax.
// An escape needs Go string unquoting, and malformed tags have unspecified Get results.
fn tag_get<'a>(mut bytes: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    std::str::from_utf8(bytes).ok()?;
    let mut found = None;
    loop {
        while bytes.first() == Some(&b' ') {
            bytes = &bytes[1..]
        }
        if bytes.is_empty() {
            return Some(found.unwrap_or(&[]));
        }
        let n = bytes
            .iter()
            .position(|b| *b <= b' ' || matches!(b, b':' | b'"' | 0x7f))?;
        let name = &bytes[..n];
        if name.is_empty() || !bytes[n..].starts_with(b":\"") {
            return None;
        }
        let value = &bytes[n + 2..];
        let end = value
            .iter()
            .position(|b| *b < b' ' || matches!(b, b'"' | b'\\'))?;
        if value[end] != b'"' {
            return None;
        }
        if name == key && found.is_none() {
            found = Some(&value[..end]);
        }
        bytes = &value[end + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::tag_get;

    #[test]
    fn only_known_well_formed_tags_yield_a_field_value() {
        assert_eq!(tag_get(br#"k:"ab" k:"cd""#, b"k"), Some(&b"ab"[..]));
        assert_eq!(tag_get(br#"k:"ab""#, b"absent"), Some(&b""[..]));
        assert_eq!(tag_get(br#"k:"ab" bad:"#, b"k"), None);
        assert_eq!(tag_get(br#"k:"a\t""#, b"k"), None);
    }
}
