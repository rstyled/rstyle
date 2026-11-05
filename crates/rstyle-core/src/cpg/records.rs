//! What a run's reports read of each function of a document: where it is written, what
//! it declares, its rule facts or, for a function a macro writes, its own events, and
//! on demand its dependence graph and runtime-bound program.

use super::model::{Document, FileSpan, Function, Generic, Origin, Type, TypeKind};
use super::source::Sources;
use super::tree::Tables;
use crate::advisory::generated_effects::{self, CONNECTED_KEY, Connections};
use crate::advisory::runtime_bounds::Input;
use crate::facts::{FnFacts, Pdg};
use std::collections::BTreeMap;

/// Why a function has no measured facts: nothing original was written to measure.
pub const UNMEASURED: &str = "Existing compiler collector has no original function source";

/// What a consumer asks of a document beyond each function's rule facts.
#[derive(Clone, Copy, Default)]
pub struct Demand {
    pub graphs: bool,
    pub bounds: bool,
}

/// A type as a signature declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredType {
    /// The scalar token of a scalar type, or `unit`.
    pub normalized: Option<String>,
    pub observed: String,
    /// The annotation as written, empty where there is none.
    pub spelling: String,
}

/// What a function declares, the receiver among its parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredSignature {
    pub parameters: Vec<DeclaredType>,
    pub result: DeclaredType,
    /// The function's own generic parameters in declaration order, lifetimes included.
    pub generics: Vec<String>,
    pub has_self: bool,
    pub modifiers: Vec<String>,
}

/// What can be measured of a function.
#[derive(Clone, Debug)]
pub enum Measured {
    Known(Box<FnFacts>),
    /// A macro writes it: its own events, located at the macro's call site.
    Generated(Box<generated_effects::Observation>),
    Unavailable,
}

/// One function of a document, as the reports read it.
#[derive(Clone, Debug)]
pub struct Record {
    pub name: String,
    pub identity: String,
    /// The compilation that owns the function: a path alone can name one in several.
    pub compilation: String,
    /// The body's root node, which tells functions of one name and source apart.
    pub root: String,
    pub located: FileSpan,
    pub source: String,
    pub signature: DeclaredSignature,
    pub measured: Measured,
    pub graph: Option<Pdg>,
    pub bounds: Option<Input>,
}

/// Every function of a bundled document, in document order.
pub fn records(document: &Document, demand: Demand) -> Result<Vec<Record>, String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = Sources::new(unit)?;
    let tables = Tables::new(unit)?;
    let derived = super::facts::derive(document, demand.graphs)?;
    let measured: BTreeMap<&str, &FnFacts> = derived
        .iter()
        .map(|facts| (facts.id.as_str(), facts))
        .collect();
    let functions = unit.functions.as_deref().unwrap_or_default();
    let mut bounds = match demand.bounds {
        true => super::lowered(document)?
            .into_iter()
            .map(|(_, input)| Some(input))
            .collect(),
        false => vec![None; functions.len()],
    };

    functions
        .iter()
        .zip(bounds.drain(..))
        .map(|(function, bounds)| {
            let definition = tables.definition(&function.definition)?;
            let identity = definition.stable_identity.clone();
            let facts = measured
                .get(identity.as_str())
                .map(|facts| (*facts).clone());
            let compilation = definition
                .compilation
                .clone()
                .unwrap_or_else(|| unit.id.clone());
            record(
                function,
                (identity, compilation),
                (facts, bounds),
                (&tables, &sources),
            )
            .map_err(|reason| format!("{}: {reason}", function.name))
        })
        .collect()
}

fn record(
    function: &Function,
    (identity, compilation): (String, String),
    (facts, bounds): (Option<FnFacts>, Option<Input>),
    (tables, sources): (&Tables<'_>, &Sources<'_>),
) -> Result<Record, String> {
    let at = located(function);
    let source = sources.between(at, at)?;
    let mut measured = measure(function, facts, sources)?;
    let graph = match &mut measured {
        Measured::Known(facts) => facts.pdg.take(),
        _ => None,
    };

    Ok(Record {
        name: function.name.clone(),
        identity,
        compilation,
        root: function.root.clone(),
        located: at.clone(),
        source: source.to_owned(),
        signature: signature(function, tables, source)?,
        measured,
        graph,
        bounds,
    })
}

/// Where a function is located: where it is written, or for a function a macro
/// generates, the macro's outermost call site.
pub fn located(function: &Function) -> &FileSpan {
    function
        .nodes
        .iter()
        .find(|node| node.id == function.root)
        .filter(|root| root.provenance.origin == Origin::MacroExpansion)
        .and_then(|root| root.provenance.call_site.as_ref())
        .unwrap_or(&function.span)
}

/// What can be measured of a function, given the facts derived for it: a macro-written
/// function has no original source, so only its own events are read.
pub fn measure(
    function: &Function,
    facts: Option<FnFacts>,
    sources: &Sources<'_>,
) -> Result<Measured, String> {
    let at = located(function);
    let generated = !std::ptr::eq(at, &function.span);
    match facts {
        Some(facts) if generated => {
            let (line, column) = sources.location(at)?;
            Ok(Measured::Generated(Box::new(
                generated_effects::Observation {
                    metrics: CONNECTED_KEY.into(),
                    name: function.name.clone(),
                    line,
                    column,
                    mutations: facts.mutations,
                    calls: facts.calls,
                    intrinsic_effects: facts.effects,
                    connections: Some(Connections {
                        id: facts.id,
                        implements: facts.implements,
                        edges: facts.edges,
                    }),
                },
            )))
        }
        Some(facts) => Ok(Measured::Known(Box::new(facts))),
        None => Ok(Measured::Unavailable),
    }
}

/// A type as a signature declares it: the scalar token, its spelling and the annotation.
pub fn declared_type(ty: &Type, spelling: String) -> DeclaredType {
    let normalized = match ty.kind {
        TypeKind::Integer | TypeKind::Bool | TypeKind::Char => Some(ty.spelling.clone()),
        TypeKind::Float if matches!(ty.bits, Some(32 | 64)) => Some(ty.spelling.clone()),
        TypeKind::Unit => Some("unit".to_owned()),
        _ => None,
    };
    DeclaredType {
        normalized,
        observed: ty.spelling.clone(),
        spelling,
    }
}

fn signature(
    function: &Function,
    tables: &Tables<'_>,
    source: &str,
) -> Result<DeclaredSignature, String> {
    let signature = &function.signature;
    let text = Text { function, source };
    let receiver = signature
        .receiver
        .iter()
        .map(|receiver| (&receiver.symbol, &receiver.ty, &receiver.annotation));
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| (&parameter.symbol, &parameter.ty, &parameter.annotation));
    let parameters = receiver
        .chain(parameters)
        .map(|(symbol, ty, annotation)| {
            // A binding's type is the one the frontend records for it, as it is used.
            let bound = function.symbols.iter().find(|record| record.id == *symbol);
            let ty = bound.and_then(|record| record.ty.as_ref()).unwrap_or(ty);
            Ok(declared_type(
                tables.ty(ty)?,
                text.spelling(annotation.as_ref()),
            ))
        })
        .collect::<Result<_, String>>()?;
    let spelling = match signature.implicit_result {
        true => "()".to_owned(),
        false => text.spelling(signature.result_span.as_ref()),
    };

    Ok(DeclaredSignature {
        parameters,
        result: declared_type(tables.ty(&signature.result)?, spelling),
        generics: generics(signature.generics.as_slice(), function),
        has_self: signature.receiver.is_some(),
        modifiers: signature.modifiers.clone(),
    })
}

fn generics(types: &[Generic], function: &Function) -> Vec<String> {
    let signature = &function.signature;
    let lifetimes = signature
        .lifetimes
        .iter()
        .map(|item| (item.position, &item.name));
    let types = types.iter().map(|item| (item.position, &item.name));
    let constants = signature
        .constants
        .iter()
        .map(|item| (item.position, &item.name));
    let mut declared: Vec<_> = lifetimes
        .chain(types)
        .chain(constants)
        .filter_map(|(position, name)| Some((position?, name.clone())))
        .collect();
    declared.sort_by_key(|(position, _)| *position);

    declared.into_iter().map(|(_, name)| name).collect()
}

/// A function's source, from which a declaration's spelling is read.
struct Text<'a> {
    function: &'a Function,
    source: &'a str,
}

impl Text<'_> {
    /// The text of a span; what a macro writes outside its function reads as the call site.
    fn spelling(&self, span: Option<&FileSpan>) -> String {
        let Some(span) = span else {
            return String::new();
        };
        let at = located(self.function);
        let outside = span.file != at.file || span.start < at.start || span.end > at.end;
        let span = match outside && !std::ptr::eq(at, &self.function.span) {
            true => at,
            false => span,
        };

        let range = span.start.saturating_sub(at.start)..span.end.saturating_sub(at.start);
        self.source.get(range).unwrap_or_default().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn example() -> Value {
        serde_json::from_str(include_str!("testdata/example.cpg.json")).unwrap()
    }

    fn read(document: Value, demand: Demand) -> Vec<Record> {
        records(
            &super::super::layout::bundle(vec![document]).unwrap(),
            demand,
        )
        .unwrap()
    }

    #[test]
    fn a_record_reads_a_function_as_its_document_records_it() {
        let document = example();
        let functions = document["unit"]["functions"].as_array().unwrap().clone();
        let records = read(document, Demand::default());
        assert_eq!(records.len(), functions.len());
        for (record, function) in records.iter().zip(&functions) {
            assert_eq!(record.name, function["name"]);
            assert_eq!(record.root, function["root"]);
            assert!(!record.source.is_empty());
            assert!(matches!(record.measured, Measured::Known(_)));
            assert!(record.graph.is_none());
            assert!(record.bounds.is_none());
        }
    }

    #[test]
    fn a_demand_adds_the_graph_and_the_bounds() {
        let records = read(
            example(),
            Demand {
                graphs: true,
                bounds: true,
            },
        );
        for record in &records {
            assert!(record.graph.is_some());
            assert!(record.bounds.is_some());
            let Measured::Known(facts) = &record.measured else {
                panic!("{} is not measured", record.name);
            };
            // The graph moves to the record, so the facts carry none of their own.
            assert!(facts.pdg.is_none());
        }
    }

    #[test]
    fn a_function_a_macro_writes_reports_its_own_events_at_the_call_site() {
        let mut document = example();
        let function = &mut document["unit"]["functions"][0];
        let root = function["root"].as_str().unwrap().to_owned();
        let span = function["span"].clone();
        let nodes = function["nodes"].as_array_mut().unwrap();
        let root = nodes
            .iter_mut()
            .find(|node| node["id"] == root.as_str())
            .unwrap();
        root["provenance"] = serde_json::json!({"origin": "macro_expansion", "call_site": span});
        let record = &read(
            document,
            Demand {
                graphs: true,
                bounds: false,
            },
        )[0];

        let Measured::Generated(observation) = &record.measured else {
            panic!("the function is not generated");
        };
        assert_eq!(observation.metrics, CONNECTED_KEY);
        assert_eq!(observation.name, record.name);
        assert!(observation.connections.is_some());
        assert!(record.graph.is_none());
    }
}
