//! Preserve deep HIR trees without recursive JSON containers or body copies.
use crate::frontend::*;
use serde::{Deserialize, Serialize, de, ser::SerializeSeq};
use std::fmt;

pub(in crate::unit_store::bundle) struct Export<'a>(pub &'a HirNode);

pub(crate) struct Import<N = HirNode> {
    node: N,
    flat: bool,
}

impl<N: BodyNode> Import<N> {
    pub fn into_node(self, allow_flat: bool) -> Result<N::Output, String> {
        if self.flat && !allow_flat {
            return Err("Flat HIR body requires its registered transport version".into());
        }
        Ok(self.node.into_body())
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
pub(crate) struct Record<N> {
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

impl<'de, N: BodyNode + Deserialize<'de>> Deserialize<'de> for Import<N> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(BodyVisitor(std::marker::PhantomData))
    }
}

struct BodyVisitor<N>(std::marker::PhantomData<N>);

impl<'de, N: BodyNode + Deserialize<'de>> de::Visitor<'de> for BodyVisitor<N> {
    type Value = Import<N>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an original HIR tree or flat preorder body")
    }

    fn visit_map<M: de::MapAccess<'de>>(self, map: M) -> Result<Import<N>, M::Error> {
        N::deserialize(de::value::MapAccessDeserializer::new(map))
            .map(|node| Import { node, flat: false })
    }

    fn visit_seq<S: de::SeqAccess<'de>>(self, sequence: S) -> Result<Import<N>, S::Error> {
        let records =
            Vec::<Record<N>>::deserialize(de::value::SeqAccessDeserializer::new(sequence))?;
        N::restore(records)
            .map(|node| Import { node, flat: true })
            .map_err(de::Error::custom)
    }
}

pub(crate) trait BodyNode: Sized {
    type Output;
    fn into_body(self) -> Self::Output;
    fn restore(records: Vec<Record<Self>>) -> Result<Self, String>;
}

impl BodyNode for HirNode {
    type Output = HirNode;
    fn into_body(self) -> HirNode {
        self
    }
    fn restore(records: Vec<Record<Self>>) -> Result<Self, String> {
        restore(records)
    }
}

pub(crate) struct HeaderNode {
    hir: HirRef,
    children: usize,
}

impl<'de> Deserialize<'de> for HeaderNode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let node = HeaderFields::deserialize(deserializer)?;
        let header = Self {
            hir: node.hir,
            children: node.children.0,
        };
        drop((
            node.kind,
            node.snippet,
            node.ty,
            node.res,
            node.dispatch,
            node.expansion,
            node.binding,
            node.binding_mut,
            node.lit,
            node.param_index,
            node.is_unsafe_block,
            node.span,
        ));
        Ok(header)
    }
}

impl BodyNode for HeaderNode {
    type Output = HirRef;
    fn into_body(self) -> HirRef {
        self.hir
    }
    fn restore(records: Vec<Record<Self>>) -> Result<Self, String> {
        let mut roots = 0usize;
        let mut root = None;
        for Record { node, children } in records.into_iter().rev() {
            if node.children != 0 || children > roots {
                return Err("Flat HIR child roster differs from preorder".into());
            }
            roots = roots - children + 1;
            root = Some(node);
        }

        if roots != 1 {
            return Err("Flat HIR body must contain exactly one rooted tree".into());
        }
        Ok(root.expect("One rooted tree"))
    }
}

crate::frontend::hir_node!(HeaderFields, DiscardedText, DiscardedChildren, Deserialize);

struct DiscardedChildren(usize);

impl<'de> Deserialize<'de> for DiscardedChildren {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ChildrenVisitor;
        impl<'de> de::Visitor<'de> for ChildrenVisitor {
            type Value = DiscardedChildren;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an HIR children sequence")
            }
            fn visit_seq<S: de::SeqAccess<'de>>(
                self,
                mut sequence: S,
            ) -> Result<Self::Value, S::Error> {
                let mut count = 0;
                while sequence.next_element::<HeaderNode>()?.is_some() {
                    count += 1;
                }
                Ok(DiscardedChildren(count))
            }
        }
        deserializer.deserialize_seq(ChildrenVisitor)
    }
}

struct DiscardedText;

impl<'de> Deserialize<'de> for DiscardedText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextVisitor;
        impl<'de> de::Visitor<'de> for TextVisitor {
            type Value = DiscardedText;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an HIR string")
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(DiscardedText)
            }
        }
        deserializer.deserialize_str(TextVisitor)
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
