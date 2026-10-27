//! Compiler observations for exact local shared-byte-view/scalar tuple construction.
mod record;
mod selected;

use super::{
    hir::{Context, reference},
    instances::definition_record,
};
use rstyle_pdg::{
    frontend::PathRes,
    scalar_product_instances::{Argument, constructors::*},
};
use rustc_hir::{
    self as hir, ExprKind,
    def::{CtorKind, CtorOf, DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty::{self, GenericArgsRef};

impl<'tcx> Context<'tcx> {
    pub(super) fn scalar_product_constructor(
        &self,
        expression: &hir::Expr<'tcx>,
    ) -> Option<PathRes> {
        if !self.profile.supports("rust.slice-prefix-product-results") {
            return None;
        }
        let ExprKind::Call(callee, values) = expression.kind else {
            return None;
        };
        let [first, second] = values else {
            return None;
        };
        let selected = selected::Constructor::capture(self, callee)?;

        let actual = [expression, first, second].map(|value| {
            self.tcx
                .erase_and_anonymize_regions(self.typeck.expr_ty(value))
        });
        let expected = [
            selected.signature.output(),
            selected.signature.inputs()[0],
            selected.signature.inputs()[1],
        ];
        if actual != expected || expression.span.from_expansion() || callee.span.from_expansion() {
            return None;
        }
        self.record_scalar_constructor(expression, &selected)?
            .resolution()
            .ok()
    }
}
