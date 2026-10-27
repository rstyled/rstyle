//! Concrete core Result observations; semantic permission remains independent.
use super::hir::Context;
use rstyle_pdg::PathRes;
use rustc_hir::{
    self as hir,
    def::{DefKind, Res},
};
use rustc_middle::ty::{self, Ty, TyCtxt};

pub(super) fn payloads<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let ty::Adt(definition, arguments) = value.kind() else {
        return None;
    };
    if !tcx.is_diagnostic_item(rustc_span::sym::Result, definition.did()) || arguments.len() != 2 {
        return None;
    }
    Some((arguments.type_at(0), arguments.type_at(1)))
}

impl<'tcx> Context<'tcx> {
    pub(super) fn result_expression(&self, expression: &hir::Expr<'tcx>) -> Option<PathRes> {
        if !self.profile.supports("rust.fresh-concrete-results") {
            return None;
        }
        let hir::ExprKind::Call(callee, arguments) = expression.kind else {
            return None;
        };
        if arguments.len() != 1 {
            return None;
        }
        let hir::ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        payloads(self.tcx, self.typeck.expr_ty(expression))?;
        let Res::Def(DefKind::Ctor(..), definition) = self.typeck.qpath_res(path, callee.hir_id)
        else {
            return None;
        };
        let variant = self.tcx.parent(definition);
        if !self
            .tcx
            .is_diagnostic_item(rustc_span::sym::Result, self.tcx.parent(variant))
            || !matches!(self.tcx.item_name(variant).as_str(), "Ok" | "Err")
        {
            return None;
        }

        let callee = super::library_resolution::definition(self.tcx, definition);
        Some(PathRes {
            kind: "resolved_result_variant".into(),
            target: None,
            description: serde_json::to_string(&callee).expect("resolved Result variant"),
        })
    }
}
