//! Reuse the compiler's existing style collectors without strict admission.
use super::hir::Context;
use crate::extract::{Metric, Options};
use rstyle_core::advisory::{CAPTURE_ENV, KEY, METRICS, Observation, patterns};
use rstyle_pdg::HirFnFacts;
use rustc_hir::def_id::LocalDefId;

pub(super) fn capture<B, S>(
    ctx: &Context<'_>,
    id: LocalDefId,
    facts: &mut HirFnFacts<B, S>,
    capture: &super::Options<'_>,
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
        complexity: Some(capture.complexity),
        root: capture
            .source_root
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
        None => {
            capture_generated(ctx, id, facts, &options)?;
            Observation::Unavailable {
                reason: "Existing compiler collector has no original function source".into(),
            }
        }
    };

    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(observation).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn capture_generated<B, S>(
    ctx: &Context<'_>,
    id: LocalDefId,
    facts: &mut HirFnFacts<B, S>,
    options: &Options<'_>,
) -> Result<(), String> {
    use rstyle_core::advisory::generated_effects::{CONNECTED_KEY, Connections, Observation};
    let node = ctx.tcx.hir_node_by_def_id(id);
    let Some(sig) = node.fn_sig().filter(|sig| sig.span.from_expansion()) else {
        return Ok(());
    };
    let Some(body) = node.body_id().map(|body| ctx.tcx.hir_body(body)) else {
        return Ok(());
    };
    let start = ctx
        .tcx
        .sess
        .source_map()
        .lookup_char_pos(sig.span.source_callsite().lo());

    let collected = crate::effects::collect(ctx.tcx, id, body, options);
    let (mutations, _) = crate::mutations::collect(ctx.tcx, id, options);
    let observation = Observation {
        metrics: CONNECTED_KEY.into(),
        name: facts.name.clone(),
        line: start.line,
        column: start.col.0 + 1,
        mutations,
        calls: collected.calls,
        intrinsic_effects: collected.effects,
        connections: Some(Connections {
            id: crate::ids::def_id_str(ctx.tcx, id.to_def_id()),
            implements: crate::extract::implemented_trait_method(ctx.tcx, id),
            edges: collected.edges,
        }),
    };
    facts.build.frontend_options.insert(
        CONNECTED_KEY.into(),
        serde_json::to_value(observation).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn capture_graph<B, S>(
    facts: &mut HirFnFacts<B, S>,
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
