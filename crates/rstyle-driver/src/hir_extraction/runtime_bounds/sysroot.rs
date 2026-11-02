//! Standard-library identity by crate number, never by crate or path spelling.
use super::*;
use rustc_hir::def_id::{CrateNum, DefId};
use rustc_middle::ty::GenericArgKind;
use rustc_span::sym;

/// `core`, `alloc` or `std`, identified by the crates defining their lang and diagnostic items.
pub(super) fn sysroot(tcx: ty::TyCtxt<'_>, krate: CrateNum) -> bool {
    let anchors = [
        tcx.lang_items().sized_trait(),
        tcx.lang_items().owned_box(),
        tcx.get_diagnostic_item(sym::HashMap),
    ];
    anchors
        .into_iter()
        .flatten()
        .any(|item| item.krate == krate)
}

/// Built only from primitives and standard-library types.
pub(super) fn standard_type<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.walk().all(|part| match part.kind() {
        GenericArgKind::Type(ty) => standard_kind(tcx, ty),
        _ => true,
    })
}

fn standard_kind<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(definition, _) => sysroot(tcx, definition.did().krate),
        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_)
        | ty::Str
        | ty::Array(..)
        | ty::Slice(_)
        | ty::RawPtr(..)
        | ty::Ref(..)
        | ty::Never
        | ty::Tuple(_) => true,
        _ => false,
    }
}

/// A standard callee may run workspace code through callable arguments or trait bounds
/// on workspace types, such as `PartialEq::ne`, `Into::into` or `sort` on workspace items.
pub(super) fn calls_back<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    callee: DefId,
    arguments: ty::GenericArgsRef<'tcx>,
) -> bool {
    let callable = arguments.types().any(|ty| {
        ty.walk().any(|part| {
            part.as_type().is_some_and(|ty| {
                matches!(
                    ty.kind(),
                    ty::Closure(..)
                        | ty::FnDef(..)
                        | ty::FnPtr(..)
                        | ty::Dynamic(..)
                        | ty::Param(_)
                        | ty::Alias(..)
                        | ty::Coroutine(..)
                )
            })
        })
    });
    let sized = tcx.lang_items().sized_trait();
    callable
        || tcx
            .clauses_of(callee)
            .instantiate(tcx, arguments)
            .clauses
            .into_iter()
            .filter_map(|clause| clause.skip_norm_wip().as_trait_clause())
            .filter(|bound| Some(bound.def_id()) != sized)
            .any(|bound| !standard_type(tcx, bound.self_ty().skip_binder()))
}

/// Language-neutral key of a standard operation, matched against library contracts:
/// the defining impl's self type and the member name, as in `rust:BTreeMap.get`.
/// Never derived from module path spelling.
pub(super) fn operation(tcx: ty::TyCtxt<'_>, callee: DefId) -> Option<String> {
    let implementation = tcx.impl_of_assoc(callee)?;
    let self_ty = tcx
        .type_of(implementation)
        .instantiate_identity()
        .skip_norm_wip();
    let owner = type_key(tcx, self_ty)?;
    Some(format!("rust:{owner}.{}", tcx.item_name(callee)))
}

// A standard type by its compiler-registered name: diagnostic item, then lang item, or
// the primitive kind. Unnamed and generic self types have no key.
fn type_key(tcx: ty::TyCtxt<'_>, ty: Ty<'_>) -> Option<String> {
    match ty.kind() {
        ty::Adt(definition, _) => {
            let id = definition.did();
            tcx.get_diagnostic_name(id)
                .or_else(|| tcx.as_lang_item(id).map(|item| item.name()))
                .map(|name| name.to_string())
        }
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => Some(ty.to_string()),
        ty::Slice(_) => Some("slice".into()),
        ty::Str => Some("str".into()),
        _ => None,
    }
}

/// Whether dropping a value of `ty` may run a workspace `Drop` impl.
pub(super) fn workspace_drop<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.walk().any(|part| {
        part.as_type().is_some_and(|ty| match ty.kind() {
            ty::Adt(definition, _) => {
                !sysroot(tcx, definition.did().krate)
                    && tcx.adt_destructor(definition.did()).is_some()
            }
            _ => false,
        })
    })
}
