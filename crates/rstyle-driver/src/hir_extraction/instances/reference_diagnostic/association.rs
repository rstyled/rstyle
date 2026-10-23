//! Exact outer trait selection; this observation grants no body or effect proof.
use super::super::{
    Definition, comparisons, definition_record,
    reference_owners::{Projection, original},
    types,
};
use rustc_hir::def_id::LocalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::{Symbol, sym};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    identity: &'static str,
    complete: bool,
    method: Definition,
    implementation: Definition,
    trait_definition: comparisons::CompilerDefinition,
    trait_method: comparisons::CompilerDefinition,
    original_trait_arguments: Vec<original::Argument>,
    substituted_trait_arguments: types::Arguments,
    impl_arguments: types::Arguments,
    selected: Definition,
    selected_arguments: types::Arguments,
}

pub(super) fn capture<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    projection: &Projection<'tcx>,
) -> Result<Option<Observation>, String> {
    let Some(item) = tcx.opt_associated_item(owner.to_def_id()) else {
        return Ok(None);
    };
    let Some(trait_method) = item.trait_item_def_id() else {
        return Ok(None);
    };
    let implementation = tcx
        .impl_of_assoc(owner.to_def_id())
        .ok_or("Reference trait method lacks its containing implementation")?;
    let original = tcx
        .impl_trait_ref(implementation)
        .instantiate_identity()
        .skip_norm_wip();

    if !implementation.is_local()
        || original.def_id.is_local()
        || tcx.get_diagnostic_item(sym::From) != Some(original.def_id)
        || tcx.get_diagnostic_item(Symbol::intern("from_fn")) != Some(trait_method)
        || tcx.trait_of_assoc(trait_method) != Some(original.def_id)
    {
        return Err("Reference outer trait is outside exact diagnostic From/from_fn".into());
    }
    let selected = projection.select(tcx, trait_method, original.args)?;
    if selected.def_id() != owner.to_def_id() {
        return Err("Reference outer trait selects a different original implementation".into());
    }

    Ok(Some(Observation {
        identity: "rstyle.rust.reference-from-association/1",
        complete: true,
        method: definition_record(tcx, owner.to_def_id()),
        implementation: definition_record(tcx, implementation),
        trait_definition: comparisons::definition(tcx, original.def_id),
        trait_method: comparisons::definition(tcx, trait_method),
        original_trait_arguments: original::arguments(tcx, original.args, 0)?,
        substituted_trait_arguments: types::arguments(tcx, projection.args(tcx, original.args)?),
        impl_arguments: types::arguments(tcx, projection.arguments()),
        selected: definition_record(tcx, selected.def_id()),
        selected_arguments: types::arguments(tcx, selected.args),
    }))
}
