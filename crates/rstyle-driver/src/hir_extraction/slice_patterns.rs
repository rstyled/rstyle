//! Faithful bounded slice-pattern capture, separate from semantic activation.
mod children;
mod observe;
mod status;

use super::hir::{Context, resolved, symbol};
use rstyle_pdg::{frontend::*, slice_patterns::*};
use rustc_hir::{self as hir, PatKind};

impl<'tcx> Context<'tcx> {
    pub(super) fn slice_prefix_pattern(&self, pattern: &hir::Pat<'tcx>) -> Option<HirNode> {
        if !self.profile.supports(CAPABILITY) || !matches!(pattern.kind, PatKind::Slice(..)) {
            return None;
        }
        Some(self.record_slice_pattern(pattern, false))
    }

    fn record_slice_pattern(&self, pattern: &hir::Pat<'tcx>, rest: bool) -> HirNode {
        let kind = match pattern.kind {
            PatKind::Slice(..) => "pat_slice_prefix",
            PatKind::Binding(..) => "pat_binding_at",
            PatKind::Range(..) => "pat_slice_range",
            PatKind::Wild if rest => "pat_slice_rest",
            _ => "pat_slice_unsupported",
        };
        let (binding, binding_mut) = match pattern.kind {
            PatKind::Binding(mode, canonical, _, _) => {
                (Some(symbol(canonical)), Some(mode.1.is_mut()))
            }
            _ => (None, None),
        };
        let observation = self.slice_observation(pattern, rest);

        HirNode {
            ty: self
                .typeck
                .node_type_opt(pattern.hir_id)
                .map(|ty| resolved(ty, String::new())),
            binding,
            binding_mut,
            expansion: pattern.span.from_expansion().then(|| ExpansionSite {
                callsite: self.span(pattern.span.source_callsite()),
                snippet: self.snippet(pattern.span.source_callsite()),
            }),
            children: self.slice_pattern_children(pattern, rest),
            res: observation.resolution().ok(),
            ..self.node(pattern.hir_id, pattern.span, kind)
        }
    }
}
