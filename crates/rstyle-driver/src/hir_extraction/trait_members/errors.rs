use super::model;
use crate::hir_extraction::{
    hir::Context,
    instances::{definition_record, types},
    module_expansions::file_span,
};
use rustc_hir::{def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;

pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    owner: LocalDefId,
    reference: ty::TraitRef<'tcx>,
) -> Result<Option<model::AssociatedError>, String> {
    let tcx = context.tcx;
    if tcx.get_diagnostic_item(Symbol::intern("TryFrom")) != Some(reference.def_id) {
        return Ok(None);
    }

    let declaration = error_member(tcx, reference.def_id)?;
    let implementation = tcx
        .impl_of_assoc(owner.to_def_id())
        .ok_or("Missing TryFrom impl")?;
    let member = error_member(tcx, implementation)?;
    if tcx.associated_item(member).trait_item_def_id() != Some(declaration) {
        return Err("TryFrom associated Error maps to another declaration".into());
    }

    let environment = ty::TypingEnv::post_analysis(tcx, owner);
    let declared = tcx.type_of(member).instantiate_identity();
    let declared = tcx
        .try_normalize_erasing_regions(environment, declared)
        .map_err(|_| "TryFrom declared Error normalization failed")?;

    let projected =
        ty::Ty::new_projection_from_args(tcx, ty::IsRigid::No, declaration, reference.args);
    let projected = tcx
        .try_normalize_erasing_regions(environment, ty::Unnormalized::new(projected))
        .map_err(|_| "TryFrom Error projection normalization failed")?;
    if declared != projected {
        return Err("TryFrom Error declaration and projection disagree".into());
    }

    Ok(Some(model::AssociatedError {
        trait_member: definition_record(tcx, declaration),
        implementation_member: definition_record(tcx, member),
        source: file_span(tcx, &context.sources, super::source_span(tcx, member)?)?,
        declared_type: types::endpoint(tcx, declared)?,
        normalized_projection: types::endpoint(tcx, projected)?,
    }))
}

fn error_member(
    tcx: TyCtxt<'_>,
    owner: rustc_hir::def_id::DefId,
) -> Result<rustc_hir::def_id::DefId, String> {
    let members: Vec<_> = tcx
        .associated_items(owner)
        .in_definition_order()
        .filter(|item| {
            tcx.def_kind(item.def_id) == DefKind::AssocTy && item.name().as_str() == "Error"
        })
        .map(|item| item.def_id)
        .collect();
    let [member] = members.as_slice() else {
        return Err("TryFrom requires one exact associated Error declaration".into());
    };
    Ok(*member)
}
