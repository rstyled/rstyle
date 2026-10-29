//! Preserve deep HIR trees without recursive JSON containers or body copies.
use crate::frontend::*;
use serde::{Deserialize, Serialize, de, ser::SerializeSeq};
use std::fmt;

pub(super) struct Export<'a>(pub &'a HirNode);

pub(super) struct Import {
    node: HirNode,
    flat: bool,
}

impl Import {
    pub fn into_node(self, allow_flat: bool) -> Result<HirNode, String> {
        if self.flat && !allow_flat {
            return Err("Flat HIR body requires its registered transport version".into());
        }
        Ok(self.node)
    }
}

impl Serialize for Export<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if !deep(self.0) {
            return self.0.serialize(serializer);
        }

        let mut output = serializer.serialize_seq(None)?;
        let mut pending = vec![self.0];
        while let Some(node) = pending.pop() {
            output.serialize_element(&Record {
                node: Leaf::new(node),
                children: node.children.len(),
            })?;
            pending.extend(node.children.iter().rev());
        }
        output.end()
    }
}

// Representation selection, not a semantic/work budget. Keep shallow JSON
// readable while leaving ample room beneath the JSON reader's existing limit.
fn deep(root: &HirNode) -> bool {
    let mut pending = vec![(root, 0)];
    while let Some((node, depth)) = pending.pop() {
        if depth >= 32 {
            return true;
        }
        pending.extend(node.children.iter().map(|child| (child, depth + 1)));
    }
    false
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record<N> {
    node: N,
    children: usize,
}

#[derive(Serialize)]
struct Leaf<'a> {
    hir: &'a HirRef,
    kind: &'a str,
    span: ByteSpan,
    snippet: &'a str,
    ty: &'a Option<ResolvedTy>,
    res: &'a Option<PathRes>,
    dispatch: &'a Option<Dispatch>,
    expansion: &'a Option<ExpansionSite>,
    binding: &'a Option<String>,
    binding_mut: Option<bool>,
    lit: &'a Option<String>,
    param_index: Option<u32>,
    is_unsafe_block: bool,
    children: &'a [HirNode],
}

impl<'a> Leaf<'a> {
    fn new(node: &'a HirNode) -> Self {
        Self {
            hir: &node.hir,
            kind: &node.kind,
            span: node.span,
            snippet: &node.snippet,
            ty: &node.ty,
            res: &node.res,
            dispatch: &node.dispatch,
            expansion: &node.expansion,
            binding: &node.binding,
            binding_mut: node.binding_mut,
            lit: &node.lit,
            param_index: node.param_index,
            is_unsafe_block: node.is_unsafe_block,
            children: &[],
        }
    }
}

impl<'de> Deserialize<'de> for Import {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(BodyVisitor)
    }
}

struct BodyVisitor;

impl<'de> de::Visitor<'de> for BodyVisitor {
    type Value = Import;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an original HIR tree or flat preorder body")
    }

    fn visit_map<M: de::MapAccess<'de>>(self, map: M) -> Result<Import, M::Error> {
        HirNode::deserialize(de::value::MapAccessDeserializer::new(map))
            .map(|node| Import { node, flat: false })
    }

    fn visit_seq<S: de::SeqAccess<'de>>(self, sequence: S) -> Result<Import, S::Error> {
        let records =
            Vec::<Record<HirNode>>::deserialize(de::value::SeqAccessDeserializer::new(sequence))?;
        restore(records)
            .map(|node| Import { node, flat: true })
            .map_err(de::Error::custom)
    }
}

fn restore(records: Vec<Record<HirNode>>) -> Result<HirNode, String> {
    let mut forest = Vec::new();
    for Record { mut node, children } in records.into_iter().rev() {
        if !node.children.is_empty() || children > forest.len() {
            return Err("Flat HIR child roster differs from preorder".into());
        }
        node.children = (0..children)
            .map(|_| forest.pop().expect("Available preorder child"))
            .collect();
        forest.push(node);
    }
    if forest.len() != 1 {
        return Err("Flat HIR body must contain exactly one rooted tree".into());
    }
    Ok(forest.pop().expect("One rooted tree"))
}

#[cfg(test)]
mod tests;
