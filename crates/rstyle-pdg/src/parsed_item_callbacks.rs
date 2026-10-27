//! Bounded original generic callback proof, without value, cleanup or graph authority.
//! Same-unit source and scoped-call joins retain original evidence. The caller must
//! still prove lexical resolution, no-drop ownership and callback invocation semantics.
use crate::{generated_sources::capture::Definition, scalar_product_instances::Argument};
use serde_json::Value;

mod observation;
mod owning_unit;
mod source;
#[cfg(test)]
mod tests;

/// Canonical definitions supplied by verified source/library ownership, never display names.
pub struct Context<'a> {
    pub member: &'a Definition,
    pub product: &'a Definition,
    pub option: &'a Definition,
    pub nonzero: &'a Definition,
    pub constructor: &'a Definition,
}

/// Ordered source semantics with a verified bounded core callback payload.
/// Aggregate cleanup, lexical resolution and graph admission remain separate.
pub struct Checked {
    argument: Argument,
}

impl Checked {
    pub fn argument(&self) -> Argument {
        self.argument
    }

    /// Tuple construction saves the suffix before consuming the scalar in one callback.
    pub fn ordered_fields(&self) -> [u32; 2] {
        [0, 1]
    }
}

fn verify(
    owner_source: &str,
    method_facts: &crate::HirFnFacts,
    nested_call: &Value,
    context: &Context<'_>,
) -> Result<Checked, String> {
    source::verify(owner_source, method_facts)?;
    let argument = observation::verify(nested_call, context)?;
    Ok(Checked { argument })
}

/// Prove the original same-unit member and exact separately scoped callback call.
/// The core callback payload is checked independently. Aggregate cleanup, lexical
/// resolution and graph admission remain separate obligations.
pub fn verify_in(
    input: &dyn crate::unit_store::EvidenceFacts,
    nested_call: &Value,
    context: &Context<'_>,
) -> Result<Checked, String> {
    let original = input
        .original_local_member(&context.member.definition_identity)?
        .ok_or("Missing original same-unit callback member")?;
    owning_unit::verify(input.facts(), original, nested_call, context)?;
    let constructor = crate::nonzero_values::verify_new(
        &nested_call["selected_arguments"][3]["identity"],
        input,
    )?;

    let census = original
        .build
        .frontend_options
        .data::<std::sync::Arc<crate::nominal_boundary::census::Census>>()?;
    let path = original
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        .ok_or("Missing original callback source file")?;
    let files = census
        .files
        .iter()
        .filter(|file| file.path == std::path::Path::new(path))
        .collect::<Vec<_>>();
    let [file] = files.as_slice() else {
        return Err("Callback source requires one complete original unit file".into());
    };
    if !file.bytes.matches_identity(&file.digest)
        || !original
            .source
            .bytes
            .matches_identity(&original.source.digest)
    {
        return Err("Original callback source identity disagrees".into());
    }

    let checked = verify(file.bytes.as_ref(), original, nested_call, context)?;
    if constructor.scalar.spelling() != checked.argument().as_str() {
        return Err(
            "Callback body scalar differs from its independently verified NonZero constructor"
                .into(),
        );
    }
    Ok(checked)
}
