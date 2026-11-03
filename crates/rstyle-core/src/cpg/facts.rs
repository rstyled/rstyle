//! FnFacts for the style rules, derived from one bundled unit document.

use super::events::{Events, Walk, mutations};
use super::metrics::{Constructs, statements, top_statements};
use super::model::{Coverage, Document, Function, Status};
use super::source::Sources;
use super::tree::{Body, Tables};
use crate::complexity::Measurement;
use crate::facts::FnFacts;

/// Every capability and layer a document must declare.
pub const CAPABILITIES: [&str; 13] = [
    "resolution",
    "dispatch",
    "generic-instances",
    "lifetimes",
    "types",
    "integer-semantics",
    "mutability",
    "aliasing",
    "implicit-calls",
    "constant-values",
    "evaluation-order",
    "exceptional-flow",
    "macro-provenance",
];
pub const LAYERS: [&str; 11] = [
    "unit",
    "method",
    "ast",
    "cfg",
    "symbol",
    "ref",
    "type",
    "value",
    "place",
    "evidence",
    "extension",
];

/// Rule facts of every function, in document order.
pub fn rule_facts(document: &Document) -> Result<Vec<FnFacts>, String> {
    declared(&document.coverage)?;
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = Sources::new(unit)?;
    let tables = Tables::new(unit)?;

    let measured = complexity_available(&document.coverage);
    let functions = unit.functions.as_deref().unwrap_or_default();
    functions
        .iter()
        .map(|function| function_facts(function, &tables, &sources, measured))
        .collect()
}

fn declared(coverage: &Coverage) -> Result<(), String> {
    let missing = CAPABILITIES
        .iter()
        .filter(|name| !coverage.capabilities.contains_key(**name))
        .chain(
            LAYERS
                .iter()
                .filter(|name| !coverage.layers.contains_key(**name)),
        )
        .copied()
        .collect::<Vec<_>>();
    match missing.as_slice() {
        [] => Ok(()),
        names => Err(format!("Coverage does not declare {}", names.join(", "))),
    }
}

/// Complexity needs the syntax tree; without it the score is unavailable, not zero.
fn complexity_available(coverage: &Coverage) -> bool {
    coverage
        .layers
        .get("ast")
        .is_some_and(|ast| matches!(ast.status, Status::Supported | Status::Approximated))
}

fn function_facts(
    function: &Function,
    tables: &Tables<'_>,
    sources: &Sources<'_>,
    measured: bool,
) -> Result<FnFacts, String> {
    let body = Body::new(function)?;
    let walk = Walk::new(&body, tables, sources);
    let events = walk.events()?;
    let mutations = mutations(&walk)?;
    let complexity = match measured {
        true => Some(measure(&body, sources, &events)?),
        false => None,
    };

    let signature = &function.signature;
    let (line, column) = sources.location(&signature.span)?;
    Ok(FnFacts {
        id: tables
            .definition(&function.definition)?
            .stable_identity
            .clone(),
        implements: implemented(function, tables)?,
        path: sources.path(&signature.span)?.to_string(),
        line,
        column,
        name: function.name.clone(),
        params: signature.parameters.len(),
        param_list: vec![],
        has_self: signature.receiver.is_some(),
        statements: statements(&body)?,
        top_statements: top_statements(&body)?,
        complexity,
        source: sources
            .between(&signature.span, &function.span)?
            .to_string(),
        preceding_line: sources.preceding_line(&signature.span, line)?,
        mutations,
        calls: events.calls,
        effects: events.effects,
        edges: events.edges,
        mutated_types: vec![],
        sig: None,
        pdg: None,
    })
}

fn measure(body: &Body<'_>, sources: &Sources<'_>, events: &Events) -> Result<Measurement, String> {
    let tree = Constructs { body, sources }.tree()?;
    Ok(Measurement::score(tree.iter().chain(&events.recursions)))
}

fn implemented(function: &Function, tables: &Tables<'_>) -> Result<Option<String>, String> {
    match &function.implements {
        Some(id) => Ok(Some(tables.definition(id)?.stable_identity.clone())),
        None => Ok(None),
    }
}
