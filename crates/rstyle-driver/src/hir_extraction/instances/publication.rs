//! Complete bounded instance observations attached to unchanged original owners.
use super::diagnostic;
use crate::hir_extraction::{Options, source};
use rstyle_pdg::{HirFnFacts, extraction_profile::ExtractionProfile};
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::{self, TyCtxt};
use serde_json::json;
use std::collections::BTreeMap;

pub(in crate::hir_extraction) fn capture(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    if options.selected() != ExtractionProfile::DataV3 {
        return Ok(());
    }
    let owners: BTreeMap<_, _> = tcx
        .hir_body_owners()
        .filter(|id| candidate(tcx, id.to_def_id()))
        .map(|id| (format!("{:?}", tcx.def_path_hash(id.to_def_id())), id))
        .collect();
    for original in facts {
        let Some(id) = original
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            .and_then(|identity| owners.get(identity))
        else {
            continue;
        };
        let entries = diagnostic::projected(tcx, *id, scanned, options);
        original.build.frontend_options.insert(
            "rstyle.rust.scalar-instance-projections".into(),
            json!({"identity":"rstyle.rust.scalar-instance-projections/1",
                "complete":true,"requests":entries}),
        );
    }
    Ok(())
}

pub(super) fn candidate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    if tcx.def_kind(id) != DefKind::Fn {
        return false;
    }

    let generics = tcx.generics_of(id);
    if generics.parent.is_some()
        || generics.own_params.len() != 1
        || !matches!(
            generics.own_params[0].kind,
            ty::GenericParamDefKind::Type { .. }
        )
    {
        return false;
    }
    let signature = tcx.fn_sig(id).instantiate_identity().skip_binder();
    let same_parameter =
        |value: ty::Ty<'_>| matches!(value.kind(), ty::Param(parameter) if parameter.index == 0);
    !signature.inputs().is_empty()
        && signature
            .inputs()
            .iter()
            .all(|value| same_parameter(*value))
        && same_parameter(signature.output())
}
