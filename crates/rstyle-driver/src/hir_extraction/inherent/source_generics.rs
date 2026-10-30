//! Match source impl lifetime binders before publishing callable ownership.
use rustc_hir::{self as hir, def_id::DefId};
use rustc_middle::ty::TyCtxt;

pub(super) fn implementation(tcx: TyCtxt<'_>, owner: DefId) -> bool {
    let Some(implementation) = source_impl(tcx, owner) else {
        return false;
    };
    let parameters = implementation.generics.params;
    if !implementation.generics.predicates.is_empty()
        || !implementation.generics.where_clause_span.is_empty()
        || parameters.len() > rstyle_pdg::concrete_types::MAX_FIELDS
        || !parameters.iter().all(|parameter| explicit(tcx, parameter))
    {
        return false;
    }

    let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = implementation.self_ty.kind else {
        return false;
    };
    let Some((last, parents)) = path.segments.split_last() else {
        return false;
    };
    parents.iter().all(|segment| segment.args.is_none()) && arguments(last.args, parameters)
}

fn explicit(tcx: TyCtxt<'_>, parameter: &hir::GenericParam<'_>) -> bool {
    matches!(
        parameter.kind,
        hir::GenericParamKind::Lifetime {
            kind: hir::LifetimeParamKind::Explicit
        }
    ) && matches!(parameter.name, hir::ParamName::Plain(_))
        && tcx.hir_attrs(parameter.hir_id).is_empty()
}

fn arguments(args: Option<&hir::GenericArgs<'_>>, parameters: &[hir::GenericParam<'_>]) -> bool {
    let Some(args) = args else {
        return parameters.is_empty();
    };
    !parameters.is_empty()
        && args.constraints.is_empty()
        && args.parenthesized == hir::GenericArgsParentheses::No
        && args.args.len() == parameters.len()
        && args
            .args
            .iter()
            .zip(parameters)
            .all(|(argument, parameter)| {
                matches!(argument, hir::GenericArg::Lifetime(lifetime)
                    if lifetime.kind == hir::LifetimeKind::Param(parameter.def_id))
            })
}

/// Concrete type and const specializations retain their existing contracts.
pub(super) fn has_lifetime_arguments(tcx: TyCtxt<'_>, owner: DefId) -> bool {
    let Some(implementation) = source_impl(tcx, owner) else {
        return false;
    };
    let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = implementation.self_ty.kind else {
        return false;
    };
    path.segments
        .iter()
        .any(|segment| segment.args().has_lifetime_args())
}

fn source_impl<'tcx>(tcx: TyCtxt<'tcx>, owner: DefId) -> Option<&'tcx hir::Impl<'tcx>> {
    let hir::Node::Item(item) = tcx.hir_node_by_def_id(owner.expect_local()) else {
        return None;
    };
    let hir::ItemKind::Impl(implementation) = &item.kind else {
        return None;
    };
    Some(implementation)
}
