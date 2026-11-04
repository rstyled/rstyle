//! The data profiles' source `for` loops over `u32` ranges, recorded as the loop with
//! its range, pattern and body.

use super::{Context, hir_ref};
use crate::frontend::{HirNode, PathRes};
use rstyle_core::cpg::model::{Definition, Node, NodeKind, Origin, TypeKind};

type Built = Result<Option<HirNode>, String>;

impl<'a> Context<'_, 'a> {
    pub(super) fn original_for_loop(&self, node: &'a Node) -> Built {
        if !self.reader.profile.is_data() || node.kind != NodeKind::For {
            return Ok(None);
        }
        let parts = self.body.children(node)?;
        let [pattern, iterable, body, _, iteration] = parts.as_slice() else {
            return Ok(None);
        };
        let plain = pattern.original_kind == "pat_wild"
            || pattern.original_kind == "pat_binding"
                && self.declared(pattern).is_some_and(|(_, mutable)| !mutable);
        let Some(head) = self.body.children(iterable)?.get(1).copied() else {
            return Ok(None);
        };
        let Some(head) = self.original_range(head)? else {
            return Ok(None);
        };
        if !plain || iteration.original_kind != "expr_for" || body.original_kind != "expr_block" {
            return Ok(None);
        }
        let base = self.node(node, "expr_for")?;
        Ok(Some(HirNode {
            ty: self.ty(node)?,
            res: Some(PathRes {
                kind: "innermost_loop".into(),
                target: Some(hir_ref(&iteration.id)),
                description: "Compiler-resolved original for-loop destination".into(),
            }),
            children: vec![head, self.pattern(pattern)?, self.expression(body)?],
            expansion: None,
            ..base
        }))
    }

    /// A `start..end` range of `u32`s, as the library range it builds.
    fn original_range(&self, head: &'a Node) -> Built {
        let range = head.provenance.origin == Origin::Source
            || head.provenance.construct.as_deref() == Some("rust:range");
        let fields = head.members.as_deref() == Some(&["start".to_owned(), "end".to_owned()][..]);
        let children = self.body.children(head)?;
        let Some(definition) = self.u32_range(head) else {
            return Ok(None);
        };
        if head.original_kind != "expr_struct"
            || !range
            || !fields
            || children.len() != 2
            || !children
                .iter()
                .all(|child| self.is_u32(child.ty.as_deref()))
        {
            return Ok(None);
        }
        let Some(mut resolution) = self.library_callee(definition) else {
            return Ok(None);
        };
        resolution.kind = "resolved_u32_range_type".into();
        Ok(Some(HirNode {
            ty: self.ty(head)?,
            res: Some(resolution),
            children: children
                .into_iter()
                .map(|child| self.expression(child))
                .collect::<Result<_, _>>()?,
            expansion: None,
            ..self.node(head, "expr_struct")?
        }))
    }

    /// The core `Range<u32>` a node's type is.
    fn u32_range(&self, node: &Node) -> Option<&'a Definition> {
        let ty = self.reader.ty(node.ty.as_deref()?).ok()?;
        let definition = self
            .reader
            .tables
            .definition(ty.definition.as_deref()?)
            .ok()?;
        let core = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("crate_name")?.as_str())
            == Some("core");
        let argument =
            matches!(ty.arguments.as_deref(), Some([argument]) if self.is_u32(Some(argument)));
        (matches!(
            definition.name.as_str(),
            "core::ops::Range" | "std::ops::Range"
        ) && core
            && argument)
            .then_some(definition)
    }

    fn is_u32(&self, ty: Option<&str>) -> bool {
        ty.and_then(|ty| self.reader.ty(ty).ok())
            .is_some_and(|ty| ty.kind == TypeKind::Integer && ty.spelling == "u32")
    }
}
