//! Capture settings from the active compiler session, never fixture defaults.

use rstyle_pdg::frontend::*;
use rustc_middle::ty::TyCtxt;

pub(super) fn compiled_crate(
    tcx: TyCtxt<'_>,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    build: &mut BuildFacts,
) {
    if super::crate_association::enabled(profile) {
        build.frontend_options.insert(
            "rstyle.rust.compiled-crate".into(),
            serde_json::to_value(super::crate_association::local(tcx))
                .expect("serialize compiled crate observation"),
        );
    }
}

pub(super) fn source_identity(
    source: &rstyle_pdg::SourceText,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> String {
    if profile == rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 {
        source.checksum().to_owned()
    } else {
        // Historical function-source conformance, including pinned r0, remains SHA.
        source.identity().to_owned()
    }
}

pub fn digest(bytes: &[u8]) -> String {
    rstyle_pdg::source_identity::checksum(bytes)
}

pub fn build(tcx: TyCtxt<'_>, invocation: &[String]) -> BuildFacts {
    let session = tcx.sess;
    let cfgs = cfgs(tcx);
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
        ])
        .into(),
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

pub fn cfgs(tcx: TyCtxt<'_>) -> Vec<String> {
    let cfgs: Vec<String> = tcx
        .sess
        .config
        .iter()
        .map(|(name, value)| {
            value.map_or_else(|| name.to_string(), |value| format!("{name}=\"{value}\""))
        })
        .collect();
    let mut cfgs = cfgs;
    cfgs.sort();
    cfgs
}
