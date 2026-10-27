//! Record types, effective modes and real identities without assigning semantics.
use super::*;
use crate::hir_extraction::hir::reference;
use rustc_middle::ty::{self, adjustment::PatAdjust};

impl<'tcx> Context<'tcx> {
    pub(super) fn slice_observation(&self, pattern: &hir::Pat<'tcx>, rest: bool) -> Observation {
        Observation {
            identity: IDENTITY.into(),
            hir: reference(pattern.hir_id),
            span: self.span(pattern.span),
            status: self.slice_pattern_status(pattern, rest),
            ty: shape(self.typeck.node_type_opt(pattern.hir_id)),
            adjustments: self.slice_adjustments(pattern.hir_id),
            details: self.slice_details(pattern, rest),
        }
    }

    fn slice_adjustments(&self, id: hir::HirId) -> Vec<Adjustment> {
        self.typeck
            .pat_adjustments()
            .get(id)
            .into_iter()
            .flatten()
            .map(|adjustment| Adjustment {
                kind: match adjustment.kind {
                    PatAdjust::BuiltinDeref => AdjustmentKind::BuiltinDeref,
                    PatAdjust::OverloadedDeref => AdjustmentKind::OverloadedDeref,
                    PatAdjust::PinDeref => AdjustmentKind::PinDeref,
                },
                source: shape(Some(adjustment.source)),
            })
            .collect()
    }

    fn slice_details(&self, pattern: &hir::Pat<'tcx>, rest: bool) -> Details {
        match pattern.kind {
            PatKind::Slice(prefix, middle, suffix) => Details::Slice {
                prefix: prefix.iter().map(|pat| reference(pat.hir_id)).collect(),
                rest: middle.map(|pat| reference(pat.hir_id)),
                suffix: suffix.iter().map(|pat| reference(pat.hir_id)).collect(),
            },
            PatKind::Binding(_, _, _, subpattern) => Details::Binding {
                binding: self.slice_binding(pattern),
                subpattern: subpattern.map(|pat| reference(pat.hir_id)),
            },
            PatKind::Range(start, end, ending) => Details::Range {
                start: start.map(|bound| reference(bound.hir_id)),
                end: end.map(|bound| reference(bound.hir_id)),
                inclusive: matches!(ending, hir::RangeEnd::Included),
            },
            PatKind::Wild if rest => Details::Rest,
            _ => Details::Unsupported {
                compiler_kind: compiler_kind(pattern.kind).into(),
            },
        }
    }

    fn slice_binding(&self, pattern: &hir::Pat<'tcx>) -> Binding {
        let PatKind::Binding(written, canonical, name, _) = pattern.kind else {
            unreachable!("Called only for compiler binding patterns");
        };
        let effective = self.typeck.pat_binding_modes().get(pattern.hir_id).copied();
        Binding {
            canonical: reference(canonical),
            name: name.name.to_string(),
            identifier_span: self.span(name.span),
            written_mode: binding_mode(written),
            written_mutable: written.1.is_mut(),
            effective_mode: effective.map(binding_mode),
            effective_mutable: effective.map(|mode| mode.1.is_mut()),
        }
    }
}

pub(super) fn binding_mode(mode: hir::BindingMode) -> BindingMode {
    match mode.0 {
        hir::ByRef::No => BindingMode::Value,
        hir::ByRef::Yes(hir::Pinnedness::Not, hir::Mutability::Not) => BindingMode::Shared,
        hir::ByRef::Yes(hir::Pinnedness::Not, hir::Mutability::Mut) => BindingMode::Mutable,
        hir::ByRef::Yes(hir::Pinnedness::Pinned, hir::Mutability::Not) => BindingMode::PinnedShared,
        hir::ByRef::Yes(hir::Pinnedness::Pinned, hir::Mutability::Mut) => {
            BindingMode::PinnedMutable
        }
    }
}

pub(super) fn shape(value: Option<ty::Ty<'_>>) -> TypeShape {
    match value.map(|value| (value, value.kind())) {
        None => TypeShape::Missing,
        Some((_, ty::Uint(ty::UintTy::U8))) => TypeShape::Byte,
        Some((_, ty::Slice(element))) => TypeShape::Slice(Box::new(shape(Some(*element)))),
        Some((_, ty::Ref(_, inner, hir::Mutability::Not))) => {
            TypeShape::Shared(Box::new(shape(Some(*inner))))
        }
        Some((_, ty::Ref(_, inner, hir::Mutability::Mut))) => {
            TypeShape::Mutable(Box::new(shape(Some(*inner))))
        }
        Some((value, _)) => TypeShape::Other(value.to_string()),
    }
}

fn compiler_kind(kind: PatKind<'_>) -> &'static str {
    match kind {
        PatKind::Missing => "Missing",
        PatKind::Wild => "Wild",
        PatKind::Binding(..) => "Binding",
        PatKind::Struct(..) => "Struct",
        PatKind::TupleStruct(..) => "TupleStruct",
        PatKind::Or(..) => "Or",
        PatKind::Never => "Never",
        PatKind::Tuple(..) => "Tuple",
        PatKind::Deref(..) => "Deref",
        PatKind::Ref(..) => "Ref",
        PatKind::Expr(..) => "Expr",
        PatKind::Guard(..) => "Guard",
        PatKind::Range(..) => "Range",
        PatKind::Slice(..) => "Slice",
        PatKind::Err(..) => "Err",
    }
}
