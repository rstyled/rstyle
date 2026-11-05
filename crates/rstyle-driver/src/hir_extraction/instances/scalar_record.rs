//! The scalar-instance projections' compiler observations for one candidate owner, under
//! each projected scalar: the projection itself, the owner's provenance and result type,
//! and, for every node of the owner, its projected type and, for an expression, its
//! comparison, operand adjustments, dispatch types and assertion. The reader walks its
//! own facts over these records.

use super::super::{Options, hir::Context};
use super::projection::{Error, Projection};
use super::{adjustments, comparisons, definition_record, provenance, types};
use rustc_hir::{self as hir, def_id::LocalDefId};
use rustc_middle::ty::{Ty, TyCtxt};
use serde_json::{Map, Value, json};

/// The scalars each candidate is projected at.
const SCALARS: [&str; 2] = ["i32", "u32"];

/// A candidate owner's scalar projection records, by scalar; none for another owner.
pub(in crate::hir_extraction) fn record(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    options: &Options<'_>,
) -> Option<Value> {
    if options.selected() != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
        || !super::publication::candidate(tcx, id.to_def_id())
    {
        return None;
    }
    let Some(context) = super::super::source_context(tcx, id, options) else {
        return Some(json!({"available": false}));
    };
    let lenses: Map<String, Value> = SCALARS
        .into_iter()
        .map(|scalar| (scalar.to_owned(), lens(&context, id, scalar)))
        .collect();

    Some(json!({"available": true, "lenses": lenses}))
}

fn lens<'tcx>(context: &Context<'tcx>, owner: LocalDefId, scalar: &str) -> Value {
    let tcx = context.tcx;
    let argument = match scalar {
        "i32" => tcx.types.i32,
        _ => tcx.types.u32,
    };
    let arguments = tcx.mk_args(&[argument.into()]);
    let projection = match Projection::new(tcx, owner, arguments) {
        Ok(projection) => projection,
        Err(error) => return json!({"projection": error_name(error)}),
    };
    let provenance = match provenance::owned(context, owner, &projection) {
        Ok(owned) => json!({"ok": owned}),
        Err(error) => json!({"error": error}),
    };
    let nodes = nodes(context, owner, &projection);

    json!({"definition": definition_record(tcx, owner.to_def_id()),
           "arguments": types::arguments(tcx, arguments),
           "result": ty(projection.ty(tcx, context.result)),
           "provenance": provenance, "nodes": nodes})
}

/// Every node of the owner, by node ID.
fn nodes<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    projection: &Projection<'tcx>,
) -> Map<String, Value> {
    let tcx = context.tcx;
    let owner_id = hir::OwnerId { def_id: owner };
    tcx.hir_owner_nodes(owner_id)
        .nodes
        .iter_enumerated()
        .map(|(local, _)| {
            let id = hir::HirId {
                owner: owner_id,
                local_id: local,
            };
            let key = format!("{}.{}", owner.local_def_index.as_u32(), local.as_u32());
            (key, node(context, projection, id))
        })
        .collect()
}

fn node<'tcx>(context: &Context<'tcx>, projection: &Projection<'tcx>, id: hir::HirId) -> Value {
    let tcx = context.tcx;
    let projected = ty(projection.node_ty(tcx, context.typeck, id));
    let hir::Node::Expr(expression) = tcx.hir_node(id) else {
        return json!({"ty": projected});
    };
    let comparison = match comparisons::observe(context, expression, projection) {
        Ok(observation) => json!({"ok": observation}),
        Err(error) => json!({"error": error}),
    };
    let adjustment = match adjustments::operand(context, expression, projection) {
        Ok(observation) => json!({"ok": observation, "empty": observation.is_empty()}),
        Err(error) => json!({"error": error}),
    };
    let assertion = match provenance::assertion(context, expression) {
        Ok(assertion) => json!({"ok": assertion}),
        Err(error) => json!({"error": error}),
    };

    json!({"ty": projected, "expression": true, "comparison": comparison,
           "adjustment": adjustment, "dispatch": dispatch(context, projection, expression),
           "assertion": assertion})
}

/// An expression's dispatch operand and result types, projected.
fn dispatch<'tcx>(
    context: &Context<'tcx>,
    projection: &Projection<'tcx>,
    expression: &hir::Expr<'tcx>,
) -> Value {
    let Some((_, operands)) = context.dispatch_operands(expression) else {
        return json!({"unsupported": "Projected dispatch lacks original operands"});
    };
    let projected = |id| {
        projection
            .node_ty(context.tcx, context.typeck, id)
            .map(|ty| ty.to_string())
    };
    let operand_tys = operands
        .iter()
        .map(|operand| projected(operand.hir_id))
        .collect::<Result<Vec<_>, _>>();
    match (operand_tys, projected(expression.hir_id)) {
        (Ok(operand_tys), Ok(result_ty)) => {
            json!({"ok": {"operand_tys": operand_tys, "result_ty": result_ty}})
        }
        (Err(error), _) | (_, Err(error)) => json!({"error": error_name(error)}),
    }
}

/// A projected type as the facts resolve it, or the projection's error.
fn ty(projected: Result<Ty<'_>, Error>) -> Value {
    match projected {
        Ok(ty) => {
            let resolved = super::super::hir::resolved(ty, String::new());
            json!({"observed": resolved.observed, "normalized": resolved.normalized})
        }
        Err(error) => json!({"error": error_name(error)}),
    }
}

fn error_name(error: Error) -> String {
    format!("{error:?}")
}
