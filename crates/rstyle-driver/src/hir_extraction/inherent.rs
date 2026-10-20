//! Concrete same-unit inherent ownership, captured before helper linking.

use super::{Options, hir::Context};
use rstyle_pdg::{
    HirFnFacts,
    data_types::TypeToken,
    inherent_calls::{CallOwner, Definition},
};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::TyCtxt;

pub fn implementation(
    tcx: TyCtxt<'_>,
    id: DefId,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Option<DefId> {
    if !profile.supports("rust.verified-inherent-helper-calls")
        || !id.is_local()
        || tcx.def_kind(id) != rustc_hir::def::DefKind::AssocFn
        || tcx.generics_of(id).count() != 0
    {
        return None;
    }
    let owner = tcx.inherent_impl_of_assoc(id)?;
    (tcx.generics_of(owner).count() == 0).then_some(owner)
}

pub fn call_owner(tcx: TyCtxt<'_>, owner: DefId) -> CallOwner {
    CallOwner {
        identity: rstyle_pdg::inherent_calls::IDENTITY.into(),
        implementation_identity: format!("{:?}", tcx.def_path_hash(owner)),
    }
}

pub fn capture(ctx: &Context<'_>, id: LocalDefId, facts: &mut HirFnFacts) -> Result<(), String> {
    let tcx = ctx.tcx;
    let Some(implementation) = implementation(tcx, id.to_def_id(), ctx.profile) else {
        return Ok(());
    };
    // Expanded functions retain their facts and macro exclusion, but cannot
    // claim a unique original-source inherent definition at a shared callsite.
    if expanded_signature(tcx, id) {
        return Ok(());
    }
    if !facts.signature.has_self && !ctx.profile.supports("rust.static-inherent-helper-calls") {
        return Ok(());
    }
    let nominal = if facts.signature.has_self {
        owner_nominal(facts, ctx.profile)
    } else {
        super::concrete_types::static_owner(ctx, implementation)?
    };
    let Some(nominal) = nominal else {
        return Ok(());
    };

    let rustc_hir::Node::Item(item) = tcx.hir_node_by_def_id(implementation.expect_local()) else {
        return Err("Inherent owner is not an original implementation item".into());
    };
    let (_, implementation_span) = ctx
        .sources
        .range(tcx, item.span)
        .ok_or("Missing original inherent implementation span")?;
    let record = Definition {
        identity: rstyle_pdg::inherent_calls::IDENTITY.into(),
        definition_identity: format!("{:?}", tcx.def_path_hash(id.to_def_id())),
        implementation_identity: call_owner(tcx, implementation).implementation_identity,
        implementation_span,
        owner_nominal: nominal,
        source_file: facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing inherent source file")?
            .into(),
        function_span: facts.fn_span,
    };
    facts.build.frontend_options.insert(
        rstyle_pdg::inherent_calls::KEY.into(),
        serde_json::to_value(record).map_err(|error| error.to_string())?,
    );

    Ok(())
}

fn expanded_signature(tcx: TyCtxt<'_>, id: LocalDefId) -> bool {
    tcx.hir_node_by_def_id(id)
        .fn_sig()
        .is_some_and(|signature| signature.span.from_expansion())
}

fn owner_nominal(
    facts: &HirFnFacts,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Option<rstyle_pdg::concrete_types::Nominal> {
    let token = facts.signature.params.first()?.ty.normalized.as_deref()?;
    let owner = match rstyle_pdg::data_types::parse(token).ok()? {
        TypeToken::Shared(owner) => *owner,
        owner @ TypeToken::Struct { .. } if profile.supports("rust.owned-copy-self") => owner,
        _ => return None,
    };

    match owner {
        TypeToken::Struct { nominal, .. } => Some(nominal),
        _ => None,
    }
}

pub fn capture_unit(facts: &mut [HirFnFacts], options: &Options<'_>) -> Result<(), String> {
    if !options
        .selected()
        .supports("rust.verified-inherent-helper-calls")
    {
        return Ok(());
    }

    let mut definitions = facts
        .iter()
        .filter_map(|function| {
            function
                .build
                .frontend_options
                .get(rstyle_pdg::inherent_calls::KEY)
        })
        .map(|value| {
            serde_json::from_value::<Definition>(value.clone()).map_err(|error| error.to_string())
        })
        .map(|record| record.map(rstyle_pdg::inherent_calls::Association::from))
        .collect::<Result<Vec<_>, _>>()?;
    definitions.sort_by(|left, right| left.definition_identity.cmp(&right.definition_identity));
    let inventory = rstyle_pdg::inherent_calls::UnitDefinitions {
        identity: rstyle_pdg::inherent_calls::UNIT_IDENTITY.into(),
        definitions,
    };
    inventory.validate()?;

    let value = serde_json::to_value(inventory).map_err(|error| error.to_string())?;
    for function in facts {
        function
            .build
            .frontend_options
            .insert(rstyle_pdg::inherent_calls::UNIT_KEY.into(), value.clone());
    }
    Ok(())
}
