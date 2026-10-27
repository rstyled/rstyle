//! Complete bounded digit-result consumption with guarded suffix and callback evidence.
mod checked;
mod selection;
pub(crate) mod source;
pub(crate) use checked::Checked;
pub(crate) const CAPABILITY: &str = "rust.checked-nonzero-digit-composition";

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }

    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Nonzero digit composition requires its registered capability".into());
    }
    checked::verify(facts).map(Some)
}

use crate::{scalar_product_instances::Argument, unit_store::EvidenceFacts};

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    matches!(
        crate::scalar_product_instances::forwarders::product_argument(
            facts,
            crate::scalar_product_instances::activation::VALUE_CAPABILITY,
            "expr_method_call"
        ),
        Some(Argument::NonZeroU8 | Argument::NonZeroU16)
    )
}

pub(crate) fn permits_type(text: &str, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    if (!text.contains("\"kind\":\"scalar_product_instance\"")
        && !text.contains("\"kind\":\"non_zero\""))
        || !candidate(facts)
    {
        return Ok(false);
    }

    let checked = verify(facts)?.ok_or("Missing nonzero receipt")?;

    let token = crate::data_types::parse(text)?;

    let permitted = [&checked.input, &checked.output]
        .into_iter()
        .any(|instance| {
            let product =
                crate::concrete_types::TypeTree::ScalarProductInstance(Box::new(instance.clone()));
            [
                &product,
                &crate::concrete_types::TypeTree::Option(Box::new(product.clone())),
                &instance.fields[0].ty,
                &instance.fields[1].ty,
            ]
            .iter()
            .any(|tree| token == crate::data_types::TypeToken::from_tree(tree))
        });

    Ok(permitted)
}

pub(crate) fn covers_method(
    node: &crate::HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    if !candidate(facts) || facts.body.children.first() != Some(node) {
        return Ok(false);
    }

    Ok(verify(facts)?.is_some_and(|checked| checked.calls.outer == node))
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || candidate(facts),
        "Nonzero digit composition requires its registered document policy",
    )
}
