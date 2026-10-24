//! Exact selected compiler owner, root expansion, and original-source joins.
use super::*;

pub(super) fn selected<'a>(input: &Input<'a>) -> Result<(&'a Observation, &'a Expansion), String> {
    let identity = input
        .facts
        .build
        .frontend_options
        .get("definition_identity")
        .and_then(serde_json::Value::as_str)
        .filter(|identity| !identity.is_empty())
        .ok_or("Generated source lacks exact facts definition identity")?;
    if input.capture.identity != UNIT_IDENTITY
        || input.capture.scope != "selected-local-bang-expansion-closure"
    {
        return Err("Unsupported generated-source capture scope".into());
    }
    let mut matches = input
        .capture
        .outcomes
        .iter()
        .filter(|outcome| match outcome {
            Outcome::Observed { observation } => observation.owner.definition_identity == identity,
            Outcome::Unsupported { owner, .. } => owner.definition_identity == identity,
        });
    let Some(Outcome::Observed { observation }) = matches.next() else {
        return Err("Missing supported generated function observation".into());
    };
    if matches.next().is_some() || observation.identity != IDENTITY {
        return Err("Ambiguous or invalid generated function observation".into());
    }
    let mut roots = input
        .capture
        .expansions
        .iter()
        .filter(|root| root.expansion == observation.defining_expansion);
    let root = roots.next().ok_or("Missing exact defining expansion")?;
    if roots.next().is_some() {
        return Err("Ambiguous defining expansion identity".into());
    }

    local(input.facts, &observation.owner)?;
    local(input.facts, &observation.implementation)?;
    root_scope(input.facts, root)?;
    Ok((observation, root))
}

fn root_scope(facts: &HirFnFacts, root: &Expansion) -> Result<(), String> {
    if root.expansion.is_empty()
        || root.parent.is_some()
        || root.kind != ExpansionKind::Bang
        || root.local_inner_macros
        || !root.invocation.marks.is_empty()
        || facts
            .build
            .frontend_options
            .get("edition")
            .and_then(serde_json::Value::as_str)
            != Some(&root.edition)
    {
        return Err("Generated source requires one original local bang expansion".into());
    }
    local(
        facts,
        root.macro_definition
            .as_ref()
            .ok_or("Missing exact local macro identity")?,
    )
}

pub(super) fn local(facts: &HirFnFacts, definition: &Definition) -> Result<(), String> {
    if definition.path.is_empty() {
        return Err("Generated definition differs from exact compiled local crate".into());
    }
    local_crate(facts, definition)
}

/// rustc names the crate-root module with the empty path. Only an exact
/// independently checked module namespace can authorize that parent spelling.
pub(super) fn module_parent(
    facts: &HirFnFacts,
    definition: &Definition,
    namespace: &[String],
) -> Result<(), String> {
    local_crate(facts, definition)?;
    if definition.path != namespace.join("::") {
        return Err("Generated impl belongs to another original module".into());
    }
    Ok(())
}

fn local_crate(facts: &HirFnFacts, definition: &Definition) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Generated source lacks compiled crate identity")?;
    if !definition.local
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"].as_str() != Some(&definition.crate_name)
        || compiled["stable_crate_id"].as_str() != Some(&definition.stable_crate_id)
    {
        return Err("Generated definition differs from exact compiled local crate".into());
    }
    Ok(())
}

pub(super) fn source(
    input: &Input<'_>,
    observation: &Observation,
    root: &Expansion,
    expansion: &source::CheckedExpansion,
) -> Result<(), String> {
    let macro_definition = root
        .macro_definition
        .as_ref()
        .ok_or("Missing local macro identity")?;
    let file = input
        .census
        .files
        .iter()
        .find(|file| file.path == root.invocation.raw.path)
        .ok_or("Missing immutable invocation buffer")?;
    let invocation = crate::source_evidence::byte_range(&file.bytes, root.invocation.raw.span)?;

    if input.census.context.crate_name != macro_definition.crate_name
        || macro_definition.path != expansion.macro_path().join("::")
        || observation.owner.path != input.facts.name
        || input.facts.fn_span != root.invocation.raw.span
        || input.facts.source.bytes.as_str() != invocation
        || !input
            .facts
            .source
            .bytes
            .matches_identity(&input.facts.source.digest)
        || input
            .facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            != root.invocation.raw.path.to_str()
    {
        return Err(
            "Generated source disagrees with exact original invocation or macro identity".into(),
        );
    }
    Ok(())
}
