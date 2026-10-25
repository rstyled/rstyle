mod inventory;
mod values;

use super::{records::*, *};
use crate::{
    NodeCategory, PdgNode,
    data_documents::{serialized_operation, value_operation, verify_saved_operands},
    data_types::TypeToken,
    slice_patterns::graph,
};

pub(crate) fn install(
    pair: &mut Pair,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let expected = expected(pair.as_view(), checked)?;

    inventory::types(pair.as_view(), &expected, facts)?;
    inventory::install(pair, &expected)?;
    pair.pdg.extensions.insert(
        KEY.into(),
        serde_json::to_value(expected).map_err(|error| error.to_string())?,
    );

    Ok(())
}

pub(crate) fn verify(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let expected = expected(pair, checked)?;
    let actual: Contract = serde_json::from_value(
        pair.pdg
            .extensions
            .get(KEY)
            .ok_or("Missing complete guarded digit-product origin inventory")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;

    if actual != expected {
        return Err(
            "Digit-product origin inventory changes source components or saved graph values".into(),
        );
    }

    inventory::types(pair, &expected, facts)?;
    inventory::annotations(pair, &expected)
}

fn expected(pair: PairView<'_>, checked: &Checked<'_>) -> Result<Contract, String> {
    let replay = graph::verify(pair, checked.prefix())?;
    let mut contract = Contract::new(checked);

    values::direct(pair, checked, &replay, &mut contract)?;
    values::constructed(pair, checked, &replay, &mut contract)?;

    Ok(contract)
}

fn source_type(node: &HirNode) -> Result<TypeToken, String> {
    crate::data_types::parse(
        node.ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            .ok_or("Digit-product origin source lacks an exact value type")?,
    )
}
