//! Observe the exact reader propagation; source, ownership and origins stay unproved.
use super::*;
use rstyle_pdg::named_error_result_propagation::capture::Shape;

impl<'tcx> Context<'tcx> {
    pub(super) fn named_error_result_shape(
        &self,
        operand: &Expr<'tcx>,
        branch: &Expr<'tcx>,
    ) -> Option<Shape> {
        let input = self.typeck.expr_ty(operand);
        let (payload, error) = payloads(self.tcx, input)?;
        let (output_payload, output_error) = payloads(self.tcx, self.result)?;
        if !payload.is_unit() || error != output_error {
            return None;
        }
        let ty::Tuple(values) = output_payload.kind() else {
            return None;
        };
        if values.as_slice() != [self.tcx.types.u32, self.tcx.types.usize] {
            return None;
        }
        // This observes nominal shape only. Exact complete source and no-drop
        // authority are mandatory in the independent reader verifier.
        let ty::Adt(owner, arguments) = error.kind() else {
            return None;
        };
        if !owner.is_struct() || !owner.did().is_local() || !arguments.is_empty() {
            return None;
        }
        let control_flow = self.typeck.expr_ty(branch);
        let residual = self.checked_result_residual(control_flow, self.tcx.types.unit, error)?;

        Some(Shape {
            input: input.to_string(),
            output: self.result.to_string(),
            payload: payload.to_string(),
            output_payload: output_payload.to_string(),
            error: error.to_string(),
            residual: residual.to_string(),
            control_flow: control_flow.to_string(),
        })
    }
}
