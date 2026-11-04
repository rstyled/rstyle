//! A shared-slice candidate's compiler observations under its `u8` lens: the lens, the
//! owner's identities and result type, every node's typed observation, and each method
//! call's receiver observations. The reader selects candidates and walks its own facts
//! over these records.

use super::{Lens, candidate, definition_record, methods, projection};
use crate::hir_extraction::{Options, hir::reference};
use rustc_hir::{self as hir, def_id::LocalDefId};
use rustc_middle::ty::TyCtxt;
use serde_json::{Map, Value, json};

/// A candidate owner's shared-slice projection record; none for another owner.
pub(in crate::hir_extraction) fn record(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    options: &Options<'_>,
) -> Option<Value> {
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
        || !candidate(tcx, id)
    {
        return None;
    }
    let Some(context) = crate::hir_extraction::source_context(tcx, id, options) else {
        return Some(json!({"available": false}));
    };
    let arguments = tcx.mk_args(&[tcx.types.u8.into()]);
    let normalized = projection::normalize(tcx, arguments, arguments);
    let checked = match projection::complete_arguments(tcx, normalized) {
        Ok(checked) => checked,
        Err(error) => return Some(json!({"available": true, "lens": {"error": error}})),
    };
    let lens = Lens {
        owner: id,
        arguments: checked.arguments,
    };
    let owner_id = hir::OwnerId { def_id: id };
    let (mut nodes, mut calls) = (Map::new(), Map::new());
    for (local, _) in tcx.hir_owner_nodes(owner_id).nodes.iter_enumerated() {
        let hir_id = hir::HirId {
            owner: owner_id,
            local_id: local,
        };
        let key = format!("{}.{}", id.local_def_index.as_u32(), local.as_u32());
        if let hir::Node::Expr(expression) = tcx.hir_node(hir_id) {
            calls.insert(key.clone(), methods::raw(&context, &lens, expression));
        }
        nodes.insert(key, fallible(lens.typed(&context, &reference(hir_id))));
    }
    let lens_record = json!({"arguments": checked.observation,
        "definition": definition_record(tcx, id.to_def_id()),
        "type_parameter": definition_record(tcx, tcx.generics_of(id).own_params[0].def_id),
        "owner": reference(tcx.local_def_id_to_hir_id(id)),
        "result": fallible(lens.endpoint(tcx, context.result)),
        "nodes": nodes, "methods": calls});

    Some(json!({"available": true, "lens": lens_record}))
}

/// An observation, or the error that prevented it.
pub(super) fn fallible<T: serde::Serialize>(observed: Result<T, String>) -> Value {
    match observed {
        Ok(value) => json!({"ok": value}),
        Err(error) => json!({"error": error}),
    }
}
