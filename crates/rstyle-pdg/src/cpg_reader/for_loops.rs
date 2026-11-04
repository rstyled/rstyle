//! The data profiles' source `for` loops over `u32` ranges, recorded as the loop with
//! its range, pattern and body.

use super::{Context, hir_ref};
use crate::frontend::{HirNode, PathRes};
use rstyle_core::cpg::model::{Definition, Node, NodeKind, Origin};

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
                .all(|child| self.reader.integer(child.ty.as_deref(), "u32"))
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
        let argument = matches!(ty.arguments.as_deref(), Some([argument]) if self.reader.integer(Some(argument), "u32"));

        (matches!(
            definition.name.as_str(),
            "core::ops::Range" | "std::ops::Range"
        ) && core
            && argument)
            .then_some(definition)
    }
}

impl<'a> Context<'_, 'a> {
    /// A destructuring assignment of a tuple to scalar places, as the profile restores it.
    pub(super) fn tuple_assignment(&self, node: &'a Node) -> Built {
        if node.kind != NodeKind::Assignment
            || node.original_kind != "expr_block"
            || !self.reader.profile.supports("rust.scalar-tuple-assignment")
        {
            return Ok(None);
        }
        let parts = self.body.children(node)?;
        let [target, value] = parts.as_slice() else {
            return Ok(None);
        };
        let places = self.body.children(target)?;
        if !self.scalar_places((target, value), &places)? {
            return Ok(None);
        }
        let left = HirNode {
            ty: self.ty(target)?,
            children: places
                .into_iter()
                .map(|place| self.expression(place))
                .collect::<Result<_, _>>()?,
            expansion: None,
            ..self.node(target, "expr_tup")?
        };

        Ok(Some(HirNode {
            ty: self.ty(node)?,
            children: vec![left, self.expression(value)?],
            expansion: None,
            ..self.node(node, "expr_tuple_assign")?
        }))
    }
}

impl Context<'_, '_> {
    /// Whether a destructuring assignment assigns a tuple, element by element, to
    /// scalar local places.
    fn scalar_places(
        &self,
        (target, value): (&Node, &Node),
        places: &[&Node],
    ) -> Result<bool, String> {
        let values = self.body.children(value)?;
        let scalar = |place: &&Node| {
            place.original_kind == "expr_path"
                && place
                    .ty
                    .as_deref()
                    .and_then(|ty| self.reader.ty(ty).ok())
                    .is_some_and(|ty| {
                        matches!(
                            ty.kind,
                            rstyle_core::cpg::model::TypeKind::Bool
                                | rstyle_core::cpg::model::TypeKind::Integer
                        )
                    })
        };
        Ok(target.original_kind == "pat_tuple"
            && value.original_kind == "expr_tup"
            && !places.is_empty()
            && places.len() == values.len()
            && places.iter().all(scalar))
    }
}
