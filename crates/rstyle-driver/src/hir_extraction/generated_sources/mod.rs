//! Explicit diagnostic-only raw generated-source capture. No admission changes.
pub(super) mod observe;
mod provenance;
mod roots;

use provenance::{Provenance, definition_record, identity};
use roots::Roots;
use rstyle_pdg::generated_sources::capture::*;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::{ExpnId, Span};

/// The unique original local bang expansion a generated definition comes from.
pub(crate) fn generated_root(tcx: TyCtxt<'_>, id: LocalDefId) -> Option<String> {
    Roots::new(tcx).definition(id).ok().flatten().map(identity)
}

pub(super) fn declaration_origin(
    ctx: &super::hir::Context<'_>,
    id: DefId,
    span: Span,
) -> Result<(Definition, Origin, String), String> {
    let root = Roots::new(ctx.tcx)
        .definition(id.expect_local())?
        .ok_or("Generated declaration has no unique original local bang root")?;
    let mut provenance = Provenance::new(ctx.tcx, &ctx.sources);

    provenance.expansion(root)?;
    let origin = provenance.origin(span)?;
    Ok((definition_record(ctx.tcx, id), origin, identity(root)))
}

// Capturing a trait member supplies no source, cleanup or callable permission.
fn implementation(tcx: TyCtxt<'_>, id: LocalDefId) -> Option<DefId> {
    if let Some(implementation) = tcx.inherent_impl_of_assoc(id.to_def_id()) {
        return Some(implementation);
    }
    let method = tcx
        .opt_associated_item(id.to_def_id())?
        .trait_item_def_id()?;
    if tcx.get_diagnostic_item(rustc_span::Symbol::intern("try_from_fn")) != Some(method) {
        return None;
    }
    tcx.impl_of_assoc(id.to_def_id())
}
