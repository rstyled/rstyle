//! Capture settings from the active compiler session, never fixture defaults.

use rstyle_pdg::frontend::*;
use rustc_middle::ty::TyCtxt;
use sha2::{Digest, Sha256};

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn build(tcx: TyCtxt<'_>, invocation: &[String]) -> BuildFacts {
    let session = tcx.sess;
    let cfgs: Vec<String> = session
        .config
        .iter()
        .map(|(name, value)| {
            value.map_or_else(|| name.to_string(), |value| format!("{name}=\"{value}\""))
        })
        .collect();
    let mut cfgs = cfgs;
    cfgs.sort();
    let features = cfgs
        .iter()
        .filter_map(|cfg| {
            cfg.strip_prefix("feature=\"")
                .and_then(|s| s.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect();

    BuildFacts {
        producer_revision: known_str(env!("RSTYLE_PRODUCER_REVISION")),
        frontend_options: std::collections::BTreeMap::from([
            (
                "edition".to_string(),
                serde_json::json!(session.edition().to_string()),
            ),
            (
                "target".to_string(),
                serde_json::json!(session.opts.target_triple.to_string()),
            ),
            ("invocation".to_string(), serde_json::json!(invocation)),
        ]),
        rustc_version: known_str(env!("RSTYLE_RUSTC_VERSION")),
        rustc_commit: known_str(env!("RSTYLE_RUSTC_COMMIT")),
        edition: known_str(&session.edition().to_string()),
        target_triple: known_str(&session.target.llvm_target),
        pointer_width: known_count(u64::from(session.target.pointer_width)),
        data_layout: known_str(&session.target.data_layout),
        cfgs,
        features,
        overflow_checks: known_bool(session.overflow_checks()),
        debug_assertions: known_bool(session.opts.debug_assertions),
        panic_strategy: known_str(session.panic_strategy().desc()),
        opt_level: known_str(&format!("{:?}", session.opts.optimize)),
    }
}
