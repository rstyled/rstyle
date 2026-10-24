//! Distinct scalar-instance observations; checked links must supply authority.
use super::{select, types};
use crate::hir_extraction::hir::{Context, resolved};
use rstyle_pdg::{Dispatch, PathRes, extraction_profile::ExtractionProfile};
use rustc_hir::{Expr, ExprKind};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(in crate::hir_extraction) fn scalar_instance_call(
        &self,
        expression: &Expr<'tcx>,
    ) -> Option<(PathRes, Dispatch)> {
        if self.profile != ExtractionProfile::DataV3 || expression.span.from_expansion() {
            return None;
        }

        let ExprKind::Call(callee, operands) = expression.kind else {
            return None;
        };
        let ty::FnDef(definition, arguments) = self.typeck.expr_ty(callee).kind() else {
            return None;
        };
        let arguments = arguments.skip_binder();
        if definition.is_local()
            || arguments.len() != 1
            || !super::publication::candidate(self.tcx, *definition)
        {
            return None;
        }

        let environment = ty::TypingEnv::post_analysis(self.tcx, expression.hir_id.owner.def_id);
        let (arguments, selected) = select(self.tcx, environment, *definition, arguments).ok()?;
        if selected.def_id() != *definition {
            return None;
        }
        let resolution = self.scalar_instance_resolution(*definition, arguments, selected)?;
        let dispatch = Dispatch {
            kind: "resolved_scalar_instance".into(),
            op: self.tcx.def_path_str(*definition),
            operand_tys: operands
                .iter()
                .map(|operand| resolved(self.typeck.expr_ty(operand), String::new()).normalized)
                .collect::<Option<_>>()?,
            result_ty: resolved(self.typeck.expr_ty(expression), String::new()).normalized?,
        };
        Some((resolution, dispatch))
    }

    fn scalar_instance_resolution(
        &self,
        definition: rustc_hir::def_id::DefId,
        arguments: ty::GenericArgsRef<'tcx>,
        selected: ty::Instance<'tcx>,
    ) -> Option<PathRes> {
        let artifact =
            super::super::crate_association::scalar_dependency(self.tcx, definition, arguments)?;
        let mut resolution =
            self.helper_callee(definition, "resolved_scalar_instance_callee", None);
        let mut value: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
        value["compiled_dependency"] = serde_json::to_value(artifact).ok()?;
        value["requested_arguments"] =
            serde_json::to_value(types::arguments(self.tcx, arguments)).ok()?;
        value["selected_arguments"] =
            serde_json::to_value(types::arguments(self.tcx, selected.args)).ok()?;

        resolution.description = serde_json::to_string(&value).ok()?;
        Some(resolution)
    }
}
