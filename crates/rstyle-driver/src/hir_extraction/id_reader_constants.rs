//! Capture the actual generated SIZE -> original wrapper size_of chain.
mod chain;
mod identity;

use super::hir::Context;
use rstyle_pdg::{
    PathRes,
    id_reader_constants::{self as contract, Annotation, Observation},
};
use rustc_hir::{ConstArgKind, ExprKind, TyKind, def::Res};

impl Context<'_> {
    pub(super) fn id_reader_constant_resolution(&self, resolution: Res) -> Option<PathRes> {
        if !self.profile.supports(contract::CAPABILITY) {
            return None;
        }
        PathRes::observation(contract::RESOLUTION, &chain::capture(self, resolution)?).ok()
    }

    pub(super) fn id_reader_annotation(&self, annotation: &rustc_hir::Ty<'_>) -> Option<PathRes> {
        if !self.profile.supports(contract::CAPABILITY) || annotation.span.from_expansion() {
            return None;
        }
        let TyKind::Array(_, length) = annotation.kind else {
            return None;
        };
        let ConstArgKind::Anon(constant) = length.kind else {
            return None;
        };
        let expression = self.tcx.hir_body(constant.body).value;
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };

        let typeck = self.tcx.typeck(constant.def_id);
        let resolution = typeck.qpath_res(path, expression.hir_id);
        let constant: Observation = chain::capture(self, resolution)?;
        PathRes::observation(
            contract::ANNOTATION_RESOLUTION,
            &Annotation {
                span: self.span(annotation.span),
                length: self.span(expression.span),
                constant,
            },
        )
        .ok()
    }
}
