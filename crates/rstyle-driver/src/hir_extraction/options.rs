//! Observational core Option identities and patterns; no construction admission.
use super::hir::Context;
use rustc_hir::{self as hir};
use rustc_middle::ty::{self, Ty, TyCtxt};

pub(super) fn payload<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(definition, arguments) = value.kind() else {
        return None;
    };
    if !tcx.is_diagnostic_item(rustc_span::sym::Option, definition.did()) {
        return None;
    }
    let payload = arguments.type_at(0);
    (scalar(payload) || shared_scalar(payload)).then_some(payload)
}

pub(super) fn product_payload<'tcx>(ctx: &Context<'tcx>, value: Ty<'tcx>) -> Option<Ty<'tcx>> {
    if !ctx.profile.supports("rust.concrete-option-propagation") {
        return None;
    }
    let ty::Adt(definition, arguments) = value.kind() else {
        return None;
    };
    if !ctx
        .tcx
        .is_diagnostic_item(rustc_span::sym::Option, definition.did())
    {
        return None;
    }

    let payload = arguments.type_at(0);
    matches!(payload.kind(), ty::Tuple(_) | ty::Adt(_, _)).then_some(payload)
}

pub(super) fn borrowed_slice_payload<'tcx>(
    ctx: &Context<'tcx>,
    value: Ty<'tcx>,
) -> Option<Ty<'tcx>> {
    if !ctx.profile.supports("rust.checked-borrowed-result-flow") {
        return None;
    }
    let ty::Adt(definition, arguments) = value.kind() else {
        return None;
    };
    if arguments.len() != 1
        || !ctx
            .tcx
            .is_diagnostic_item(rustc_span::sym::Option, definition.did())
    {
        return None;
    }

    let payload = arguments.type_at(0);
    matches!(payload.kind(), ty::Ref(_, inner, hir::Mutability::Not) if matches!(inner.kind(), ty::Slice(element) if *element == ctx.tcx.types.u8)).then_some(payload)
}

fn scalar(value: Ty<'_>) -> bool {
    matches!(value.kind(), ty::Bool | ty::Int(_) | ty::Uint(_))
}

fn shared_scalar(value: Ty<'_>) -> bool {
    matches!(value.kind(), ty::Ref(_, inner, hir::Mutability::Not) if scalar(*inner))
}
