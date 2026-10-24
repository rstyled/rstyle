//! Owning concrete conversion observations; no callable or admission authority.
mod conversions;
mod defaults;
mod errors;
mod model;

use super::{hir::Context, instances};
use rstyle_pdg::{HirFnFacts, extraction_profile::ExtractionProfile};
use rustc_hir::def_id::{DefId, LocalDefId};
use serde::Serialize;

pub(super) use defaults::default_call;

const KEY: &str = "rstyle.rust.concrete-conversion-member";

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Outcome {
    Observed {
        observation: Box<model::Observation>,
    },
    Unsupported {
        reason: String,
    },
}

pub(super) fn capture(
    context: &Context<'_>,
    owner: LocalDefId,
    facts: &mut HirFnFacts,
) -> Result<(), String> {
    if context.profile != ExtractionProfile::DataV3 {
        return Ok(());
    }
    if candidate(context.tcx, owner, &["default_fn"]) {
        return defaults::capture(context, owner, facts);
    }
    if !candidate(context.tcx, owner, &["from_fn", "try_from_fn"]) {
        return Ok(());
    }

    let outcome = match conversions::observe(context, owner) {
        Ok(observation) => Outcome::Observed {
            observation: Box::new(observation),
        },
        Err(reason) => Outcome::Unsupported { reason },
    };
    facts.build.frontend_options.insert(
        KEY.into(),
        serde_json::to_value(outcome).map_err(|error| error.to_string())?,
    );
    Ok(())
}

fn candidate(
    tcx: rustc_middle::ty::TyCtxt<'_>,
    owner: LocalDefId,
    diagnostic_names: &[&str],
) -> bool {
    tcx.opt_associated_item(owner.to_def_id())
        .and_then(|item| item.trait_item_def_id())
        .is_some_and(|method| {
            diagnostic_names.iter().any(|name| {
                tcx.get_diagnostic_item(rustc_span::Symbol::intern(name)) == Some(method)
            })
        })
}

fn source_span(
    tcx: rustc_middle::ty::TyCtxt<'_>,
    definition: DefId,
) -> Result<rustc_span::Span, String> {
    let local = definition
        .as_local()
        .ok_or("Conversion source must be local")?;
    let span = match tcx.hir_node_by_def_id(local) {
        rustc_hir::Node::Item(item) => item.span,
        rustc_hir::Node::ImplItem(item) => item.span,
        _ => return Err("Conversion source requires an original impl or member".into()),
    };
    if span.from_expansion() {
        return Err("Generated conversion source remains unsupported".into());
    }
    Ok(span)
}
