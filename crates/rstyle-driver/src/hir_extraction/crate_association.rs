//! Compiler observations only; Cargo artifact association grants semantic authority separately.
use rstyle_pdg::extraction_profile::ExtractionProfile;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::ty::TyCtxt;
use serde::Serialize;
use std::path::PathBuf;

const IDENTITY: &str = "rstyle.rust.compiled-crate/1";

#[derive(Serialize)]
pub(super) struct CompiledCrate {
    identity: &'static str,
    crate_name: String,
    stable_crate_id: String,
}

#[derive(Serialize)]
pub(super) struct Dependency {
    #[serde(flatten)]
    compiled: CompiledCrate,
    crate_hash: String,
    rmeta: PathBuf,
    instance: EmptyInstance,
}

#[derive(Serialize)]
struct EmptyInstance {
    type_arguments: [String; 0],
    const_arguments: [String; 0],
}

pub(super) fn enabled(profile: ExtractionProfile) -> bool {
    profile == ExtractionProfile::DataV3 && profile.supports("rust.verified-helper-calls")
}

pub(super) fn local(tcx: TyCtxt<'_>) -> CompiledCrate {
    observed(tcx, LOCAL_CRATE)
}

pub(super) fn dependency(tcx: TyCtxt<'_>, callee: DefId) -> Option<Dependency> {
    if callee.is_local() || tcx.generics_of(callee).count() != 0 {
        return None;
    }
    let rmeta = tcx.used_crate_source(callee.krate).rmeta.clone()?;
    Some(Dependency {
        compiled: observed(tcx, callee.krate),
        crate_hash: tcx.crate_hash(callee.krate).to_string(),
        rmeta,
        instance: EmptyInstance {
            type_arguments: [],
            const_arguments: [],
        },
    })
}

fn observed(tcx: TyCtxt<'_>, krate: rustc_hir::def_id::CrateNum) -> CompiledCrate {
    CompiledCrate {
        identity: IDENTITY,
        crate_name: tcx.crate_name(krate).to_string(),
        stable_crate_id: format!("{:?}", tcx.stable_crate_id(krate)),
    }
}
