use super::*;
use crate::data_origins;

pub(super) fn types(
    pair: PairView<'_>,
    contract: &Contract,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        let ty = wire_type(node)?;
        match contract.values.get(&node.id) {
            Some(expected) if expected.ty == ty && data_origins::shared(&ty) => {}
            None if !data_origins::shared(&ty) => ordinary_type(node, facts)?,
            _ => {
                return Err(
                    "Digit-product reference inventory changes a type or omits a value".into(),
                );
            }
        }
    }

    if contract
        .values
        .keys()
        .any(|id| !pair.pdg.nodes.iter().any(|node| node.id == *id))
    {
        return Err("Digit-product origin inventory names a nonexistent graph value".into());
    }
    Ok(())
}

fn ordinary_type(node: &PdgNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    // These exact synthetic operations were independently inventoried and typed
    // by graph::verify before this pass; their anchors are original patterns.
    if matches!(
        node.attributes.construct.as_deref(),
        Some("slice.prefix-length" | "slice.prefix-byte" | "slice.prefix-digit")
    ) {
        return Ok(());
    }
    crate::data_documents::origins::verify_type(node, facts)
}

fn wire_type(node: &PdgNode) -> Result<TypeToken, String> {
    crate::data_types::parse(&crate::data_documents::wire_type(node)?)
}

pub(super) fn install(pair: &mut Pair, contract: &Contract) -> Result<(), String> {
    for node in &mut pair.pdg.nodes {
        let origins = direct(contract.values.get(&node.id));
        if let Some(origins) = origins {
            let expected = serde_json::to_value(origins).map_err(|error| error.to_string())?;
            if node
                .annotations
                .get(data_origins::KEY)
                .is_some_and(|actual| actual != &expected)
            {
                return Err(
                    "Digit-product origins cannot replace contradictory reference annotations"
                        .into(),
                );
            }
            node.annotations.insert(data_origins::KEY.into(), expected);
        } else if node.annotations.contains_key(data_origins::KEY) {
            return Err(
                "Digit-product aggregate or scalar claims flattened reference origins".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn annotations(pair: PairView<'_>, contract: &Contract) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        let expected = direct(contract.values.get(&node.id));
        let actual: Option<Vec<Origin>> = node
            .annotations
            .get(data_origins::KEY)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|error| error.to_string())?;
        if actual.as_deref() != expected {
            return Err(
                "Digit-product annotations lose direct origins or flatten guarded components"
                    .into(),
            );
        }
    }
    Ok(())
}

fn direct(value: Option<&Value>) -> Option<&[Origin]> {
    match value.map(|value| &value.footprint) {
        Some(Footprint::Direct { origins, .. }) => Some(origins),
        _ => None,
    }
}
