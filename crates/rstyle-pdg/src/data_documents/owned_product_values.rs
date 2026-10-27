//! Independently bind aggregate replacement to its old value, RHS and owner.
use super::*;
use crate::{owned_product_locals, owned_product_values::UPDATE, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let checked = if owned_product_locals::candidate(facts) {
        Some(owned_product_locals::verify(facts)?)
    } else {
        None
    };

    let count = checked
        .as_ref()
        .map_or(0, |checked| checked.updates().len());
    verify_construct_inventory(pair, [UPDATE], [count])?;

    let Some(checked) = checked else {
        return Ok(());
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        owned_product_locals::CAPABILITY,
        || true,
        "Owned aggregate update requires its registered policy",
    )?;

    for update in checked.updates() {
        verify_update(pair, &checked, update)?;
    }
    Ok(())
}

fn verify_update(
    pair: PairView<'_>,
    checked: &owned_product_locals::Checked<'_>,
    update: &owned_product_locals::CheckedUpdate<'_>,
) -> Result<(), String> {
    let source = update.assignment();

    let replaced = typed_operation(pair, source, UPDATE, &checked.product().text()?)?;
    verify_value_effects(
        replaced,
        true,
        "Owned aggregate replacement invents effects",
    )?;

    if replaced.category != NodeCategory::Computation
        || replaced
            .attributes
            .literal
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != Some(update.field_position().to_string().as_str())
        || replaced.attributes.write_form.is_some()
        || replaced.annotations.contains_key(crate::data_places::KEY)
    {
        return Err("Owned aggregate replacement loses its exact field".into());
    }

    let old = value_operation(pair, update.receiver())?;
    let rhs = value_operation(pair, update.value())?;
    verify_saved_operands(pair, replaced, &[old, rhs])?;

    let assigned = serialized_operation(pair, &source.span, |node| {
        node.category == NodeCategory::Assignment
    })?;

    if assigned.attributes.write_form.as_deref() != Some("reassignment")
        || assigned.attributes.symbol_ids != vec![checked.binding().symbol.clone()]
    {
        return Err("Owned aggregate assignment substitutes a different owner".into());
    }

    verify_saved_operands(pair, assigned, &[replaced])?;
    Ok(())
}

pub(super) fn field_role(
    field: &crate::field_locations::FieldOperation,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if !owned_product_locals::candidate(facts) {
        return Ok(false);
    }
    let checked = owned_product_locals::verify(facts)?;

    for update in checked.updates() {
        let target = update
            .assignment()
            .children
            .first()
            .ok_or("Missing checked update field")?;
        if field.hir == target.hir && field.span == target.span {
            if !matches!(
                field.location,
                crate::field_locations::Location::Unsupported { .. }
            ) {
                return Err("Owned local field update invents a formal memory location".into());
            }
            return Ok(true);
        }
    }
    Ok(false)
}
