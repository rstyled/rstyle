//! Compiler capture modes, projections, storage and closure signature remain observations.
use super::*;
use rstyle_pdg::{PathRes, id_error_maps::capture as records, option_callbacks::CaptureMode};
use rustc_middle::hir::place::ProjectionKind;

impl<'tcx> Context<'tcx> {
    pub(super) fn id_error_map_environment(
        &self,
        expression: &Expr<'tcx>,
        observe: &mut impl FnMut(Ty<'tcx>) -> Result<Observation, String>,
    ) -> Result<PathRes, String> {
        let ExprKind::Closure(closure) = expression.kind else {
            return Err("ID callback environment is not a compiler closure".into());
        };

        let ty::Closure(definition, arguments) = self.typeck.expr_ty(expression).kind() else {
            return Err("ID callback has no compiler closure type".into());
        };

        if *definition != closure.def_id.to_def_id() {
            return Err("ID callback compiler owner mismatch".into());
        }

        let captures = self.tcx.closure_captures(closure.def_id);

        if captures.len() > rstyle_pdg::concrete_types::MAX_FIELDS {
            return Err("ID callback environment exceeds capture budget".into());
        }

        let arguments = arguments.as_closure();

        let signature = arguments.sig().skip_binder();

        let captures = captures
            .iter()
            .map(|place| capture(place, observe))
            .collect::<Result<_, _>>()?;
        let stored_type = observe(arguments.tupled_upvars_ty())?;
        let (signature_inputs, signature_output) = signature_types(signature, observe)?;

        let environment = records::Environment {
            identity: records::IDENTITY.into(),
            closure: reference(expression.hir_id),
            body: reference(self.tcx.hir_body(closure.body).value.hir_id),
            definition_identity: format!("{:?}", self.tcx.def_path_hash(*definition)),
            captures,
            stored_type,
            signature_abi: format!("{:?}", signature.abi()),
            signature_inputs,
            signature_output,
        };

        Ok(PathRes {
            kind: records::KIND.into(),
            target: None,
            description: serde_json::to_string(&environment).map_err(|error| error.to_string())?,
        })
    }
}

fn capture<'tcx>(
    place: &ty::CapturedPlace<'tcx>,
    observe: &mut impl FnMut(Ty<'tcx>) -> Result<Observation, String>,
) -> Result<records::Capture, String> {
    let mode = match place.info.capture_kind {
        ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) => CaptureMode::Shared,
        ty::UpvarCapture::ByRef(ty::BorrowKind::UniqueImmutable) => CaptureMode::Unique,
        ty::UpvarCapture::ByRef(ty::BorrowKind::Mutable) => CaptureMode::Mutable,
        ty::UpvarCapture::ByValue => CaptureMode::Value,
        ty::UpvarCapture::ByUse => CaptureMode::Use,
    };

    let preceding_types = std::iter::once(place.place.base_ty).chain(
        place
            .place
            .projections
            .iter()
            .map(|projection| projection.ty),
    );

    let projections = preceding_types
        .zip(&place.place.projections)
        .map(|(before, projection)| {
            Ok(records::Projection {
                kind: if projection.kind == ProjectionKind::Deref {
                    "deref"
                } else {
                    "unsupported"
                }
                .into(),
                before: observe(before)?,
                after: observe(projection.ty)?,
            })
        })
        .collect::<Result<_, String>>()?;

    Ok(records::Capture {
        root: reference(place.get_root_variable()),
        mode,
        root_type: observe(place.place.base_ty)?,
        projections,
    })
}

fn signature_types<'tcx>(
    signature: ty::FnSig<'tcx>,
    observe: &mut impl FnMut(Ty<'tcx>) -> Result<Observation, String>,
) -> Result<(Vec<Observation>, Observation), String> {
    let inputs = signature
        .inputs()
        .iter()
        .map(|ty| observe(*ty))
        .collect::<Result<_, _>>()?;

    let output = observe(signature.output())?;

    Ok((inputs, output))
}
