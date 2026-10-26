//! Replay ordered scalar definitions from one exact saved tuple initializer.
use crate::{
    NodeCategory, PdgNode, producer::PairView, scalar_tuple_bindings as bindings,
    unit_store::EvidenceFacts,
};
use std::collections::BTreeSet;

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources =
        crate::field_locations::selected_operations(facts, &["stmt_let"], bindings::candidate);
    let mut expected = BTreeSet::new();
    for source in sources {
        let checked = bindings::verify(source, facts)?;
        verify_binding(pair, &checked, &mut expected)?;
    }

    for node in &pair.pdg.nodes {
        if node
            .attributes
            .construct
            .as_deref()
            .is_some_and(|construct| {
                [bindings::SELECTOR, bindings::DECLARATION].contains(&construct)
            })
            && !expected.contains(node.id.as_str())
        {
            return Err("Graph claims an unproved scalar tuple component or declaration".into());
        }
    }
    if !expected.is_empty() {
        crate::extraction_profile::require_document(
            &pair.pdg.profile,
            bindings::CAPABILITY,
            || true,
            "Scalar tuple bindings require their registered document policy",
        )?;
    }
    Ok(())
}

fn verify_binding<'a>(
    pair: PairView<'a>,
    checked: &bindings::Checked<'_>,
    expected: &mut BTreeSet<&'a str>,
) -> Result<(), String> {
    let saved = super::value_operation(pair, checked.initializer)?;
    let mut preceding = saved;
    for entry in &checked.entries {
        let selected =
            super::typed_operation(pair, entry.pattern, bindings::SELECTOR, entry.token)?;
        let declared = super::typed_operation(pair, entry.pattern, bindings::DECLARATION, "unit")?;
        component(selected, declared, entry)?;
        super::verify_saved_operands(pair, selected, &[saved])?;
        super::verify_saved_operands(pair, declared, &[selected])?;
        super::callbacks::order::completed_before(pair, preceding, selected)?;
        super::callbacks::order::completed_before(pair, selected, declared)?;
        if !expected.insert(selected.id.as_str()) || !expected.insert(declared.id.as_str()) {
            return Err("Scalar tuple components alias another original binding".into());
        }
        preceding = declared;
    }
    Ok(())
}

fn component(
    selected: &PdgNode,
    declared: &PdgNode,
    entry: &bindings::Entry<'_>,
) -> Result<(), String> {
    if selected.category != NodeCategory::Computation
        || selected
            .attributes
            .literal
            .as_ref()
            .and_then(|value| value.known_value())
            != Some(entry.position.to_string().as_str())
        || !selected.attributes.symbol_ids.is_empty()
        || !selected.attributes.reads.is_empty()
        || !selected.attributes.writes.is_empty()
        || selected.attributes.write_form.is_some()
        || declared.category != NodeCategory::Declaration
        || declared.attributes.symbol_ids != [entry.binding.symbol.clone()]
        || declared.attributes.write_form.as_deref() != Some("initialization")
        || declared.attributes.defines.len() != 1
        || !declared.attributes.reads.is_empty()
        || declared.attributes.writes.len() != 1
        || declared.attributes.writes[0].symbol_id.as_ref() != Some(&entry.binding.symbol)
        || declared.attributes.writes[0].location_id.is_some()
    {
        return Err("Scalar tuple graph changes component position or lexical definition".into());
    }
    Ok(())
}
