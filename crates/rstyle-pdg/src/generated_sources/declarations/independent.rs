//! Declaration source authority independent of any generated callable observation.
use super::*;
use crate::nominal_boundary::{
    census::generated_sources::{self as source, CompilerInput},
    resolution::Environment,
};

pub(crate) struct IndependentInput<'a> {
    pub facts: &'a HirFnFacts,
    pub capture: &'a UnitCapture,
    pub tree: &'a TypeTree,
    pub origin: &'a GeneratedOrigin,
}

pub(crate) fn verify_in(
    input: IndependentInput<'_>,
    environment: &Environment,
) -> Result<CheckedGeneratedDeclaration, String> {
    environment.check_context(input.facts)?;
    verify_using(input, environment.census(), |input| {
        source::verify_compiler_in(input, environment)
    })
}

pub(super) fn verify_using(
    input: IndependentInput<'_>,
    census: &Census,
    compiler: impl FnOnce(CompilerInput<'_>) -> Result<CheckedExpansion, String>,
) -> Result<CheckedGeneratedDeclaration, String> {
    let root = root(&input)?;
    authority::root_scope(input.facts, root)?;
    let expansion = compiler(CompilerInput {
        census,
        cfgs: &input.facts.build.cfgs,
        definition_header: root
            .definition
            .as_ref()
            .ok_or("Missing declaration macro header")?,
        invocation_macro: &root.invocation.raw,
        preceding_expansions: crate::extraction_profile::facts_support(
            input.facts,
            "rust.ordered-local-macro-source",
        ),
    })?;

    let definition = root
        .macro_definition
        .as_ref()
        .ok_or("Missing local declaration macro")?;
    if definition.crate_name != census.context.crate_name
        || definition.path != expansion.macro_path().join("::")
        || Some(root.name.as_str()) != expansion.macro_path().last().map(String::as_str)
    {
        return Err("Generated declaration macro identity differs from original source".into());
    }

    let source = SourceInput {
        facts: input.facts,
        census,
        capture: input.capture,
        expansion: &expansion,
        tree: input.tree,
        origin: input.origin,
    };
    module(&source)?;
    verify_source(source)
}

fn root<'a>(input: &IndependentInput<'a>) -> Result<&'a Expansion, String> {
    if input.capture.identity != UNIT_IDENTITY
        || input.capture.scope != "selected-local-bang-expansion-closure"
    {
        return Err("Unsupported generated declaration capture scope".into());
    }

    let mut roots = input
        .capture
        .expansions
        .iter()
        .filter(|root| root.expansion == input.origin.defining_expansion);
    let root = roots
        .next()
        .ok_or("Missing generated declaration defining expansion")?;
    if roots.next().is_some() {
        return Err("Ambiguous generated declaration defining expansion".into());
    }

    Ok(root)
}

pub(super) fn module<'a>(input: &SourceInput<'a>) -> Result<&'a Definition, String> {
    let definition = &input.origin.definition;
    unique(input.capture, definition)?;
    let structure = input
        .capture
        .emitted
        .iter()
        .find(|row| &row.definition == definition)
        .ok_or("Missing exact generated declaration definition")?;

    authority::module_parent(input.facts, &structure.parent, input.expansion.namespace())?;
    if input
        .capture
        .emitted
        .iter()
        .any(|row| row.parent.path == structure.parent.path && row.parent != structure.parent)
    {
        return Err("Conflicting exact generated module identities".into());
    }
    Ok(&structure.parent)
}

pub(super) fn unique(capture: &UnitCapture, definition: &Definition) -> Result<(), String> {
    if capture
        .emitted
        .iter()
        .filter(|row| row.definition.definition_identity == definition.definition_identity)
        .count()
        != 1
    {
        return Err("Missing or duplicate generated declaration definition identity".into());
    }
    Ok(())
}
