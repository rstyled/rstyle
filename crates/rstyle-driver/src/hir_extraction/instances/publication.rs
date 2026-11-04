//! Complete bounded instance observations attached to unchanged original owners.
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::{self, TyCtxt};

pub(super) fn candidate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    if tcx.def_kind(id) != DefKind::Fn {
        return false;
    }

    let generics = tcx.generics_of(id);
    if generics.parent.is_some()
        || generics.own_params.len() != 1
        || !matches!(
            generics.own_params[0].kind,
            ty::GenericParamDefKind::Type { .. }
        )
    {
        return false;
    }
    let signature = tcx.fn_sig(id).instantiate_identity().skip_binder();
    let same_parameter =
        |value: ty::Ty<'_>| matches!(value.kind(), ty::Param(parameter) if parameter.index == 0);
    !signature.inputs().is_empty()
        && signature
            .inputs()
            .iter()
            .all(|value| same_parameter(*value))
        && same_parameter(signature.output())
}
