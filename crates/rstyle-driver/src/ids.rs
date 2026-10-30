//! Stable ids and workspace membership for definitions.

use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt};
use std::path::Path;

/// `<crate>::<def path>`, e.g. `rstyle_core::rules::fn_length` or
/// `rstyle_core::config::{impl#3}::default`. Unlike `def_path_str` it does not
/// depend on re-exports or visibility, so caller and callee always agree.
pub fn def_id_str(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    let krate = tcx.crate_name(def_id.krate);
    format!(
        "{krate}{}",
        tcx.def_path(def_id).to_string_no_crate_verbose()
    )
}

/// True when the definition's source lives under the workspace root
/// (a member crate, not a registry or std dependency).
pub fn in_workspace(tcx: TyCtxt<'_>, def_id: DefId, root: &Path) -> bool {
    if def_id.is_local() {
        return true;
    }

    let source_map = tcx.sess.source_map();
    let span = tcx.def_span(def_id);
    let file = source_map.lookup_source_file(span.source_callsite().lo());
    crate::extract::source_path(tcx, &file, root).is_some()
}

/// Reuse rustc's instance selection in the captured owner's compilation context.
pub(crate) fn selected_instance<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    callee: DefId,
    args: GenericArgsRef<'tcx>,
) -> Option<ty::Instance<'tcx>> {
    let environment = ty::TypingEnv::post_analysis(tcx, owner);
    ty::Instance::try_resolve(tcx, environment, callee, args)
        .ok()
        .flatten()
}

/// Dependence graphs classify external calls by the original definition file.
pub fn graph_source_in_workspace(tcx: TyCtxt<'_>, def_id: DefId, root: &Path) -> bool {
    let file = tcx
        .sess
        .source_map()
        .lookup_source_file(tcx.def_span(def_id).lo());
    crate::extract::display_path(&file, root).is_some()
}
