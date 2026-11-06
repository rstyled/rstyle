//! Places from rustc's expression-use visitor: every write and exclusive borrow of a
//! place rooted at a binding, in the order rustc reports them.

use super::body::Builder;
use crate::mutations::{Use, escapes, field_name, uses};
use rustc_hir::def_id::LocalDefId;
use rustc_middle::hir::place::ProjectionKind;
use rustc_middle::ty::TyCtxt;
use serde_json::{Value, json};

pub fn places<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    builder: &mut Builder<'_, 'tcx>,
) -> Vec<Value> {
    let uses = uses(tcx, owner);
    uses.iter()
        .enumerate()
        .map(|(index, place_use)| place(tcx, builder, index, place_use))
        .collect()
}

/// What the bodies of the function's coroutines write and borrow exclusively. Those bodies
/// run in its control flow, so its accesses need them; they are not its own for its rule
/// facts, so they are not among its places.
pub fn coroutine_places<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    builder: &mut Builder<'_, 'tcx>,
) -> Vec<Value> {
    let coroutines = tcx
        .nested_bodies_within(owner)
        .iter()
        .filter(|nested| tcx.is_coroutine(nested.to_def_id()));
    let uses: Vec<Use<'tcx>> = coroutines.flat_map(|nested| uses(tcx, nested)).collect();

    uses.iter()
        .enumerate()
        .map(|(index, place_use)| place(tcx, builder, index, place_use))
        .collect()
}

// rstyle-allow(mutation_per_target): the builder interns the place's type and node
fn place<'tcx>(
    tcx: TyCtxt<'tcx>,
    builder: &mut Builder<'_, 'tcx>,
    index: usize,
    place_use: &Use<'tcx>,
) -> Value {
    let Use {
        place,
        root,
        at,
        borrow,
    } = place_use;
    let before = std::iter::once(place.base_ty).chain(place.projections.iter().map(|p| p.ty));
    let path: Vec<Value> = before
        .zip(&place.projections)
        .filter_map(|(ty, projection)| match projection.kind {
            ProjectionKind::Field(field, variant) => {
                Some(json!({"field": field_name(ty, field.as_usize(), variant.as_usize())}))
            }
            ProjectionKind::Deref => Some(json!({"deref": true})),
            ProjectionKind::Index | ProjectionKind::Subslice => Some(json!({"index": null})),
            _ => None,
        })
        .collect();
    let node = builder.nearest_node(tcx, *at);

    let (access, mode) = if *borrow {
        ("read", "exclusive")
    } else {
        ("write", "value")
    };
    json!({"id": format!("{node}.place{index}"), "node": node, "root": {"symbol": builder.symbol_id(*root)},
           "path": path, "access": access, "mode": mode, "type": builder.unit.ty(place.ty()),
           "escapes": escapes(place)})
}
