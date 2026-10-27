//! Replay the entire operation roster independently of producer lowering.
mod effects;
mod flow;
mod operands;
mod records;

use super::*;
use operations::Operations;
use serde_json::Value;

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &contract::Checked<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let ops = operations::load(pair, checked)?;

    crate::helper_calls::verify_documents(pair, facts)?;
    effects::verify(pair, checked, &ops)?;
    operands::verify(pair, facts, checked, &ops)?;
    flow::verify(pair, &ops)?;
    records::derive(pair, facts, checked, &ops)
}
