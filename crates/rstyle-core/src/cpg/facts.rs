//! FnFacts for the style rules, derived from one bundled unit document.

use super::dependence::{Roles, dependence_graph};
use super::effects::EffectClasses;
use super::events::{Events, Walk, mutations};
use super::metrics::{Constructs, statements, top_statements};
use super::model::{Coverage, Document, Function, Status};
use super::shapes::{mutated_types, param_list, signature_shape};
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

/// Rule facts of every function, in document order; dependence graphs are left out.
pub fn rule_facts(document: &Document) -> Result<Vec<FnFacts>, String> {
    derive(document, false)
}

/// Every fact of every function, dependence graphs included.
pub fn all_facts(document: &Document) -> Result<Vec<FnFacts>, String> {
    derive(document, true)
}

pub(super) fn derive(document: &Document, graphs: bool) -> Result<Vec<FnFacts>, String> {
    derive_each(document, graphs)?
        .into_iter()
        .map(|(_, facts)| facts)
        .collect()
}

/// Each function's facts, or why they could not be derived.
type EachFunction<'d> = Vec<(&'d Function, Result<FnFacts, String>)>;

/// Each function's facts, or why they could not be derived, in document order.
pub(super) fn derive_each(document: &Document, graphs: bool) -> Result<EachFunction<'_>, String> {
    declared(&document.coverage)?;
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = Sources::new(unit)?;
    let tables = Tables::new(unit)?;

    let measured = complexity_available(&document.coverage);
    let roles = graphs.then(|| Roles::of(&document.producer.namespace));
    let classes = EffectClasses::of(&document.producer.namespace);
    let functions = unit.functions.as_deref().unwrap_or_default();
    // A function whose file lies outside the workspace root has no workspace facts,
    // unless the build wrote it there, as it does when the target directory is elsewhere.
    let inside = |function: &&Function| {
        sources.marks(&function.signature.span).generated
            || sources
                .path(&function.signature.span)
                .is_ok_and(|path| !path.starts_with("../"))
    };
    Ok(functions
        .iter()
        .filter(inside)
        .map(|function| {
            let facts = function_facts(function, (&tables, &sources, roles, classes), measured);
            (function, facts)
        })
        .collect())
}

/// Every function's rule facts that can be derived, for a frontend's own capture: one
/// function that cannot be read is a gap in the report, not a lost unit.
pub fn derivable_facts(document: &Document, graphs: bool) -> Result<Vec<FnFacts>, String> {
    Ok(derive_each(document, graphs)?
        .into_iter()
        .filter_map(|(_, facts)| facts.ok())
        .collect())
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
    (tables, sources, roles, classes): (&Tables<'_>, &Sources<'_>, Option<&Roles>, &EffectClasses),
    measured: bool,
) -> Result<FnFacts, String> {
    let body = Body::new(function)?;
    let walk = Walk::new(&body, tables, sources, classes);
    let events = walk.events()?;
    let (mutations, mutated): (Vec<_>, Vec<_>) = mutations(&walk)?.into_iter().unzip();
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
        param_list: param_list(&body, sources)?,
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
        mutated_types: mutated_types(tables, mutated)?,
        sig: signature_shape(&body, tables)?,
        // Patterns are searched in source functions; a macro-written one has no graph.
        pdg: match roles.filter(|_| !super::records::written_by_macro(function)) {
            Some(roles) => Some(dependence_graph(&body, tables, sources, roles)?),
            None => None,
        },
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
