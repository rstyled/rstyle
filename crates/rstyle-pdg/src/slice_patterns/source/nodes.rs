//! Exact recorded rosters and compiler observations, independent of source grammar.
use super::*;
use crate::{data_types::TypeToken, frontend::HirRef};
use std::collections::BTreeSet;

impl Proof<'_, '_> {
    pub(super) fn observed(
        &self,
        node: &HirNode,
        expected: &TypeShape,
    ) -> Result<Observation, String> {
        if node.res.is_none() {
            return Err(unknown("Missing raw pattern receipt"));
        }
        let receipt = Observation::decode(node)?;
        match &receipt.status {
            CaptureStatus::Captured => {}
            CaptureStatus::Unsupported(reason) => return Err(unsupported(reason)),
            CaptureStatus::Unknown(reason) => return Err(unknown(reason)),
        }

        if node.expansion.is_some()
            || node.dispatch.is_some()
            || node.is_unsafe_block
            || node.param_index.is_some()
            || node.lit.is_some()
        {
            return Err(unsupported(
                "Pattern carries unexpected expansion or expression metadata",
            ));
        }
        let bytes = crate::source_evidence::byte_range(&self.facts.source.bytes, node.span)?;
        if bytes != node.snippet {
            return Err(unsupported("Pattern source bytes disagree"));
        }

        let ty = node
            .ty
            .as_ref()
            .ok_or_else(|| unknown("Missing pattern type"))?;
        let normalized = ty
            .normalized
            .as_ref()
            .ok_or_else(|| unknown("Missing normalized pattern type"))?;
        if receipt.ty != *expected || crate::data_types::parse(normalized)? != token(expected)? {
            return Err(unsupported(
                "Raw and normalized compiler pattern types disagree",
            ));
        }

        Ok(receipt)
    }

    pub(super) fn root_observation(&self, count: usize) -> Result<(), String> {
        let slice = TypeShape::Slice(Box::new(TypeShape::Byte));
        let receipt = self.observed(self.root, &slice)?;
        let Details::Slice {
            prefix,
            rest,
            suffix,
        } = receipt.details
        else {
            return Err(unsupported("Missing slice roster"));
        };

        if self.root.kind != "pat_slice_prefix"
            || self.root.binding.is_some()
            || self.root.binding_mut.is_some()
            || self.root.children.len() != count + 1
            || prefix.len() != count
            || !suffix.is_empty()
            || prefix
                .iter()
                .ne(self.root.children[..count].iter().map(|node| &node.hir))
            || rest.as_ref() != Some(&self.root.children[count].hir)
        {
            return Err(unsupported(
                "Original prefix/rest roster disagrees with recorded children",
            ));
        }

        let expected = Adjustment {
            kind: AdjustmentKind::BuiltinDeref,
            source: TypeShape::Shared(Box::new(slice)),
        };
        if receipt.adjustments != [expected] {
            return Err(unsupported(
                "Expected exactly one builtin shared byte-slice dereference",
            ));
        }
        Ok(())
    }

    pub(super) fn leaf(
        &self,
        node: &HirNode,
        kind: &str,
        shape: &TypeShape,
    ) -> Result<Observation, String> {
        let receipt = self.observed(node, shape)?;
        if node.kind != kind
            || !receipt.adjustments.is_empty()
            || node.binding.is_some()
            || node.binding_mut.is_some()
        {
            return Err(unsupported(
                "Unexpected leaf pattern role, binding or adjustment",
            ));
        }
        Ok(receipt)
    }
}

fn token(shape: &TypeShape) -> Result<TypeToken, String> {
    match shape {
        TypeShape::Byte => Ok(TypeToken::Scalar("u8".into())),
        TypeShape::Slice(inner) => Ok(TypeToken::Slice(Box::new(token(inner)?))),
        TypeShape::Shared(inner) => Ok(TypeToken::Shared(Box::new(token(inner)?))),
        TypeShape::Missing => Err(unknown("Missing compiler type shape")),
        _ => Err(unsupported(
            "Pattern type is outside immutable byte grammar",
        )),
    }
}

pub(super) fn unique_ids(root: &HirNode) -> Result<(), String> {
    fn visit(node: &HirNode, owner: &HirRef, ids: &mut BTreeSet<(u32, u32)>) -> bool {
        node.hir.krate == 0
            && node.hir.owner == owner.owner
            && ids.insert((node.hir.owner, node.hir.local))
            && node.children.iter().all(|child| visit(child, owner, ids))
    }
    if !visit(root, &root.hir, &mut BTreeSet::new()) {
        return Err(unsupported(
            "Pattern contains foreign or duplicate HIR identities",
        ));
    }
    Ok(())
}
