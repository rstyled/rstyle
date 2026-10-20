//! Positive builtin Copy observations attached to existing declaration ownership.
use super::super::hir::Context;
use rstyle_pdg::{data_library::Callee, declaration_origins::BuiltinCopy};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

pub(super) fn observe(ctx: &Context<'_>, owner: DefId) -> Option<BuiltinCopy> {
    if ctx.profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 {
        return None;
    }
    let tcx = ctx.tcx;
    let copy = tcx.lang_items().copy_trait()?;
    let self_ty = tcx.type_of(owner).instantiate_identity().skip_norm_wip();
    let mut implementations = tcx
        .non_blanket_impls_for_ty(copy, self_ty)
        .filter(|id| id.is_local());
    let implementation = implementations.next()?;
    if implementations.next().is_some() || !tcx.is_automatically_derived(implementation) {
        return None;
    }
    let expansion = tcx.def_span(implementation).ctxt().outer_expn_data();
    let definition = expansion.macro_def_id?;
    if definition.is_local()
        || tcx.crate_name(definition.krate).as_str() != "core"
        || tcx.item_name(definition).as_str() != "Copy"
        || !matches!(expansion.kind, rustc_span::hygiene::ExpnKind::Macro(rustc_span::hygiene::MacroKind::Derive, name) if name.as_str() == "Copy")
    {
        return None;
    }
    let (_, derive_span) = ctx
        .sources
        .range(tcx, expansion.call_site.source_callsite())?;

    Some(BuiltinCopy {
        trait_definition: identity(tcx, copy),
        derive_definition: identity(tcx, definition),
        derive_span,
    })
}

fn identity(tcx: TyCtxt<'_>, definition: DefId) -> Callee {
    Callee {
        path: tcx.def_path_str(definition),
        crate_name: tcx.crate_name(definition.krate).to_string(),
        crate_hash: tcx.crate_hash(definition.krate).to_string(),
        definition_hash: format!("{:?}", tcx.def_path_hash(definition)),
        implementation: None,
    }
}
