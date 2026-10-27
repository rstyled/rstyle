//! Symbolic unit construction retains the complete checked mixed enum.
use super::*;
use crate::{reference_instances::products, unit_store::EvidenceFacts};

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = crate::field_locations::selected_operations(
        facts,
        &["expr_path"],
        products::unit_candidate,
    );
    verify_construct_inventory(pair, [products::UNIT], [sources.len()])?;
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        products::CAPABILITY,
        || !sources.is_empty(),
        "Checked mixed unit construction requires its registered policy",
    )?;

    for source in sources {
        let unit = products::unit(source, facts)?;
        let node = typed_operation(pair, source, products::UNIT, &unit.ty.text()?)?;
        if node.category != NodeCategory::Computation
            || node
                .attributes
                .operator
                .as_ref()
                .and_then(crate::StringFact::known_value)
                != Some(unit.name)
            || node.attributes.literal.is_some()
            || node.attributes.callee.is_some()
            || node.attributes.call_form.is_some()
            || node.attributes.write_form.is_some()
            || !node.attributes.reads.is_empty()
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
            || !node.attributes.symbol_ids.is_empty()
            || node.annotations.contains_key(crate::data_places::KEY)
        {
            return Err(
                "Checked mixed unit construction loses its exact tag or invents effects".into(),
            );
        }
        verify_saved_operands(pair, node, &[])?;
    }

    Ok(())
}
