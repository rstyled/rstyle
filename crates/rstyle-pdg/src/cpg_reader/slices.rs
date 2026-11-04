//! Byte-slice prefix patterns, recorded with the compiler observation the frontend
//! attaches to each nested pattern.

use super::{Context, hir_ref, symbol_name};
use crate::frontend::{HirNode, PathRes};
use crate::slice_patterns::{
    Adjustment, AdjustmentKind, Binding, BindingMode, CAPABILITY, CaptureStatus, Details, IDENTITY,
    Observation, TypeShape,
};
use rstyle_core::cpg::model::{Node, Origin, TypeKind};

type Built = Result<Option<HirNode>, String>;

impl<'a> Context<'_, 'a> {
    pub(super) fn slice_prefix_pattern(&self, pattern: &'a Node) -> Built {
        if !self.reader.profile.supports(CAPABILITY) || pattern.original_kind != "pat_slice" {
            return Ok(None);
        }
        self.slice_pattern(pattern, false).map(Some)
    }

    fn slice_pattern(&self, pattern: &'a Node, rest: bool) -> Result<HirNode, String> {
        let kind = match pattern.original_kind.as_str() {
            "pat_slice" => "pat_slice_prefix",
            "pat_binding" | "pat_binding_pattern" => "pat_binding_at",
            "pat_range" => "pat_slice_range",
            "pat_wild" if rest => "pat_slice_rest",
            _ => "pat_slice_unsupported",
        };
        let symbol = self.declared(pattern);
        let base = self.node(pattern, kind)?;
        let observation = self.slice_observation(pattern, rest, base.span)?;

        Ok(HirNode {
            ty: self.ty(pattern)?,
            binding: symbol.map(|(symbol, _)| symbol_name(symbol)),
            binding_mut: symbol.map(|(_, mutable)| mutable),
            expansion: self.expansion(pattern, base.span),
            children: self.slice_children(pattern)?,
            res: observation.resolution().ok(),
            ..base
        })
    }

    fn slice_children(&self, pattern: &'a Node) -> Result<Vec<HirNode>, String> {
        let children = self.parts(pattern)?;
        match pattern.original_kind.as_str() {
            "pat_slice" => children
                .into_iter()
                .map(|child| self.slice_pattern(child, is_rest(child)))
                .collect(),
            "pat_binding" | "pat_binding_pattern" => children
                .into_iter()
                .map(|child| self.slice_pattern(child, is_rest(pattern)))
                .collect(),
            "pat_tuple_struct" | "pat_tuple" | "pat_or" | "pat_deref" | "pat_ref" => children
                .into_iter()
                .map(|child| self.slice_pattern(child, false))
                .collect(),
            "pat_range" => children
                .into_iter()
                .map(|bound| self.slice_bound(bound))
                .collect(),
            "pat_guard" => children
                .into_iter()
                .enumerate()
                .map(|(index, child)| match index {
                    0 => self.slice_pattern(child, false),
                    _ => self.expression(child),
                })
                .collect(),
            _ => Ok(Vec::new()),
        }
    }

    fn slice_observation(
        &self,
        pattern: &'a Node,
        rest: bool,
        span: crate::ByteSpan,
    ) -> Result<Observation, String> {
        Ok(Observation {
            identity: IDENTITY.into(),
            hir: hir_ref(&pattern.id),
            span,
            status: self.slice_status(pattern, rest)?,
            ty: self.shape(pattern.ty.as_deref()),
            adjustments: pattern
                .adjustments
                .iter()
                .flatten()
                .map(|adjustment| Adjustment {
                    kind: match adjustment.kind.as_str() {
                        "auto_deref" => AdjustmentKind::BuiltinDeref,
                        "overloaded_deref" => AdjustmentKind::OverloadedDeref,
                        _ => AdjustmentKind::PinDeref,
                    },
                    source: self.shape(adjustment.target.as_deref()),
                })
                .collect(),
            details: self.slice_details(pattern, rest)?,
        })
    }

    fn slice_details(&self, pattern: &'a Node, rest: bool) -> Result<Details, String> {
        let children = self.parts(pattern)?;
        Ok(match pattern.original_kind.as_str() {
            "pat_slice" => {
                let middle = children.iter().position(|child| is_rest(child));
                let refs = |nodes: &[&Node]| nodes.iter().map(|node| hir_ref(&node.id)).collect();
                match middle {
                    Some(middle) => Details::Slice {
                        prefix: refs(&children[..middle]),
                        rest: Some(hir_ref(&children[middle].id)),
                        suffix: refs(&children[middle + 1..]),
                    },
                    None => Details::Slice {
                        prefix: refs(&children),
                        rest: None,
                        suffix: Vec::new(),
                    },
                }
            }
            "pat_binding" | "pat_binding_pattern" => Details::Binding {
                binding: self.slice_binding(pattern)?,
                subpattern: children.first().map(|child| hir_ref(&child.id)),
            },
            "pat_range" => {
                let (start, end) = self.range_bounds(pattern, &children);
                Details::Range {
                    start: start.map(|bound| hir_ref(&bound.id)),
                    end: end.map(|bound| hir_ref(&bound.id)),
                    inclusive: pattern.modifiers.iter().any(|m| m == "inclusive"),
                }
            }
            "pat_wild" if rest => Details::Rest,
            kind => Details::Unsupported {
                compiler_kind: compiler_kind(kind).into(),
            },
        })
    }

    /// A range pattern's start and end bounds; a lone bound is the start when the
    /// range begins with it.
    fn range_bounds(
        &self,
        pattern: &Node,
        bounds: &[&'a Node],
    ) -> (Option<&'a Node>, Option<&'a Node>) {
        let starts = |bound: &Node| {
            bound.span.as_ref().map(|span| span.start)
                == pattern.span.as_ref().map(|span| span.start)
        };
        match bounds {
            [start, end] => (Some(start), Some(end)),
            [bound] if starts(bound) => (Some(bound), None),
            [bound] => (None, Some(bound)),
            _ => (None, None),
        }
    }

    fn slice_binding(&self, pattern: &Node) -> Result<Binding, String> {
        let (symbol, mutable) = self
            .declared(pattern)
            .ok_or("Slice binding has no symbol")?;
        let symbol = self.body.symbol(symbol)?;
        Ok(Binding {
            canonical: hir_ref(&symbol.id),
            name: symbol.name.clone(),
            identifier_span: symbol
                .declared
                .as_ref()
                .map_or(crate::ByteSpan { start: 0, end: 0 }, |span| self.span(span)),
            written_mode: self.written_mode(pattern),
            written_mutable: mutable,
            effective_mode: pattern.binding_mode.as_deref().map(effective_mode),
            effective_mutable: pattern.binding_mode.as_ref().map(|_| mutable),
        })
    }

    /// How a binding is written to bind: by value, or `ref` and `ref mut`.
    fn written_mode(&self, pattern: &Node) -> BindingMode {
        let text = self.snippet(self.written(pattern));
        match text.strip_prefix("ref") {
            Some(rest) if rest.trim_start().starts_with("mut ") => BindingMode::Mutable,
            Some(rest) if rest.starts_with(char::is_whitespace) => BindingMode::Shared,
            _ => BindingMode::Value,
        }
    }

    fn slice_status(&self, pattern: &'a Node, rest: bool) -> Result<CaptureStatus, String> {
        if pattern.ty.is_none() {
            return Ok(CaptureStatus::Unknown(
                "Missing compiler pattern type".into(),
            ));
        }
        if pattern.provenance.origin != Origin::Source {
            return Ok(unsupported("Expanded pattern source"));
        }
        let children = self.parts(pattern)?;
        let digits = pattern.modifiers.iter().any(|m| m == "inclusive")
            && matches!(
                self.range_bounds(pattern, &children),
                (Some(start), Some(end)) if self.byte_bound(start) == Some(b'0') && self.byte_bound(end) == Some(b'9')
            );

        Ok(match pattern.original_kind.as_str() {
            "pat_slice" => self.prefix_status(pattern, &children)?,
            "pat_binding" | "pat_binding_pattern" => {
                self.binding_status(pattern, &children, rest)?
            }
            "pat_range" if digits => self.digit_status(pattern, &children),
            "pat_wild" if rest => CaptureStatus::Captured,
            _ => unsupported("Pattern is outside bounded digit-prefix capture grammar"),
        })
    }

    fn digit_status(&self, pattern: &Node, bounds: &[&Node]) -> CaptureStatus {
        if !self.reader.integer(pattern.ty.as_deref(), "u8") {
            return unsupported("Digit range does not have compiler byte type");
        }
        for bound in bounds {
            if bound.ty.is_none() {
                return CaptureStatus::Unknown("Missing compiler range endpoint type".into());
            }
            if !self.reader.integer(bound.ty.as_deref(), "u8")
                || bound.provenance.origin != Origin::Source
            {
                return unsupported("Outside original compiler byte endpoint contract");
            }
        }
        CaptureStatus::Captured
    }

    fn prefix_status(
        &self,
        pattern: &'a Node,
        children: &[&'a Node],
    ) -> Result<CaptureStatus, String> {
        let middle = children.iter().position(|child| is_rest(child));
        let (prefix, middle, suffix) = match middle {
            Some(index) => (
                &children[..index],
                Some(children[index]),
                &children[index + 1..],
            ),
            None => (children, None, &[][..]),
        };
        let binding = |node: &Node| {
            matches!(
                node.original_kind.as_str(),
                "pat_binding" | "pat_binding_pattern"
            )
        };
        if !(1..=3).contains(&prefix.len())
            || !suffix.is_empty()
            || !self.shared_slice_adjustment(pattern)
            || !prefix.iter().all(|child| binding(child))
        {
            return Ok(unsupported(
                "Outside shared byte-slice prefix capture contract",
            ));
        }
        let Some(middle) = middle else {
            return Ok(unsupported("Missing named rest"));
        };
        if !binding(middle) {
            return Ok(unsupported("Trailing rest requires a named binding"));
        }
        for (child, rest) in prefix
            .iter()
            .map(|child| (*child, false))
            .chain([(middle, true)])
        {
            let status = self.slice_status(child, rest)?;
            if status != CaptureStatus::Captured {
                return Ok(status);
            }
        }

        Ok(CaptureStatus::Captured)
    }

    fn shared_slice_adjustment(&self, pattern: &Node) -> bool {
        let Some(value) = pattern.ty.as_deref() else {
            return false;
        };
        let Some([adjustment]) = pattern.adjustments.as_deref() else {
            return false;
        };
        let slice = self.reader.ty(value).is_ok_and(|ty| {
            ty.kind == TypeKind::Sequence
                && ty.fixed != Some(true)
                && self.reader.integer(ty.element.as_deref(), "u8")
        });
        let source = adjustment
            .target
            .as_deref()
            .and_then(|target| self.reader.ty(target).ok())
            .is_some_and(|ty| {
                ty.kind == TypeKind::Reference
                    && ty.mode.as_deref() == Some("shared")
                    && ty.target.as_deref() == Some(value)
            });

        slice && adjustment.kind == "auto_deref" && source
    }

    fn binding_status(
        &self,
        pattern: &'a Node,
        children: &[&'a Node],
        rest: bool,
    ) -> Result<CaptureStatus, String> {
        let Some(subpattern) = children.first().copied() else {
            return Ok(unsupported("Missing @ subpattern"));
        };
        let Some(effective) = pattern.binding_mode.as_deref() else {
            return Ok(CaptureStatus::Unknown(
                "Missing effective compiler binding mode".into(),
            ));
        };
        let mutable = self.declared(pattern).is_some_and(|(_, mutable)| mutable);
        if self.written_mode(pattern) != BindingMode::Value
            || mutable
            || effective_mode(effective) != BindingMode::Shared
            || !self.shared_binding_type(pattern, rest)
        {
            return Ok(unsupported(
                "Binding is outside immutable shared-byte capture contract",
            ));
        }
        if rest && subpattern.original_kind != "pat_wild"
            || !rest && subpattern.original_kind != "pat_range"
        {
            return Ok(unsupported("Expected digit range or named trailing rest"));
        }

        self.slice_status(subpattern, rest)
    }

    fn shared_binding_type(&self, pattern: &Node, rest: bool) -> bool {
        let Some(reference) = pattern.ty.as_deref().and_then(|ty| self.reader.ty(ty).ok()) else {
            return false;
        };
        if reference.kind != TypeKind::Reference || reference.mode.as_deref() != Some("shared") {
            return false;
        }
        let inner = reference.target.as_deref();
        match rest {
            true => inner
                .and_then(|inner| self.reader.ty(inner).ok())
                .is_some_and(|ty| {
                    ty.kind == TypeKind::Sequence
                        && ty.fixed != Some(true)
                        && self.reader.integer(ty.element.as_deref(), "u8")
                }),
            false => self.reader.integer(inner, "u8"),
        }
    }

    fn slice_bound(&self, bound: &'a Node) -> Result<HirNode, String> {
        let mut node = self.node(bound, "pat_slice_bound")?;
        node.ty = self.ty(bound)?;
        node.expansion = self.expansion(bound, node.span);
        let (details, status) = match bound.original_kind.as_str() {
            "pat_expr_lit" => match self.byte_literal(bound) {
                Some((value, negated)) => (
                    Details::ByteLiteral { value, negated },
                    CaptureStatus::Captured,
                ),
                None => unsupported_bound("Non-byte literal"),
            },
            _ => (
                Details::Path {
                    resolution: self.bound_path(bound),
                },
                unsupported("Path range endpoint"),
            ),
        };
        let status = if node.ty.is_none() {
            CaptureStatus::Unknown("Missing compiler range endpoint type".into())
        } else if node.expansion.is_some() {
            unsupported("Expanded range endpoint")
        } else {
            status
        };
        let observation = Observation {
            identity: IDENTITY.into(),
            hir: hir_ref(&bound.id),
            span: node.span,
            ty: self.shape(bound.ty.as_deref()),
            adjustments: Vec::new(),
            details,
            status,
        };
        node.res = observation.resolution().ok();

        Ok(node)
    }

    /// A range bound path, as the frontend records the resolution it does not support.
    fn bound_path(&self, bound: &Node) -> PathRes {
        let resolution = self.path_resolution(bound);
        PathRes {
            kind: "unsupported_pattern_path".into(),
            target: resolution
                .as_ref()
                .and_then(|resolution| resolution.target.clone()),
            description: resolution
                .map(|resolution| resolution.description)
                .unwrap_or_else(|| "Err".into()),
        }
    }

    fn byte_literal(&self, bound: &Node) -> Option<(u8, bool)> {
        let text = self.snippet(self.written(bound));
        let (negated, literal) = match text.strip_prefix('-') {
            Some(literal) => (true, literal.trim_start()),
            None => (false, text.as_str()),
        };
        let literal = syn::parse_str::<syn::LitByte>(literal).ok()?;
        Some((literal.value(), negated))
    }

    fn byte_bound(&self, bound: &Node) -> Option<u8> {
        if bound.original_kind != "pat_expr_lit" {
            return None;
        }
        match self.byte_literal(bound)? {
            (value, false) => Some(value),
            _ => None,
        }
    }

    fn shape(&self, ty: Option<&str>) -> TypeShape {
        let Some(ty) = ty.and_then(|ty| self.reader.ty(ty).ok()) else {
            return TypeShape::Missing;
        };
        let inner = |id: Option<&String>| Box::new(self.shape(id.map(String::as_str)));
        match ty.kind {
            TypeKind::Integer if ty.spelling == "u8" => TypeShape::Byte,
            TypeKind::Sequence if ty.fixed != Some(true) => {
                TypeShape::Slice(inner(ty.element.as_ref()))
            }
            TypeKind::Reference if ty.mode.as_deref() == Some("shared") => {
                TypeShape::Shared(inner(ty.target.as_ref()))
            }
            TypeKind::Reference if ty.mode.as_deref() == Some("exclusive") => {
                TypeShape::Mutable(inner(ty.target.as_ref()))
            }
            _ => TypeShape::Other(ty.spelling.clone()),
        }
    }
}

fn is_rest(node: &Node) -> bool {
    node.modifiers.iter().any(|modifier| modifier == "rest")
}

fn effective_mode(mode: &str) -> BindingMode {
    match mode {
        "shared" => BindingMode::Shared,
        "exclusive" => BindingMode::Mutable,
        _ => BindingMode::Value,
    }
}

fn compiler_kind(kind: &str) -> &'static str {
    match kind {
        "pat_wild" => "Wild",
        "pat_struct" => "Struct",
        "pat_tuple_struct" => "TupleStruct",
        "pat_or" => "Or",
        "pat_never" => "Never",
        "pat_tuple" => "Tuple",
        "pat_deref" => "Deref",
        "pat_ref" => "Ref",
        "pat_literal" | "pat_path" => "Expr",
        "pat_guard" => "Guard",
        "pat_range" => "Range",
        "pat_slice" => "Slice",
        "pat_binding" | "pat_binding_pattern" => "Binding",
        _ => "Missing",
    }
}

fn unsupported(reason: &str) -> CaptureStatus {
    CaptureStatus::Unsupported(reason.into())
}

fn unsupported_bound(reason: &str) -> (Details, CaptureStatus) {
    (
        Details::Unsupported {
            compiler_kind: reason.into(),
        },
        unsupported(reason),
    )
}
