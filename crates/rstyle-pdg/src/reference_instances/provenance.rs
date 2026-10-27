//! Independent original owner/binder proof. This grants no operation or admission authority.
mod model;
mod source;
#[cfg(test)]
mod tests;
mod types;

use crate::HirFnFacts;
use model::{Definition, Observation};
use serde::Deserialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;

pub(super) fn verify(canonical: &HirFnFacts, observation: &Value, haystack: &str) -> Result<()> {
    let record = Observation::deserialize(observation).map_err(|error| error.to_string())?;
    if record.identity != "rstyle.diagnostic.reference-owner-provenance/1" || !record.complete {
        return Err("Reference provenance requires its complete original owner census".into());
    }
    types::arguments(&record.arguments, haystack, true)?;
    definition(canonical, &record.definition)?;

    let source = source::verify(canonical, &record)?;
    types::signature(canonical, &record.signature)?;
    clauses(
        Context {
            canonical,
            haystack,
        },
        observation,
        &record,
        &source,
    )
}

fn definition(canonical: &HirFnFacts, definition: &Definition) -> Result<()> {
    let options = &canonical.build.frontend_options;
    let owner = options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Reference provenance lacks canonical compiled-crate identity")?;
    if !definition.local
        || definition.path != canonical.name
        || options.get("definition_identity").and_then(Value::as_str)
            != Some(definition.definition_identity.as_str())
        || owner.get("crate_name").and_then(Value::as_str) != Some(definition.crate_name.as_str())
        || owner.get("stable_crate_id").and_then(Value::as_str)
            != Some(definition.stable_crate_id.as_str())
    {
        return Err("Reference provenance differs from the canonical original owner".into());
    }
    Ok(())
}

struct Context<'a> {
    canonical: &'a HirFnFacts,
    haystack: &'a str,
}

fn clauses(
    context: Context<'_>,
    raw: &Value,
    record: &Observation,
    source: &source::Bounds,
) -> Result<()> {
    let owner = &record.owners[source.owner];
    if owner.clauses.len() != 2 || !record.owners[1 - source.owner].clauses.is_empty() {
        return Err("Reference provenance omits or invents own/inherited compiler clauses".into());
    }

    for (index, clause) in owner.clauses.iter().enumerate() {
        let expected = ["core::marker::MetaSized", "core::convert::AsRef"][index];
        if clause.kind != "trait" || !clause.positive || clause.anchor != source.clauses[index] {
            return Err(
                "Reference compiler clause disagrees with the original source bound".into(),
            );
        }
        types::clause(clause, context.haystack, index)?;
        let definition = &clause.trait_definition;
        if definition.local
            || definition.crate_name != "core"
            || definition.path != expected
            || definition.definition_identity.is_empty()
            || definition.stable_crate_id.is_empty()
            || definition.crate_hash.is_empty()
        {
            return Err("Reference compiler clause lacks complete core identity".into());
        }
        super::operations::authority::bound(
            context.canonical,
            &raw["owners"][source.owner]["clauses"][index]["trait_definition"],
            expected,
        )?;
    }
    Ok(())
}
