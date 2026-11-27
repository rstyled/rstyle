use super::{DefinitionKind, Literal, Node, NodeKind, RefStatus, Scalar, Walk};

impl Walk<'_, '_, '_> {
    pub(super) fn literal(&self, node: &Node, value: &Literal) -> Option<Scalar> {
        match value.kind.as_str() {
            "integer" => Some(Scalar::Int(value.value.as_deref()?.parse().ok()?)),
            "bool" => Some(Scalar::Bool(value.value.as_deref()?.parse().ok()?)),
            "char" => {
                let mut chars = value.value.as_deref()?.chars();
                let rune = chars.next()?;
                chars
                    .next()
                    .is_none()
                    .then_some(Scalar::Int(i128::from(u32::from(rune))))
            }
            "string" => Some(Scalar::Text(self.bytes(node, value)?)),
            _ => None,
        }
    }

    fn bytes(&self, node: &Node, value: &Literal) -> Option<Vec<u8>> {
        // A lossy frontend may already have substituted a replacement character.
        // Only a lossless payload permits a byte comparison or an index.
        if let Some(text) = &value.value
            && !text.contains('\u{fffd}')
            && text.len() <= 4096
        {
            return Some(text.as_bytes().to_vec());
        }
        let entry = self
            .facts
            .tables
            .extension(&self.profile.byte_constants)?
            .get(&node.id)?;
        if entry.get("kind")?.as_str()? != "string" {
            return None;
        }
        let hex = entry.get("bytes")?.as_str()?;
        if hex.len() > 8192 || !hex.len().is_multiple_of(2) || !hex.is_ascii() {
            return None;
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect()
    }

    /// A known numeric value does not by itself establish that evaluating its operands
    /// cannot modify local storage. Restrict this shortcut to primitive constant trees.
    pub(super) fn constant_tree(&mut self, node: &Node, depth: usize) -> bool {
        if self.enter(node, depth).is_none() {
            return false;
        }
        match node.kind {
            NodeKind::Identifier => {
                self.facts
                    .reference(node)
                    .is_some_and(|r| r.status == RefStatus::Resolved)
                    && self
                        .facts
                        .resolved(node)
                        .is_some_and(|(_, def)| def.kind == DefinitionKind::Constant)
            }
            NodeKind::Literal => node.children.is_empty(),
            NodeKind::Operator | NodeKind::Cast if self.facts.reference(node).is_none() => {
                node.children.iter().all(|id| {
                    self.facts
                        .body
                        .node(id)
                        .is_ok_and(|n| self.constant_tree(n, depth + 1))
                })
            }
            _ => false,
        }
    }
}
