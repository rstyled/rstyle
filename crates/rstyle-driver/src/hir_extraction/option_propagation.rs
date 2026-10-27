//! Exact Try-desugaring observations, without extraction permission.
use super::{hir::Context, options::payload};
use rstyle_pdg::{PathRes, option_propagation::capture::Shape};
use rustc_hir::{self as hir, Expr};
use rustc_middle::ty;

impl<'tcx> Context<'tcx> {
    pub(super) fn option_propagation(&self, root: &Expr<'tcx>) -> Option<PathRes> {
        self.propagation_capture(root, super::result_propagation::Protocol::Option)
    }

    pub(super) fn option_propagation_shape(
        &self,
        operand: &Expr<'tcx>,
        branch: &Expr<'tcx>,
    ) -> Option<Shape> {
        let input = self.typeck.expr_ty(operand);
        let value = payload(self.tcx, input)?;
        if !matches!(value.kind(), ty::Ref(_, scalar, hir::Mutability::Not) if *scalar == self.tcx.types.u8)
        {
            return None;
        }
        let ty::Adt(output, output_args) = self.result.kind() else {
            return None;
        };
        if !self
            .tcx
            .is_diagnostic_item(rustc_span::sym::Option, output.did())
        {
            return None;
        }
        let control_flow = self.typeck.expr_ty(branch);
        let ty::Adt(_, args) = control_flow.kind() else {
            return None;
        };
        if args.len() != 2 || args.type_at(1) != value {
            return None;
        }

        Some(Shape {
            input: input.to_string(),
            output: self.result.to_string(),
            payload: value.to_string(),
            output_payload: output_args.type_at(0).to_string(),
            residual: args.type_at(0).to_string(),
            control_flow: control_flow.to_string(),
        })
    }
}
