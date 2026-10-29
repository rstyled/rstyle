//! Reuse the compiler's existing style collectors without strict admission.
use super::hir::Context;
use crate::extract::{Metric, Options};
use rstyle_core::advisory::{CAPTURE_ENV, KEY, METRICS, Observation, patterns};
use rstyle_pdg::HirFnFacts;
use rustc_hir::def_id::LocalDefId;

pub(super) fn capture(
    ctx: &Context<'_>,
    id: LocalDefId,
    facts: &mut HirFnFacts,
    source_root: Option<&std::path::Path>,
) -> Result<(), String> {
    if std::env::var_os(CAPTURE_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return Ok(());
    }
    let context = ctx
        .unit_context
        .as_ref()
        .ok_or("Advisory capture needs a selected unit")?;
    let root = context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing advisory package root")?;
    let with_graph =
        std::env::var_os(patterns::CAPTURE_ENV).as_deref() == Some(std::ffi::OsStr::new("1"));
    let options = Options {
        complexity: None,
        root: source_root
            .filter(|workspace| root.starts_with(workspace))
            .unwrap_or(root),
        with_pdg: with_graph,
        metric: Metric::Advisory,
    };
    let observation = match crate::extract::function_facts(ctx.tcx, id, &options) {
        Some(mut collected) => {
            capture_graph(facts, &mut collected)?;
            Observation::Known {
                metrics: METRICS.into(),
                facts: Box::new(collected),
            }
        }
        None => Observation::Unavailable {
            reason: "Existing compiler collector has no original function source".into(),
        },
    };

    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(observation).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn capture_graph(
    facts: &mut HirFnFacts,
    collected: &mut rstyle_core::facts::FnFacts,
) -> Result<(), String> {
    if let Some(graph) = collected.pdg.take() {
        let observation = patterns::Observation {
            schema: patterns::KEY.into(),
            owner: collected.id.clone(),
            graph,
        };
        facts.build.frontend_options.insert(
            patterns::KEY.into(),
            serde_json::to_value(observation).map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}
