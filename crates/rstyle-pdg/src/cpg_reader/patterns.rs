//! Scalar patterns: alternatives, ranges and constants of a scalar type, where the
//! profile observes them.

use super::Context;
use crate::frontend::{HirNode, PathRes};
use crate::scalar_constants::ScalarConstant;
use rstyle_core::cpg::model::{Node, TargetKind, TypeKind};

type Built = Result<Option<HirNode>, String>;

impl<'a> Context<'_, 'a> {
    pub(super) fn scalar_pattern(&self, pattern: &'a Node) -> Built {
        match pattern.original_kind.as_str() {
            "pat_or" => self.or_pattern(pattern),
            "pat_range" => self.range_pattern(pattern),
            "pat_path" => self.constant_pattern(pattern),
            _ => Ok(None),
        }
    }

    fn scalar_kind(&self, pattern: &Node, character: &str) -> bool {
        let Some(ty) = pattern.ty.as_deref().and_then(|ty| self.reader.ty(ty).ok()) else {
            return false;
        };
        match ty.kind {
            TypeKind::Integer => true,
            TypeKind::Char => self.reader.profile.supports(character),
            _ => false,
        }
    }

    fn or_pattern(&self, pattern: &'a Node) -> Built {
        let bool_typed = pattern
            .ty
            .as_deref()
            .and_then(|ty| self.reader.ty(ty).ok())
            .is_some_and(|ty| ty.kind == TypeKind::Bool);
        if !self.reader.profile.supports("rust.scalar-or-patterns")
            || !(bool_typed || self.scalar_kind(pattern, "rust.character-scalars"))
        {
            return Ok(None);
        }
        let children = self
            .parts(pattern)?
            .into_iter()
            .map(|alternative| self.pattern(alternative))
            .collect::<Result<_, _>>()?;
        Ok(Some(HirNode {
            children,
            ..self.pattern_node(pattern, "pat_or")?
        }))
    }

    fn range_pattern(&self, pattern: &'a Node) -> Built {
        let bounds = self.parts(pattern)?;
        let mut constants = Vec::new();
        let mut texts = Vec::new();
        for bound in &bounds {
            let text = if bound.original_kind == "pat_expr_path" {
                let Some(constant) = self.range_constant(pattern, bound)? else {
                    return Ok(None);
                };
                let text = constant.lit.clone().unwrap_or_default();
                constants.push(constant);
                text
            } else {
                self.snippet(self.written(bound))
            };
            texts.push((bound, text));
        }
        if !self.reader.profile.supports("rust.scalar-range-patterns")
            || !self.scalar_kind(pattern, "rust.character-literal-ranges")
        {
            return Ok(None);
        }
        // A lone bound is the start when the range begins with it.
        let starts = |bound: &Node| {
            bound.span.as_ref().map(|span| span.start)
                == pattern.span.as_ref().map(|span| span.start)
        };
        let (start, end) = match texts.as_slice() {
            [(_, start), (_, end)] => (Some(start.clone()), Some(end.clone())),
            [(bound, text)] if starts(bound) => (Some(text.clone()), None),
            [(_, text)] => (None, Some(text.clone())),
            _ => (None, None),
        };
        let inclusive = end.is_some() && pattern.modifiers.iter().any(|m| m == "inclusive");
        let lit = crate::scalar_patterns::IntegerRange {
            start,
            end,
            inclusive,
        }
        .text();

        Ok(Some(HirNode {
            lit: Some(lit),
            children: constants,
            ..self.pattern_node(pattern, "pat_range")?
        }))
    }

    /// A range bound naming a local scalar constant, as the constant it evaluates to.
    fn range_constant(&self, pattern: &'a Node, bound: &'a Node) -> Built {
        if !self
            .reader
            .profile
            .supports("rust.scalar-constant-range-bounds")
        {
            return Ok(None);
        }
        let Some(resolution) = self.pattern_constant(bound) else {
            return Ok(None);
        };
        let Ok(constant) = ScalarConstant::decode(&resolution) else {
            return Ok(None);
        };
        Ok(Some(HirNode {
            ty: self.ty(pattern)?,
            lit: Some(constant.value),
            res: Some(resolution),
            ..self.node(bound, "pat_constant")?
        }))
    }

    fn constant_pattern(&self, pattern: &'a Node) -> Built {
        if !self
            .reader
            .profile
            .supports("rust.scalar-constant-patterns")
        {
            return Ok(None);
        }
        let Some(resolution) = self.pattern_constant(pattern) else {
            return Ok(None);
        };
        let lit = ScalarConstant::decode(&resolution)
            .ok()
            .map(|constant| constant.value);
        Ok(Some(HirNode {
            lit,
            res: Some(resolution),
            ..self.pattern_node(pattern, "pat_constant")?
        }))
    }

    /// The local scalar constant a pattern's path names.
    fn pattern_constant(&self, path: &Node) -> Option<PathRes> {
        let definition = self.target_definition(path, TargetKind::Definition)?;
        self.constant_resolution(definition)
    }
}
