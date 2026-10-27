//! Shared diagnostic mechanics; callers retain their owner and publication scopes.
use rustc_hir::{self as hir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::TyCtxt;
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

pub(super) fn owners(tcx: TyCtxt<'_>, kinds: &[DefKind]) -> BTreeMap<String, Option<LocalDefId>> {
    let mut owners = BTreeMap::new();
    for id in tcx
        .hir_body_owners()
        .filter(|id| kinds.contains(&tcx.def_kind(*id)))
    {
        owners
            .entry(tcx.def_path_str(id.to_def_id()))
            .and_modify(|existing| *existing = None)
            .or_insert(Some(id));
    }
    owners
}

pub(in crate::hir_extraction) fn hir_id(
    reference: &rstyle_pdg::HirRef,
    error: &str,
) -> Result<hir::HirId, String> {
    if reference.krate != 0 {
        return Err(error.into());
    }
    Ok(hir::HirId {
        owner: hir::OwnerId {
            def_id: LocalDefId {
                local_def_index: hir::def_id::DefIndex::from_u32(reference.owner),
            },
        },
        local_id: hir::ItemLocalId::from_u32(reference.local),
    })
}

pub(super) fn persist(
    inventory: &impl Serialize,
    suffix: &str,
    directory_error: &str,
) -> Result<(), String> {
    let path = destination(suffix, directory_error)?;
    super::super::diagnostics::persist_file(&path, inventory).map_err(|error| error.to_string())
}

fn destination(suffix: &str, directory_error: &str) -> Result<PathBuf, String> {
    let directory =
        PathBuf::from(std::env::var_os("RSTYLE_DIAGNOSTIC_FACTS_DIR").ok_or(directory_error)?);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(format!("{}-{suffix}.json", std::process::id())))
}
