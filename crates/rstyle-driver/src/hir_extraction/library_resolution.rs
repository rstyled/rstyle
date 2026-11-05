//! Compiler-resolved core identities under the supported local toolchain contract.

use rstyle_pdg::data_library::*;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

/// One exact compiler definition identity, without granting operation support.
pub(super) fn definition(tcx: TyCtxt<'_>, definition: DefId) -> Callee {
    Callee {
        path: tcx.def_path_str(definition),
        crate_name: tcx.crate_name(definition.krate).to_string(),
        crate_hash: tcx.crate_hash(definition.krate).to_string(),
        definition_hash: format!("{:?}", tcx.def_path_hash(definition)),
        implementation: None,
    }
}
