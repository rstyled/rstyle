//! Runtime-bound observations for the advisory capture: each owner's program, as the
//! core lowers it from the crate's `compat.cpg` document.

use super::hir::Context;
use rstyle_core::advisory::runtime_bounds::{CAPTURE_ENV, Input, KEY, Observation};
use rstyle_pdg::HirFnFacts;
use rustc_hir::def_id::{LOCAL_CRATE, LocalDefId};
use rustc_middle::ty::TyCtxt;
use std::cell::OnceCell;
use std::collections::BTreeMap;

/// The crate's programs by owner identity, lowered once for every owner the capture asks.
#[derive(Default)]
pub struct Programs(OnceCell<Result<BTreeMap<String, Input>, String>>);

impl Programs {
    fn get(&self, tcx: TyCtxt<'_>, options: &super::Options<'_>, owner: &str) -> Input {
        let package = options.unit_context.map(|context| &context.package);
        let programs = self.0.get_or_init(|| match options.source_root {
            Some(root) => crate::cpg::runtime_bounds(tcx, (root, package)),
            None => Err("Advisory capture has no source root".into()),
        });
        let unavailable = |reason: String| Input::Unavailable { reason };
        match programs {
            Ok(programs) => programs.get(owner).cloned().unwrap_or_else(|| {
                unavailable("Owner is absent from the crate's compat.cpg document".into())
            }),
            Err(error) => unavailable(format!("compat.cpg lowering failed: {error}")),
        }
    }
}

pub(super) fn capture<B, S>(
    ctx: &Context<'_>,
    id: LocalDefId,
    facts: &mut HirFnFacts<B, S>,
    options: &super::Options<'_>,
) -> Result<(), String> {
    if std::env::var_os(CAPTURE_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Ok(());
    }

    let tcx = ctx.tcx;
    let owner = crate::ids::def_id_str(tcx, id.to_def_id());
    let input = options.runtime_bounds.get(tcx, options, &owner);
    let observation = Observation {
        schema: KEY.into(),
        owner,
        krate: format!("{:016x}", tcx.stable_crate_id(LOCAL_CRATE).as_u64()),
        input,
    };
    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(observation).map_err(|e| e.to_string())?,
    );
    Ok(())
}
