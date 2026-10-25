//! Whole tagged results retain a scoped reference to the complete verified callee.
use super::*;
use crate::{
    copy_tagged_values::call_wrappers::WholeCall, scalar_product_instances::forwarders,
    unit_store::EvidenceFacts,
};

const KEY: &str = "rstyle.data:digit-product-call";

#[derive(serde::Serialize)]
struct Contract<'a> {
    identity: &'static str,
    call: &'a crate::HirRef,
    actual: &'a crate::HirRef,
    formal: &'a crate::HirRef,
    parameter: u32,
    callee: serde_json::Value,
    result: &'static str,
    conditions: &'static str,
}

fn contract(checked: &forwarders::Checked<'_>) -> Result<serde_json::Value, String> {
    serde_json::to_value(Contract {
        identity: "rstyle.data.guarded-digit-forwarder/1",
        call: &checked.call().hir,
        actual: &checked.actual().hir,
        formal: &checked.formal().hir,
        parameter: 0,
        callee: checked.dependency().wire()?,
        result: "unchanged-whole-callee-return/1",
        conditions: "callee-graph-scoped-under-call/1",
    })
    .map_err(|error| error.to_string())
}

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = forwarders::verify(facts)? else {
        return Ok(false);
    };
    super::tagged_calls::graph(pair.as_view(), facts, &checked, true)?;
    pair.pdg.extensions.insert(KEY.into(), contract(&checked)?);
    Ok(true)
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = forwarders::verify(facts)? else {
        if pair.pdg.extensions.contains_key(KEY) {
            return Err("Unproved digit forwarding contract".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        forwarders::CAPABILITY,
        || true,
        "Digit forwarding requires its registered document policy",
    )?;
    if pair.pdg.extensions.get(KEY) != Some(&contract(&checked)?) {
        return Err(
            "Digit forwarding changes its saved value, call scope or complete callee result".into(),
        );
    }

    super::tagged_calls::graph(pair, facts, &checked, true)?;
    Ok(true)
}
