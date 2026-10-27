//! Effects are restricted by operation role, never inferred from empty origins.
use super::*;

pub(super) fn verify(
    pair: PairView<'_>,
    checked: &contract::Checked<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    for spec in operations::specs(checked)? {
        let node = operations::operation(pair, &spec)?;
        match spec.name {
            "binding" | "outer-environment" | "map-environment" => {}
            "decoded-payload" => literal(node, "0")?,
            "count-payload" => literal(node, "1")?,
            _ => super::super::super::callbacks::effects(node, spec.category)?,
        }
    }

    binding_effects(checked, ops)?;
    reads(checked, ops)?;
    let reader = ops.get("reader")?;
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_places::KEY)
            || node
                .attributes
                .writes
                .iter()
                .any(|access| access.location_id.is_some())
            || node.id != reader.id
                && node
                    .attributes
                    .reads
                    .iter()
                    .any(|access| access.location_id.is_some())
        {
            return Err("Widening reader invents content reads or memory writes".into());
        }
    }
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial {
        return Err("Widening reader must retain its selected reader's panic limitation".into());
    }
    returned(ops.get("returned")?)
}

fn literal(node: &PdgNode, expected: &str) -> Result<(), String> {
    crate::data_documents::verify_value_effects(
        node,
        true,
        "Widening tuple projection invents effects",
    )?;
    if node
        .attributes
        .literal
        .as_ref()
        .and_then(crate::StringFact::known_value)
        != Some(expected)
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || node.attributes.exit_kind.is_some()
        || !node.attributes.outcomes.is_empty()
        || node.attributes.description.is_some()
    {
        return Err("Widening tuple projection changes its exact saved component".into());
    }
    Ok(())
}

fn binding_effects(checked: &contract::Checked<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (node, binding) in ops.bindings.into_iter().zip([
        checked.roles.decoded,
        checked.roles.count,
        checked.roles.converted,
    ]) {
        crate::data_documents::verify_binding_initialization(node, binding)?;
        if node.category != NodeCategory::Declaration
            || node.attributes.operator.is_some()
            || node.attributes.literal.is_some()
            || node.attributes.callee.is_some()
            || node.attributes.call_form.is_some()
            || node.attributes.exit_kind.is_some()
            || node.attributes.position.is_some()
            || !node.attributes.outcomes.is_empty()
        {
            return Err("Widening reader changes scalar callback initialization".into());
        }
    }
    Ok(())
}

fn reads(checked: &contract::Checked<'_>, ops: &Operations<'_>) -> Result<(), String> {
    for (name, symbol) in [
        ("slice", checked.roles.slice.symbol.as_str()),
        ("message", checked.roles.message.symbol.as_str()),
        ("outer-environment", checked.roles.message.symbol.as_str()),
        (
            "map-environment",
            checked
                .roles
                .count
                .binding
                .as_deref()
                .ok_or("Missing saved count binding")?,
        ),
        (
            "decoded-read",
            checked
                .roles
                .decoded
                .binding
                .as_deref()
                .ok_or("Missing decoded binding")?,
        ),
        (
            "converted-read",
            checked
                .roles
                .converted
                .binding
                .as_deref()
                .ok_or("Missing converted binding")?,
        ),
    ] {
        read(ops.get(name)?, symbol)?;
    }
    Ok(())
}

fn read(node: &PdgNode, symbol: &str) -> Result<(), String> {
    if node.category != NodeCategory::Read
        || node.attributes.symbol_ids != [symbol]
        || !matches!(node.attributes.reads.as_slice(), [read]
            if read.symbol_id.as_deref() == Some(symbol) && read.location_id.is_none())
        || !node.attributes.defines.is_empty()
        || !node.attributes.writes.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || node.attributes.exit_kind.is_some()
        || !node.attributes.outcomes.is_empty()
        || node.attributes.description.is_some()
    {
        return Err("Widening reader changes an immutable saved binding read".into());
    }
    Ok(())
}

fn returned(node: &PdgNode) -> Result<(), String> {
    crate::data_documents::verify_value_effects(node, false, "Widening return invents effects")?;
    if node.category != NodeCategory::Return
        || node.attributes.exit_kind != Some(crate::ExitKind::Normal)
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || !node.attributes.outcomes.is_empty()
    {
        return Err("Widening reader loses its complete ordinary return".into());
    }
    Ok(())
}
