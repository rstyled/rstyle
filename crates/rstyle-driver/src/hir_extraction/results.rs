//! Concrete core Result observations; semantic permission remains independent.
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
