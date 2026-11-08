//! A call that passes one variable where its sibling methods' same calls all pass another. Methods
//! of one type written as near copies of each other agree on what they pass, so the one that
//! doesn't stands out, as in "Bugs as Deviant Behavior" (Engler et al., SOSP 2001):
//!
//! ```go
//! func (d *Writer) Create(ctx context.Context, options *Options) {
//!     d.record(false, options.Kind, method, start)
//! }
//! func (d *Writer) Delete(ctx context.Context, name string, options *Options) {
//!     d.record(false, name, method, start) // the siblings all pass options.Kind
//! }
//! ```
//!
//! A difference is only a slip when nothing else explains it, so the rule asks four things of it:
//! that both sides read a variable rather than a constant, that the call differs in that argument
//! alone, that the siblings' variable holds a value at the call, and that the one passed wasn't
//! computed from it.

use super::super::flow::Vertex;
use super::super::model::{AccessMode, Dispatch, FileSpan, Node, NodeKind, Origin, TargetKind};
use super::ignored_result::declared_callee;
use super::{Defect, Reading};
use crate::config::DeviantArgumentConfig;
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub const RULE: &str = "deviant_argument";

/// What the receiver is called in a written value, so methods naming it differently compare.
const RECEIVER: &str = "self";

/// One argument a call passes, and what its function knows at the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    /// The callee's identity, which calls are compared by.
    pub callee: String,
    /// The callee as a message names it.
    pub name: String,
    pub position: usize,
    pub line: usize,
    /// The argument as its siblings would write it, with the receiver called `self`.
    pub value: String,
    /// The argument as its author wrote it, which a message quotes.
    pub text: String,
    /// The argument's type as spelled, when the frontend gives one.
    pub ty: Option<String>,
    /// Every argument of the call, written the same way.
    pub call: Vec<String>,
    /// The variable the argument reads, when it reads one rather than a constant.
    pub variable: Option<String>,
    /// The variables that hold a value when the call runs.
    pub available: BTreeSet<String>,
    /// The variables the argument's own variable was computed from in this function.
    pub sources: BTreeSet<String>,
}

/// A method's arguments, and the callees it calls, which say how alike two methods are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arguments {
    /// The type the function is a method of; a free function has none and is never compared.
    pub owner: Option<String>,
    pub callees: BTreeSet<String>,
    pub arguments: Vec<Argument>,
}

pub(super) fn of(reading: &Reading<'_>) -> Result<Arguments, String> {
    let Some(owner) = reading.function.signature.owner_type.as_deref() else {
        return Ok(Arguments::default());
    };
    let mut found = Arguments {
        owner: Some(reading.tables.ty(owner)?.spelling.clone()),
        ..Arguments::default()
    };

    for node in reading
        .function
        .nodes
        .iter()
        .filter(|node| compared(reading, node))
    {
        let Some(callee) = declared_callee(reading, node)? else {
            continue;
        };
        found.callees.insert(callee.stable_identity.clone());
        let callee = (callee.stable_identity.as_str(), callee.name.as_str());
        found.arguments.extend(arguments_of(reading, node, callee)?);
    }
    Ok(found)
}

/// Whether a call is one its siblings' calls say anything of: one someone wrote, of something
/// other than a builtin, whose arguments vary with what it's asked, as `len`'s do.
fn compared(reading: &Reading<'_>, node: &Node) -> bool {
    let builtin = reading
        .body
        .reference(&node.id)
        .is_some_and(|reference| reference.dispatch == Some(Dispatch::Builtin));
    node.kind == NodeKind::Call && node.provenance.origin == Origin::Source && !builtin
}

/// What a call passes. A call's first child is what it calls or calls it on; the rest are passed.
fn arguments_of(
    reading: &Reading<'_>,
    call: &Node,
    (callee, name): (&str, &str),
) -> Result<Vec<Argument>, String> {
    let Some(span) = call.span.as_ref() else {
        return Ok(Vec::new());
    };
    let Some(point) = point_of(reading, call, span) else {
        return Ok(Vec::new());
    };
    let passed: Vec<&Node> = reading.body.children(call)?.into_iter().skip(1).collect();
    let call_values = passed
        .iter()
        .map(|argument| written(reading, argument))
        .collect::<Result<Vec<_>, _>>()?;
    let texts = passed
        .iter()
        .map(|argument| source_text(reading, argument))
        .collect::<Result<Vec<_>, _>>()?;
    let available = available(reading, &point)?;
    let (line, _) = reading.sources.location(span)?;

    let mut found = Vec::new();
    for (position, argument) in passed.iter().enumerate() {
        let variable = variable_of(reading, argument)?;
        let sources = match &variable {
            Some((symbol, _)) => sources(reading, symbol, &point)?,
            None => BTreeSet::new(),
        };
        found.push(Argument {
            callee: callee.to_owned(),
            name: name.to_owned(),
            position,
            line,
            value: call_values[position].clone(),
            text: texts[position].clone(),
            ty: argument
                .ty
                .as_deref()
                .map(|ty| reading.tables.ty(ty).map(|ty| ty.spelling.clone()))
                .transpose()?,
            call: call_values.clone(),
            variable: variable.map(|(_, name)| name),
            available: available.clone(),
            sources,
        });
    }
    Ok(found)
}

/// The argument as its siblings would write it: a variable by name, the receiver as `self`, a
/// field by what it's read from, and anything else as its source text.
fn written(reading: &Reading<'_>, node: &Node) -> Result<String, String> {
    if node.kind == NodeKind::FieldAccess
        && let (Some(base), Some(member)) = (node.children.first(), node.member.as_deref())
    {
        let base = written(reading, reading.body.node(base)?)?;
        return Ok(format!("{base}.{member}"));
    }
    if let Some((_, name)) = symbol_read(reading, node)? {
        return Ok(name);
    }

    source_text(reading, node)
}

fn source_text(reading: &Reading<'_>, node: &Node) -> Result<String, String> {
    let Some(span) = node.span.as_ref() else {
        return Ok(String::new());
    };
    Ok(reading.sources.between(span, span)?.to_owned())
}

/// The variable an argument reads, through fields, indexes and references to it.
fn variable_of(reading: &Reading<'_>, node: &Node) -> Result<Option<(String, String)>, String> {
    let mut at = node;
    loop {
        match at.kind {
            NodeKind::FieldAccess | NodeKind::IndexAccess | NodeKind::Reference => {
                let Some(base) = at.children.first() else {
                    return Ok(None);
                };
                at = reading.body.node(base)?;
            }
            NodeKind::Identifier => return symbol_read(reading, at),
            _ => return Ok(None),
        }
    }
}

/// The symbol an identifier reads, by id and as written, when it names a variable of this
/// function rather than something declared elsewhere.
fn symbol_read(reading: &Reading<'_>, node: &Node) -> Result<Option<(String, String)>, String> {
    if node.kind != NodeKind::Identifier {
        return Ok(None);
    }
    let target = reading
        .body
        .reference(&node.id)
        .and_then(|reference| reference.target.as_ref());
    let Some(id) = target
        .filter(|target| target.kind == TargetKind::Symbol)
        .and_then(|target| target.id.as_deref())
    else {
        return Ok(None);
    };
    Ok(Some((id.to_owned(), name_of(reading, id)?)))
}

fn name_of(reading: &Reading<'_>, symbol: &str) -> Result<String, String> {
    let receiver = reading.function.signature.receiver.as_ref();
    if receiver.is_some_and(|receiver| receiver.symbol == symbol) {
        return Ok(RECEIVER.to_owned());
    }
    Ok(reading.body.symbol(symbol)?.name.clone())
}

/// Where a call's values are judged: at its own vertex, or, for a call in a closure the flow
/// doesn't enter, at the vertex that makes the closure, with what the closure binds before the
/// call.
struct Point<'r> {
    vertex: Vertex,
    /// The closure's span, and where in it the call starts.
    closure: Option<(&'r FileSpan, usize)>,
}

fn point_of<'r>(reading: &Reading<'r>, call: &Node, span: &FileSpan) -> Option<Point<'r>> {
    if let Some(vertex) = reading.flow.vertex(&call.id) {
        return Some(Point {
            vertex,
            closure: None,
        });
    }
    let mut at = reading.parents.get(call.id.as_str()).copied()?;
    loop {
        if at.kind == NodeKind::Closure
            && let (Some(vertex), Some(closure)) = (reading.flow.vertex(&at.id), at.span.as_ref())
        {
            return Some(Point {
                vertex,
                closure: Some((closure, span.start)),
            });
        }
        at = reading.parents.get(at.id.as_str()).copied()?;
    }
}

/// The variables holding a value at the point: those reaching it, and those a closure binds
/// before its call.
fn available(reading: &Reading<'_>, point: &Point<'_>) -> Result<BTreeSet<String>, String> {
    let mut found = available_at(reading, point.vertex)?;
    let Some((closure, call)) = point.closure else {
        return Ok(found);
    };
    for symbol in &reading.function.symbols {
        let bound_before = symbol
            .declared
            .as_ref()
            .is_some_and(|declared| within(declared, closure) && declared.end <= call);
        if bound_before {
            found.insert(name_of(reading, &symbol.id)?);
        }
    }
    Ok(found)
}

/// What a variable's value at the point was made of. Inside a closure the flow doesn't enter,
/// that's what the statement declaring it reads.
fn sources(
    reading: &Reading<'_>,
    symbol: &str,
    point: &Point<'_>,
) -> Result<BTreeSet<String>, String> {
    let mut found = sources_of(reading, symbol, point.vertex)?;
    let Some((closure, _)) = point.closure else {
        return Ok(found);
    };

    let declared = reading.body.symbol(symbol)?;
    let inside = declared
        .declared
        .as_ref()
        .is_some_and(|span| within(span, closure));
    let statement = declared
        .node
        .as_deref()
        .filter(|_| inside)
        .and_then(|node| statement_of(reading, node));

    if let Some(statement) = statement {
        let reads = reads_within(reading, statement)?;
        let pending = reads.iter().map(|read| (*read, point.vertex)).collect();
        found.extend(derived(reading, pending)?);
        for read in reads {
            found.insert(name_of(reading, read)?);
        }
    }
    Ok(found)
}

fn within(span: &FileSpan, outer: &FileSpan) -> bool {
    span.file == outer.file && outer.start <= span.start && span.end <= outer.end
}

/// The variables some write of which reaches the vertex; parameters are written at entry.
fn available_at(reading: &Reading<'_>, vertex: Vertex) -> Result<BTreeSet<String>, String> {
    let mut found = BTreeSet::new();
    for (index, definition) in reading.reaching.definitions.iter().enumerate() {
        if reading.reaching.reaches(index, vertex) {
            found.insert(name_of(reading, definition.symbol)?);
        }
    }
    Ok(found)
}

/// What the writes of a symbol that reach the vertex read: the variables its value was made of.
fn sources_of(
    reading: &Reading<'_>,
    symbol: &str,
    vertex: Vertex,
) -> Result<BTreeSet<String>, String> {
    derived(reading, vec![(symbol, vertex)])
}

/// Every variable the pending values were computed from, through each write before them:
/// `body := read(resp.Body)` then `rdr := wrap(body)` makes `rdr` of `resp` too.
fn derived<'r>(
    reading: &Reading<'r>,
    mut pending: Vec<(&'r str, Vertex)>,
) -> Result<BTreeSet<String>, String> {
    let (mut seen, mut found) = (HashSet::new(), BTreeSet::new());
    while let Some(value) = pending.pop() {
        if !seen.insert(value) {
            continue;
        }
        for (read, at) in made_of(reading, value)? {
            found.insert(name_of(reading, read)?);
            pending.push((read, at));
        }
    }
    Ok(found)
}

/// What the writes of a symbol that reach a vertex read, and where each write is.
fn made_of<'r>(
    reading: &Reading<'r>,
    (symbol, vertex): (&str, Vertex),
) -> Result<Vec<(&'r str, Vertex)>, String> {
    let mut found = Vec::new();
    for (index, definition) in reading.reaching.definitions.iter().enumerate() {
        if definition.symbol != symbol || !reading.reaching.reaches(index, vertex) {
            continue;
        }
        let Some(statement) = statement_of(reading, reading.flow.id(definition.vertex)) else {
            continue;
        };
        let reads = reads_within(reading, statement)?;
        found.extend(reads.into_iter().map(|read| (read, definition.vertex)));
    }
    Ok(found)
}

/// The declaration or assignment a write belongs to.
fn statement_of<'r>(reading: &Reading<'r>, node: &str) -> Option<&'r Node> {
    let mut at = reading.parents.get(node).copied()?;
    loop {
        if matches!(
            at.kind,
            NodeKind::LocalDeclaration
                | NodeKind::Assignment
                | NodeKind::CompoundAssignment
                | NodeKind::DestructuringAssignment
        ) {
            return Some(at);
        }
        at = reading.parents.get(at.id.as_str()).copied()?;
    }
}

fn reads_within<'r>(reading: &Reading<'r>, statement: &Node) -> Result<Vec<&'r str>, String> {
    let mut within = HashSet::new();
    let mut pending = vec![statement.id.as_str()];
    while let Some(id) = pending.pop() {
        if within.insert(id) {
            pending.extend(reading.body.node(id)?.children.iter().map(String::as_str));
        }
    }
    Ok(reading
        .function
        .accesses
        .iter()
        .filter(|access| access.mode != AccessMode::Write && within.contains(access.node.as_str()))
        .map(|access| access.symbol.as_str())
        .collect())
}

/// A method's arguments, where its record says it is.
#[derive(Clone, Debug)]
pub struct MethodArguments {
    pub identity: String,
    pub path: String,
    /// Whether the function says this rule doesn't apply to it. It still counts as a sibling.
    pub allowed: bool,
    pub arguments: Arguments,
}

/// A defect in one of the methods given, by index.
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    pub method: usize,
    pub defect: Defect,
}

/// One argument of one method's call: which method, which of its arguments.
#[derive(Clone, Copy)]
struct Site {
    method: usize,
    argument: usize,
}

/// The arguments the run's sibling methods pass that their siblings say are a slip.
pub fn find(methods: &[MethodArguments], limits: &DeviantArgumentConfig) -> Vec<Finding> {
    let mut found = Vec::new();
    for slots in slots_by_owner(methods).values() {
        for sites in slots.values() {
            found.extend(deviations(methods, sites, limits));
        }
    }

    found.sort_by(|a, b| (a.method, &a.defect).cmp(&(b.method, &b.defect)));
    found.dedup();
    found
}

type Slots<'m> = BTreeMap<(&'m str, usize), Vec<Site>>;

/// Each owner's calls, by callee and position.
fn slots_by_owner(methods: &[MethodArguments]) -> BTreeMap<&str, Slots<'_>> {
    let mut owners: BTreeMap<&str, Slots<'_>> = BTreeMap::new();
    for (method, holder) in methods.iter().enumerate() {
        let Some(owner) = holder.arguments.owner.as_deref() else {
            continue;
        };
        let slots = owners.entry(owner).or_default();
        for (argument, passed) in holder.arguments.arguments.iter().enumerate() {
            slots
                .entry((passed.callee.as_str(), passed.position))
                .or_default()
                .push(Site { method, argument });
        }
    }
    owners
}

fn deviations(
    methods: &[MethodArguments],
    sites: &[Site],
    limits: &DeviantArgumentConfig,
) -> Vec<Finding> {
    let argument = |site: &Site| &methods[site.method].arguments.arguments[site.argument];
    let mut found = Vec::new();
    for deviant in sites {
        let peers: Vec<&Site> = sites
            .iter()
            .filter(|site| siblings(methods, (site.method, deviant.method), limits))
            .collect();
        let Some(majority) = agreed_otherwise(methods, &peers, deviant, limits) else {
            continue;
        };
        let passed = argument(deviant);
        if !is_a_slip(passed, majority, sites, argument) {
            continue;
        }
        let total = peers.len();
        found.push(Finding {
            method: deviant.method,
            defect: Defect {
                rule: RULE,
                line: passed.line,
                message: message(passed, &majority.text, (total - 1, total)),
            },
        });
    }
    found
}

pub fn message(passed: &Argument, majority: &str, (agreeing, total): (usize, usize)) -> String {
    format!(
        "`{}` is passed `{}` here, where {agreeing} of its {total} calls in sibling methods pass `{majority}`",
        passed.name, passed.text
    )
}

/// The argument every peer but the deviant passes, when the peers span enough methods.
fn agreed_otherwise<'m>(
    methods: &'m [MethodArguments],
    peers: &[&Site],
    deviant: &Site,
    limits: &DeviantArgumentConfig,
) -> Option<&'m Argument> {
    let argument = |site: &Site| &methods[site.method].arguments.arguments[site.argument];
    let value = &argument(deviant).value;
    let others: Vec<&Argument> = peers
        .iter()
        .filter(|site| site.method != deviant.method || site.argument != deviant.argument)
        .map(|site| argument(site))
        .collect();
    let majority = others.first()?;
    let agree = others.iter().all(|other| other.value == majority.value);
    let methods_calling: BTreeSet<usize> = peers.iter().map(|site| site.method).collect();

    let enough = methods_calling.len() >= limits.min_methods.0;
    (agree && enough && majority.value != *value).then_some(*majority)
}

/// Whether nothing but a slip explains the difference: both read a variable, a sibling makes the
/// same call but for that argument, with one of the same type, the siblings' variable holds a value
/// at the call, and the passed one isn't made of it.
fn is_a_slip<'m>(
    passed: &Argument,
    majority: &Argument,
    sites: &[Site],
    argument: impl Fn(&Site) -> &'m Argument,
) -> bool {
    let (Some(_), Some(expected)) = (&passed.variable, &majority.variable) else {
        return false;
    };
    let twin = sites.iter().map(argument).any(|other| {
        other.value == majority.value
            && other.ty == passed.ty
            && differs_only_at(&other.call, &passed.call, passed.position)
    });
    twin && passed.available.contains(expected) && !passed.sources.contains(expected)
}

fn differs_only_at(a: &[String], b: &[String], position: usize) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .enumerate()
            .all(|(index, (a, b))| index == position || a == b)
}

/// Whether two methods call enough of the same callees to be written as copies of each other.
fn siblings(
    methods: &[MethodArguments],
    (a, b): (usize, usize),
    limits: &DeviantArgumentConfig,
) -> bool {
    if a == b {
        return true;
    }
    let (a, b) = (&methods[a].arguments.callees, &methods[b].arguments.callees);
    let shared = a.intersection(b).count();

    let either = a.union(b).count();
    either > 0 && shared * 100 >= either * limits.min_shared_callees_percent.0
}

#[cfg(test)]
mod tests;
