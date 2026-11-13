//! What a run's reports read of each function of a document: where it is written, what
//! it declares, its rule facts or, for a function a macro writes, its own events, and
//! on demand its dependence graph and runtime-bound program. A body, the code a module or a
//! type runs, is its author's but wasn't written as a function: it has facts and a graph,
//! and no rule reads them.

use super::defects::{self, Examined};
use super::flow::Derived;
use super::model::{
    Document, FileSpan, Function, Generic, Node, Origin, Provenance, Type, TypeKind,
};
use super::source::Sources;
use super::tree::{Tables, generated};
use crate::advisory::generated_effects::{self, CONNECTED_KEY, Connections};
use crate::advisory::runtime_bounds::Input;
use crate::facts::{FnFacts, Pdg, TyShape};
use crate::wording::Wording;
use std::collections::BTreeMap;

/// Why a function has no measured facts: nothing original was written to measure.
pub const UNMEASURED: &str = "Existing compiler collector has no original function source";

/// What a consumer asks of a document beyond each function's rule facts.
#[derive(Clone, Copy, Default)]
pub struct Demand {
    pub graphs: bool,
    pub bounds: bool,
    /// Dominators, post-dominators and the dependence graph the control flow gives.
    pub flow: bool,
    /// What the defect rules find in each function.
    pub defects: bool,
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
    /// Code its author wrote, but not as a function: what a module or a type runs when it's
    /// loaded or defined. It's compared like a function, and its effects count, but nothing
    /// that judges how a function is written reads it.
    Body(Box<FnFacts>),
    /// A macro writes it: its own events, located at the macro's call site.
    Generated(Box<generated_effects::Observation>),
    /// Nothing could be measured, and why.
    Unavailable(String),
}

impl Measured {
    /// The events of a function no rule reads, as its callers' effects read them: one a macro or
    /// the build writes, or a body. None for one a rule reads, or one nothing was measured of.
    pub fn observation(&self) -> Option<generated_effects::Observation> {
        match self {
            Self::Generated(observation) => Some((**observation).clone()),
            Self::Body(facts) => Some(body_observation((**facts).clone())),
            Self::Known(_) | Self::Unavailable(_) => None,
        }
    }
}

/// One function of a document, as the reports read it.
#[derive(Clone, Debug)]
pub struct Record {
    /// The function's document id.
    pub function: String,
    pub name: String,
    pub identity: String,
    /// The compilation that owns the function: a path alone can name one in several.
    pub compilation: String,
    /// The body's root node, which tells functions of one name and source apart.
    pub root: String,
    pub located: FileSpan,
    /// The path of the file it is located in.
    pub path: String,
    pub source: String,
    pub signature: DeclaredSignature,
    pub measured: Measured,
    pub graph: Option<Pdg>,
    pub bounds: Option<Input>,
    pub flow: Option<Derived>,
    pub defects: Option<Examined>,
    /// The language says the function's file holds tests.
    pub test: bool,
    /// Why the function's tree isn't all of it, when it isn't.
    pub syntax_gap: Option<&'static str>,
    /// The terms its language's messages use.
    pub wording: Wording,
    /// With graphs: what its receiver, parameters and result are, whatever spells them.
    pub signature_shapes: Vec<Option<TyShape>>,
    /// With graphs: every value the frontend knows at a written node, sorted. A constant's use can
    /// read the same while its value changes somewhere no function is. What a macro expands to can
    /// hold anything a build knows, such as its own revision, so it's left out.
    pub known_values: Vec<String>,
}

/// What a run reads beyond a function's graph when it asks for graphs.
fn with_graph_facts(
    record: Record,
    function: &Function,
    demand: Demand,
    tables: &Tables<'_>,
) -> Record {
    if !demand.graphs {
        return record;
    }
    Record {
        signature_shapes: super::shapes::signature_shapes(function, tables),
        known_values: known_values(function),
        ..record
    }
}

fn known_values(function: &Function) -> Vec<String> {
    let written = function.nodes.iter().filter(|node| !generated(node));
    let values = written.filter_map(|node| node.value.as_ref());
    let mut known: Vec<String> = values.map(|value| format!("{value:?}")).collect();

    known.sort_unstable();
    known
}

/// Records in an order independent of how their documents arrived. Runtime bounds compose
/// in record order, so without this a run's bounds would depend on it.
pub fn canonical_order(records: &mut [Record]) {
    records.sort_by(|a, b| a.canonical_key().cmp(&b.canonical_key()));
}

impl Record {
    /// Drops what only a merge-risk comparison reads (where the function is written, its text,
    /// its declared signature, the shapes and the known values), for a run whose records go to
    /// the analyses alone. None of their reports reads these.
    pub fn without_review_fields(self) -> Self {
        Self {
            located: FileSpan {
                file: String::new(),
                ..self.located
            },
            source: String::new(),
            signature: DeclaredSignature {
                parameters: Vec::new(),
                result: DeclaredType {
                    normalized: None,
                    observed: String::new(),
                    spelling: String::new(),
                },
                generics: Vec::new(),
                has_self: false,
                modifiers: Vec::new(),
            },
            signature_shapes: Vec::new(),
            known_values: Vec::new(),
            ..self
        }
    }

    /// Where this record sorts in a run, whatever order its documents arrived in.
    pub fn canonical_key(&self) -> (&str, &str, &str, &str) {
        (
            &self.compilation,
            &self.identity,
            &self.function,
            &self.root,
        )
    }

    /// `canonical_key`, owned, to keep after the record is gone.
    pub fn owned_canonical_key(&self) -> CanonicalKey {
        let (compilation, identity, function, root) = self.canonical_key();
        (
            compilation.into(),
            identity.into(),
            function.into(),
            root.into(),
        )
    }
}

/// Where a record sorts in a run: its compilation, identity, function and root.
pub type CanonicalKey = (String, String, String, String);

/// Every function of a bundled document, in document order. A document that cannot be
/// read in full is refused, as a run from another frontend must be.
pub fn records(document: &Document, demand: Demand) -> Result<Vec<Record>, String> {
    let (records, failed) = read(document, demand)?;
    match failed.into_values().next() {
        Some(reason) => Err(reason),
        None => Ok(records),
    }
}

/// Every function of a frontend's own capture, in document order: one whose facts cannot
/// be derived is recorded as unavailable, with why, so its unit is still reported.
pub fn records_with_gaps(document: &Document, demand: Demand) -> Result<Vec<Record>, String> {
    let (mut records, failed) = read(document, demand)?;
    for record in &mut records {
        if let Some(reason) = failed.get(record.function.as_str()) {
            record.measured = Measured::Unavailable(reason.clone());
        }
    }
    Ok(records)
}

/// Why each function whose facts failed could not be derived, by its document ID.
type Failed = BTreeMap<String, String>;

/// The records of a document, and why each function's facts that failed could not be derived.
fn read(document: &Document, demand: Demand) -> Result<(Vec<Record>, Failed), String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let sources = Sources::new(unit)?;
    let tables = Tables::new(unit)?;
    let (mut measured, failed) = measured_and_failed(document, demand)?;
    let functions = unit.functions.as_deref().unwrap_or_default();
    let mut bounds = match demand.bounds {
        true => super::lowered(document)?
            .into_iter()
            .map(|(_, input)| Some(input))
            .collect(),
        false => vec![None; functions.len()],
    };

    let records = functions
        .iter()
        .zip(bounds.drain(..))
        .map(|(function, bounds)| {
            let definition = tables.definition(&function.definition)?;
            let identity = definition.stable_identity.clone();
            let facts = measured.remove(&identity);
            let compilation = definition
                .compilation
                .clone()
                .unwrap_or_else(|| unit.id.clone());
            record(
                function,
                (identity, compilation),
                (
                    facts,
                    bounds,
                    flow_of(document, function, demand, tables.wording()),
                    defects_of(document, function, demand, (&tables, &sources)),
                ),
                (&tables, &sources),
            )
            .map(|record| Record {
                test: sources.marks(located(function)).test,
                syntax_gap: super::flow::syntax_gap(&document.coverage, function),
                ..with_graph_facts(record, function, demand, &tables)
            })
            .map_err(|reason| format!("{}: {reason}", function.name))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let records = with_own_text(records, functions, &sources)?;
    Ok((records, failed))
}

/// Each body with only its own text: its span, less the functions written in it. Each of
/// those is compared as itself, so a change to one isn't a change to the module that holds it.
fn with_own_text(
    mut records: Vec<Record>,
    functions: &[Function],
    sources: &Sources<'_>,
) -> Result<Vec<Record>, String> {
    let spans: Vec<&FileSpan> = functions.iter().map(located).collect();
    for (at, record) in records.iter_mut().enumerate() {
        let Measured::Body(facts) = &mut record.measured else {
            continue;
        };
        let others = spans[..at].iter().chain(&spans[at + 1..]).copied();
        let own = own_text(&record.located, others, sources)?;
        facts.source.clone_from(&own);
        record.source = own;
    }
    Ok(records)
}

/// The text of `span` with every span in `others` that lies within it left out.
fn own_text<'s>(
    span: &FileSpan,
    others: impl Iterator<Item = &'s FileSpan>,
    sources: &Sources<'_>,
) -> Result<String, String> {
    let mut inside: Vec<&FileSpan> = others
        .filter(|other| other.file == span.file)
        .filter(|other| span.start <= other.start && other.end <= span.end)
        .collect();
    inside.sort_by_key(|other| (other.start, std::cmp::Reverse(other.end)));

    let piece = |start, end| FileSpan {
        file: span.file.clone(),
        start,
        end,
    };
    let (mut text, mut from) = (String::new(), span.start);
    for other in inside {
        // It's within one already left out.
        if other.start < from {
            continue;
        }
        let kept = piece(from, other.start);
        text.push_str(sources.between(&kept, &kept)?);
        from = other.end;
    }
    let rest = piece(from, span.end);
    text.push_str(sources.between(&rest, &rest)?);
    Ok(text)
}

/// Each function's facts by identity, and why each one's that failed could not be derived.
fn measured_and_failed(
    document: &Document,
    demand: Demand,
) -> Result<(BTreeMap<String, FnFacts>, Failed), String> {
    let mut failed = BTreeMap::new();
    let mut measured = BTreeMap::new();
    for (function, facts) in super::facts::derive_each(document, demand.graphs)? {
        match facts {
            Ok(facts) => {
                measured.insert(facts.id.clone(), facts);
            }
            Err(reason) => {
                failed.insert(function.id.clone(), reason);
            }
        }
    }
    Ok((measured, failed))
}

fn defects_of(
    document: &Document,
    function: &Function,
    demand: Demand,
    (tables, sources): (&Tables<'_>, &Sources<'_>),
) -> Option<Examined> {
    (demand.defects && !sources.marks(located(function)).test).then(|| {
        let read = (&document.coverage, document.producer.namespace.as_str());
        defects::examine(read, function, tables, sources)
    })
}

fn flow_of(
    document: &Document,
    function: &Function,
    demand: Demand,
    wording: Wording,
) -> Option<Derived> {
    demand
        .flow
        .then(|| super::flow::derive(&document.coverage, function, wording))
}

fn record(
    function: &Function,
    (identity, compilation): (String, String),
    (facts, bounds, flow, defects): (
        Option<FnFacts>,
        Option<Input>,
        Option<Derived>,
        Option<Examined>,
    ),
    (tables, sources): (&Tables<'_>, &Sources<'_>),
) -> Result<Record, String> {
    let at = located(function);
    let source = sources.between(at, at)?;
    let mut measured = measure(function, facts, sources)?;
    let graph = match &mut measured {
        Measured::Known(facts) | Measured::Body(facts) => facts.pdg.take(),
        _ => None,
    };

    Ok(Record {
        function: function.id.clone(),
        name: function.name.clone(),
        identity,
        compilation,
        root: function.root.clone(),
        located: at.clone(),
        path: sources.path(at)?.to_owned(),
        source: source.to_owned(),
        signature: signature(function, tables, source)?,
        measured,
        graph,
        bounds,
        flow,
        defects,
        test: false,
        syntax_gap: None,
        wording: tables.wording(),
        signature_shapes: Vec::new(),
        known_values: Vec::new(),
    })
}

/// The body's root, when a macro or the compiler writes the whole function: its signature
/// and its root. A macro that wraps a function, or rewrites its signature, leaves it its
/// author's.
fn macro_written_root(function: &Function) -> Option<&Node> {
    let signature = function.signature.provenance.as_ref()?;
    matches!(signature.origin, Origin::MacroExpansion | Origin::Analysis)
        .then(|| function.nodes.iter().find(|node| node.id == function.root))
        .flatten()
        .filter(|root| root.provenance.origin == signature.origin)
}

/// Whether a macro writes the whole function.
pub fn written_by_macro(function: &Function) -> bool {
    macro_written_root(function).is_some()
}

/// The constructs a body is desugared from: what a module runs when it's loaded, and what a
/// type's definition runs, as a Python module or class body does.
const BODY_CONSTRUCTS: [&str; 2] = ["module", "type"];

/// Whether the function is a body: its signature and its root are desugared from a module or a
/// type, so the code is its author's but wasn't written as a function. A desugaring of anything
/// else, such as an `async fn`'s, leaves the function what it was.
pub fn is_body(function: &Function) -> bool {
    let from_body = |provenance: &Provenance| {
        provenance.origin == Origin::Desugaring
            && provenance
                .construct
                .as_deref()
                .is_some_and(|construct| BODY_CONSTRUCTS.contains(&construct))
    };
    let root = || function.nodes.iter().find(|node| node.id == function.root);
    function
        .signature
        .provenance
        .as_ref()
        .is_some_and(from_body)
        && root().is_some_and(|root| from_body(&root.provenance))
}

/// Where a function is located: where it is written, or for a function a macro
/// generates, the macro's outermost call site.
pub fn located(function: &Function) -> &FileSpan {
    macro_written_root(function)
        .and_then(|root| root.provenance.call_site.as_ref())
        .unwrap_or(&function.span)
}

/// What can be measured of a function, given the facts derived for it: a function a
/// macro or the build writes has no original source, so only its own events are read. A body
/// keeps its facts, apart from those a function's.
pub fn measure(
    function: &Function,
    facts: Option<FnFacts>,
    sources: &Sources<'_>,
) -> Result<Measured, String> {
    let at = located(function);
    let generated_file = sources.marks(&function.span).generated;
    let generated = generated_file || written_by_macro(function);
    match facts {
        Some(facts) if generated => Ok(Measured::Generated(Box::new(observed(
            facts,
            sources.location(at)?,
            generated_file,
        )))),
        Some(facts) if is_body(function) => Ok(Measured::Body(Box::new(facts))),
        Some(facts) => Ok(Measured::Known(Box::new(facts))),
        None => Ok(Measured::Unavailable(UNMEASURED.to_owned())),
    }
}

/// A body's own events, located where it's written.
pub fn body_observation(facts: FnFacts) -> generated_effects::Observation {
    let at = (facts.line, facts.column);
    observed(facts, at, false)
}

/// A function's own events, located at `(line, column)`, as a function no rule reads reports
/// them for its callers' effects: one a macro or the build writes, or a body.
pub fn observed(
    facts: FnFacts,
    (line, column): (usize, usize),
    generated_file: bool,
) -> generated_effects::Observation {
    generated_effects::Observation {
        metrics: CONNECTED_KEY.into(),
        name: facts.name,
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
        generated_file,
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
                ..Demand::default()
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
    fn a_function_the_compiler_writes_is_nobody_s_to_review() {
        let mut document = example();
        let function = &mut document["unit"]["functions"][0];
        let root = function["root"].as_str().unwrap().to_owned();
        let provenance = serde_json::json!({"origin": "analysis"});
        function["signature"]["provenance"] = provenance.clone();
        let nodes = function["nodes"].as_array_mut().unwrap();
        let root = nodes
            .iter_mut()
            .find(|node| node["id"] == root.as_str())
            .unwrap();
        root["provenance"] = provenance;
        let record = &read(document, Demand::default())[0];

        assert!(matches!(record.measured, Measured::Generated(_)));
    }

    #[test]
    fn a_function_a_macro_writes_reports_its_own_events_at_the_call_site() {
        let mut document = example();
        let function = &mut document["unit"]["functions"][0];
        let root = function["root"].as_str().unwrap().to_owned();
        let span = function["span"].clone();
        let provenance = serde_json::json!({"origin": "macro_expansion", "call_site": span});
        function["signature"]["provenance"] = provenance.clone();
        let nodes = function["nodes"].as_array_mut().unwrap();
        let root = nodes
            .iter_mut()
            .find(|node| node["id"] == root.as_str())
            .unwrap();
        root["provenance"] = provenance;
        let record = &read(
            document,
            Demand {
                graphs: true,
                ..Demand::default()
            },
        )[0];

        let Measured::Generated(observation) = &record.measured else {
            panic!("the function is not generated");
        };
        assert_eq!(observation.metrics, CONNECTED_KEY);
        assert_eq!(observation.name, record.name);
        assert!(observation.connections.is_some());
        assert!(record.graph.is_none());
        assert_eq!(
            record
                .measured
                .observation()
                .map(|observation| observation.line),
            Some(observation.line)
        );
    }
}
