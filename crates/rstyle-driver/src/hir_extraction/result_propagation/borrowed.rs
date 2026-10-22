//! Observe differing success payloads without asserting move or origin permission.
use super::*;
use rstyle_pdg::borrowed_result_propagation::capture::Shape;

impl<'tcx> Context<'tcx> {
    pub(super) fn borrowed_result_shape(
        &self,
        operand: &Expr<'tcx>,
        branch: &Expr<'tcx>,
    ) -> Option<Shape> {
        let input = self.typeck.expr_ty(operand);
        let (payload, error) = payloads(self.tcx, input)?;
        let (output_payload, output_error) = payloads(self.tcx, self.result)?;
        if error != output_error || !self.borrowed_byte_slice(payload) {
            return None;
        }
        let ty::Tuple(values) = output_payload.kind() else {
            return None;
        };
        let [size, rest] = values.as_slice() else {
            return None;
        };
        if *size != self.tcx.types.usize || !self.borrowed_byte_slice(*rest) {
            return None;
        }
        let control_flow = self.typeck.expr_ty(branch);
        let residual = self.borrowed_result_residual(control_flow, payload, error)?;

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

    fn borrowed_byte_slice(&self, value: Ty<'tcx>) -> bool {
        matches!(value.kind(), ty::Ref(_, inner, hir::Mutability::Not)
            if matches!(inner.kind(), ty::Slice(element) if *element == self.tcx.types.u8))
    }

    fn borrowed_result_residual(
        &self,
        control_flow: Ty<'tcx>,
        payload: Ty<'tcx>,
        error: Ty<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let ty::Adt(owner, arguments) = control_flow.kind() else {
            return None;
        };
        let lang = self.tcx.lang_items();
        if arguments.len() != 2
            || arguments.type_at(1) != payload
            || self.tcx.parent(lang.cf_break_variant()?) != owner.did()
            || self.tcx.parent(lang.cf_continue_variant()?) != owner.did()
        {
            return None;
        }

        let residual = arguments.type_at(0);
        let (never, same_error) = payloads(self.tcx, residual)?;
        (never.is_never() && same_error == error).then_some(residual)
    }
}
