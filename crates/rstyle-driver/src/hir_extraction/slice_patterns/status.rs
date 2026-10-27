//! Bounded capture status is descriptive and grants no extraction permission.
use super::*;
use rustc_ast::LitKind;
use rustc_middle::ty::{self, adjustment::PatAdjust};

impl<'tcx> Context<'tcx> {
    pub(super) fn slice_pattern_status(
        &self,
        pattern: &hir::Pat<'tcx>,
        rest: bool,
    ) -> CaptureStatus {
        if self.typeck.node_type_opt(pattern.hir_id).is_none() {
            return CaptureStatus::Unknown("Missing compiler pattern type".into());
        }
        if pattern.span.from_expansion() {
            return unsupported("Expanded pattern source");
        }
        match pattern.kind {
            PatKind::Slice(..) => self.slice_prefix_status(pattern),
            PatKind::Binding(..) => self.slice_binding_status(pattern, rest),
            PatKind::Range(start, end, hir::RangeEnd::Included)
                if byte_bound(start) == Some(b'0') && byte_bound(end) == Some(b'9') =>
            {
                self.digit_range_status(pattern)
            }
            PatKind::Wild if rest => CaptureStatus::Captured,
            _ => unsupported("Pattern is outside bounded digit-prefix capture grammar"),
        }
    }

    fn digit_range_status(&self, pattern: &hir::Pat<'tcx>) -> CaptureStatus {
        let PatKind::Range(start, end, _) = pattern.kind else {
            unreachable!("Called only for compiler range patterns");
        };
        if self.typeck.node_type_opt(pattern.hir_id) != Some(self.tcx.types.u8) {
            return unsupported("Digit range does not have compiler byte type");
        }
        for bound in start.into_iter().chain(end) {
            let Some(value) = self.typeck.node_type_opt(bound.hir_id) else {
                return CaptureStatus::Unknown("Missing compiler range endpoint type".into());
            };
            if value != self.tcx.types.u8 || bound.span.from_expansion() {
                return unsupported("Outside original compiler byte endpoint contract");
            }
        }
        CaptureStatus::Captured
    }

    fn slice_prefix_status(&self, pattern: &hir::Pat<'tcx>) -> CaptureStatus {
        let PatKind::Slice(prefix, middle, suffix) = pattern.kind else {
            unreachable!("Called only for compiler slice patterns");
        };

        if !(1..=3).contains(&prefix.len())
            || !suffix.is_empty()
            || !self.shared_slice_adjustment(pattern)
            || !prefix
                .iter()
                .all(|pat| matches!(pat.kind, PatKind::Binding(..)))
        {
            return unsupported("Outside shared byte-slice prefix capture contract");
        }
        let Some(middle) = middle else {
            return unsupported("Missing named rest");
        };
        if !matches!(middle.kind, PatKind::Binding(..)) {
            return unsupported("Trailing rest requires a named binding");
        }

        prefix
            .iter()
            .map(|pat| self.slice_pattern_status(pat, false))
            .chain(std::iter::once(self.slice_pattern_status(middle, true)))
            .find(|status| *status != CaptureStatus::Captured)
            .unwrap_or(CaptureStatus::Captured)
    }

    fn shared_slice_adjustment(&self, pattern: &hir::Pat<'tcx>) -> bool {
        let Some(value) = self.typeck.node_type_opt(pattern.hir_id) else {
            return false;
        };
        let Some(adjustments) = self.typeck.pat_adjustments().get(pattern.hir_id) else {
            return false;
        };
        let [adjustment] = adjustments.as_slice() else {
            return false;
        };
        matches!(value.kind(), ty::Slice(element) if *element == self.tcx.types.u8)
            && adjustment.kind == PatAdjust::BuiltinDeref
            && matches!(adjustment.source.kind(),
                ty::Ref(_, inner, hir::Mutability::Not) if *inner == value)
    }

    fn slice_binding_status(&self, pattern: &hir::Pat<'tcx>, rest: bool) -> CaptureStatus {
        let PatKind::Binding(written, _, _, Some(subpattern)) = pattern.kind else {
            return unsupported("Missing @ subpattern");
        };
        let Some(effective) = self.typeck.pat_binding_modes().get(pattern.hir_id) else {
            return CaptureStatus::Unknown("Missing effective compiler binding mode".into());
        };

        if written.0 != hir::ByRef::No
            || written.1.is_mut()
            || super::observe::binding_mode(*effective) != BindingMode::Shared
            || effective.1.is_mut()
            || !self.shared_binding_type(pattern, rest)
        {
            return unsupported("Binding is outside immutable shared-byte capture contract");
        }
        if rest && !matches!(subpattern.kind, PatKind::Wild)
            || !rest && !matches!(subpattern.kind, PatKind::Range(..))
        {
            return unsupported("Expected digit range or named trailing rest");
        }

        self.slice_pattern_status(subpattern, rest)
    }

    fn shared_binding_type(&self, pattern: &hir::Pat<'tcx>, rest: bool) -> bool {
        let Some(value) = self.typeck.node_type_opt(pattern.hir_id) else {
            return false;
        };
        let ty::Ref(_, inner, hir::Mutability::Not) = value.kind() else {
            return false;
        };
        if rest {
            matches!(inner.kind(), ty::Slice(element) if *element == self.tcx.types.u8)
        } else {
            *inner == self.tcx.types.u8
        }
    }
}

fn byte_bound(bound: Option<&hir::PatExpr<'_>>) -> Option<u8> {
    match bound?.kind {
        hir::PatExprKind::Lit {
            lit,
            negated: false,
        } => match lit.node {
            LitKind::Byte(value) => Some(value),
            _ => None,
        },
        _ => None,
    }
}

fn unsupported(reason: &str) -> CaptureStatus {
    CaptureStatus::Unsupported(reason.into())
}
