//! Whole-formal shared results derived from independently verified callee returns.

use crate::{
    HirNode, NodeCategory,
    data_origins::{self, Origin},
    data_types::{self, TypeToken},
    extraction_profile::ExtractionProfile,
    field_locations::Projection,
    producer::Pair,
};
use std::collections::BTreeSet;

pub fn supported_result(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<bool, String> {
    if !ExtractionProfile::from_facts(facts)?.is_data() {
        return Ok(true);
    }
    let result = result_type(facts)?;
    Ok(!data_origins::shared(&result) || matches!(result, TypeToken::Shared(_)))
}

fn result_type(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<TypeToken, String> {
    data_types::parse(
        facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .ok_or("Missing helper result type")?,
    )
}

pub fn returned(
    facts: &dyn crate::unit_store::EvidenceFacts,
    pair: &Pair,
) -> Result<Option<Vec<Origin>>, String> {
    returned_view(facts, pair.as_view())
}

fn returned_view(
    facts: &dyn crate::unit_store::EvidenceFacts,
    pair: crate::producer::PairView<'_>,
) -> Result<Option<Vec<Origin>>, String> {
    if !ExtractionProfile::from_facts(facts)?.is_data() {
        return Ok(None);
    }
    let result = result_type(facts)?;
    if !data_origins::shared(&result) {
        return Ok(None);
    }
    if !matches!(result, TypeToken::Shared(_)) || !pair.pdg.memory_locations.is_empty() {
        return Err("Shared helper result requires a whole-formal memory-free proof".into());
    }

    let mut origins = BTreeSet::new();
    for node in pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Return)
    {
        let returned: Vec<Origin> = serde_json::from_value(
            node.annotations
                .get(data_origins::KEY)
                .ok_or("Missing verified callee RETURN origins")?
                .clone(),
        )
        .map_err(|error| error.to_string())?;
        if returned.is_empty() {
            return Err("Shared helper RETURN has no verified origin".into());
        }
        for origin in returned {
            verify_formal(&origin, facts, &result)?;
            origins.insert(origin);
        }
    }
    if origins.is_empty() {
        return Err("Shared helper result has no returning evidence".into());
    }
    Ok(Some(origins.into_iter().collect()))
}

fn verify_formal(
    origin: &Origin,
    facts: &dyn crate::unit_store::EvidenceFacts,
    result: &TypeToken,
) -> Result<(), String> {
    let parameter = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.position == origin.parameter)
        .ok_or("Unknown returned helper formal")?;
    let token = data_types::parse(
        parameter
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing returned helper formal type")?,
    )?;
    if token != *result || origin.path != [Projection::SharedReferent] {
        return Err(
            "Returned helper origin is not an exact same-type shared formal referent".into(),
        );
    }
    Ok(())
}

pub fn parameters(
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<u32>, String> {
    if !ExtractionProfile::from_facts(facts)?.is_data() {
        return Err("Shared helper result requires the Data profile".into());
    }
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let origins = dependency
        .summary()
        .reference_outputs
        .as_ref()
        .ok_or("Missing verified shared helper output summary")?;
    Ok(origins.iter().map(|origin| origin.parameter).collect())
}

pub fn origins(
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let mut origins = BTreeSet::new();
    for position in parameters(call, facts)? {
        let argument = call
            .children
            .get(position as usize)
            .ok_or("Missing returned helper argument")?;
        let actual = data_origins::origins(argument, facts)?;
        if actual.is_empty() {
            return Err("Returned helper argument lacks verified reference origins".into());
        }
        origins.extend(actual);
    }
    Ok(origins.into_iter().collect())
}

pub fn same_unit(
    caller: &dyn crate::unit_store::EvidenceFacts,
    callee: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    let context = caller
        .build
        .frontend_options
        .get(crate::nominal_boundary::CONTEXT_KEY)
        .ok_or("Missing caller selected-unit context")?;
    let captured: crate::nominal_boundary::UnitContext =
        serde_json::from_value(context.clone()).map_err(|error| error.to_string())?;
    captured.validate()?;
    let invocation = caller
        .build
        .frontend_options
        .get("invocation")
        .filter(|value| value.as_array().is_some_and(|args| !args.is_empty()))
        .ok_or("Missing caller compiler invocation")?;

    Ok(callee
        .build
        .frontend_options
        .get(crate::nominal_boundary::CONTEXT_KEY)
        == Some(context)
        && callee.build.frontend_options.get("invocation") == Some(invocation)
        && caller.build.rustc_commit == callee.build.rustc_commit
        && caller.build.rustc_version == callee.build.rustc_version
        && caller.build.producer_revision == callee.build.producer_revision
        && caller.build.edition == callee.build.edition
        && caller.build.target_triple == callee.build.target_triple
        && caller.build.data_layout == callee.build.data_layout
        && caller.build.cfgs == callee.build.cfgs
        && caller.build.features == callee.build.features
        && caller.build.opt_level == callee.build.opt_level)
}
