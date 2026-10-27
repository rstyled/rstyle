//! Observe the exact reader propagation; source, ownership and origins stay unproved.
use super::*;
use rstyle_pdg::named_error_result_propagation::{Mode, capture::Shape};

impl<'tcx> Context<'tcx> {
    pub(super) fn named_error_result_shape(
        &self,
        operand: &Expr<'tcx>,
        branch: &Expr<'tcx>,
        mode: Mode,
    ) -> Option<Shape> {
        let input = self.typeck.expr_ty(operand);
        let (payload, error) = payloads(self.tcx, input)?;
        let (output_payload, output_error) = payloads(self.tcx, self.result)?;
        if error != output_error || !self.named_error_payloads(mode, payload, output_payload) {
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
        let residual = self.checked_result_residual(control_flow, payload, error)?;

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

    // Observation only: source ownership, builtin Copy and no-drop are checked
    // independently after capture, never inferred from these scalar fields.
    fn reader_copy_product(&self, output: Ty<'tcx>) -> bool {
        let ty::Tuple(values) = output.kind() else {
            return false;
        };
        let [product, count] = values.as_slice() else {
            return false;
        };
        if *count != self.tcx.types.usize {
            return false;
        }

        let ty::Adt(owner, arguments) = product.kind() else {
            return false;
        };
        owner.is_struct()
            && owner.did().is_local()
            && arguments.is_empty()
            && owner.non_enum_variant().fields.len() == 3
            && owner
                .non_enum_variant()
                .fields
                .iter()
                .all(|field| field.ty(self.tcx, arguments).skip_norm_wip() == self.tcx.types.bool)
    }

    fn named_error_payloads(&self, mode: Mode, input: Ty<'tcx>, output: Ty<'tcx>) -> bool {
        let tuple = |value: Ty<'tcx>| {
            matches!(value.kind(), ty::Tuple(values)
            if values.as_slice() == [self.tcx.types.u32, self.tcx.types.usize])
        };
        match mode {
            Mode::UnitToReader => input.is_unit() && tuple(output),
            Mode::UnitToU16Reader => {
                input.is_unit()
                    && matches!(output.kind(), ty::Tuple(values)
                if values.as_slice() == [self.tcx.types.u16, self.tcx.types.usize])
            }
            Mode::ReaderToScalar => tuple(input) && output == self.tcx.types.usize,
            Mode::ReaderToCopyProduct => tuple(input) && self.reader_copy_product(output),
            Mode::IdToReader => {
                matches!(input.kind(), ty::Adt(owner, arguments)
                if owner.is_struct() && owner.did().is_local() && arguments.is_empty())
                    && matches!(output.kind(), ty::Tuple(values)
                    if values.as_slice() == [input, self.tcx.types.usize])
            }
        }
    }
}
