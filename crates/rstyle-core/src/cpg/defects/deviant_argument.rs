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
use super::super::model::{
    AccessMode, Dispatch, FileSpan, Node, NodeKind, Origin, TargetKind, Type,
};
use super::ignored_result::{declared_callee, pointed_at};
use super::{Defect, Reading};
use crate::config::DeviantArgumentConfig;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

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
    /// The field the argument reads that variable through first, as `Kind` of `options.Kind`.
    pub field: Option<String>,
    /// The variables that hold a value when the call runs, with each one's type when known.
    pub available: BTreeMap<String, Option<String>>,
    /// The variables the argument's own variable was computed from in this function.
    pub sources: BTreeSet<String>,
    /// A branch chooses whether this call runs.
    pub conditional: bool,
}

/// A method's arguments, and the callees it calls, which say how alike two methods are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arguments {
    /// The type the function is a method of; a free function has none and is never compared.
    pub owner: Option<String>,
    pub callees: BTreeSet<String>,
    pub arguments: Vec<Argument>,
    /// Each field the function reads, and of what type: evidence of which types have which fields,
    /// for a frontend that doesn't list them.
    pub fields: BTreeSet<(String, String)>,
    /// The fields selectable through each of its variables' types, where the frontend lists them.
    pub declared: BTreeMap<String, BTreeSet<String>>,
}

pub(super) fn of(reading: &Reading<'_>) -> Result<Arguments, String> {
    let Some(owner) = reading.function.signature.owner_type.as_deref() else {
        return Ok(Arguments::default());
    };
    if test_suite(reading, reading.tables.ty(owner)?)? {
        return Ok(Arguments::default());
    }
    let mut found = Arguments {
        owner: Some(reading.tables.ty(owner)?.spelling.clone()),
        fields: fields_read(reading)?,
        declared: declared_fields(reading)?,
        ..Arguments::default()
    };

    let mut lineage = Lineage::new(reading);
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
        found
            .arguments
            .extend(arguments_of(&mut lineage, node, callee)?);
    }
    Ok(found)
}

/// The fields the function reads, with the type each is read from, behind any pointer.
fn fields_read(reading: &Reading<'_>) -> Result<BTreeSet<(String, String)>, String> {
    let mut found = BTreeSet::new();
    for node in &reading.function.nodes {
        let (NodeKind::FieldAccess, Some(member), Some(base)) =
            (node.kind, node.member.as_deref(), node.children.first())
        else {
            continue;
        };
        if let Some(ty) = reading.body.node(base)?.ty.as_deref() {
            found.insert((spelled(reading, ty)?, member.to_owned()));
        }
    }
    Ok(found)
}

/// The fields each variable's type lists, with those its embedded fields promote.
fn declared_fields(reading: &Reading<'_>) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut found = BTreeMap::new();
    for symbol in &reading.function.symbols {
        let Some(ty) = symbol.ty.as_deref() else {
            continue;
        };
        let record = pointed_at(reading.tables, reading.tables.ty(ty)?)?;
        if record.fields.is_some() && !found.contains_key(&record.spelling) {
            found.insert(record.spelling.clone(), selectable(reading, record)?);
        }
    }
    Ok(found)
}

/// How deep embedded fields are followed. Far more than a type nests them.
const EMBEDDING_DEPTH: usize = 16;

/// Testify runs methods of its embedded Suite even when a shared suite lives in a `.go` file.
/// Its test cases intentionally vary inputs; they aren't production sibling implementations.
fn test_suite(reading: &Reading<'_>, owner: &Type) -> Result<bool, String> {
    let (mut pending, mut seen) = (vec![(owner, 0)], HashSet::new());
    while let Some((record, depth)) = pending.pop() {
        let record = pointed_at(reading.tables, record)?;
        if depth > EMBEDDING_DEPTH || !seen.insert(record.id.as_str()) {
            continue;
        }
        if record.spelling == "github.com/stretchr/testify/suite.Suite" {
            return Ok(true);
        }
        for field in record
            .fields
            .iter()
            .flatten()
            .filter(|field| field.embedded)
        {
            pending.push((reading.tables.ty(&field.ty)?, depth + 1));
        }
    }
    Ok(false)
}

/// A record's field names, and those of every embedded field's type, as Go selects them.
fn selectable(reading: &Reading<'_>, record: &Type) -> Result<BTreeSet<String>, String> {
    let (mut found, mut pending, mut seen) = (BTreeSet::new(), vec![(record, 0)], HashSet::new());
    while let Some((record, depth)) = pending.pop() {
        if depth > EMBEDDING_DEPTH || !seen.insert(record.id.as_str()) {
            continue;
        }
        for field in record.fields.iter().flatten() {
            found.insert(field.name.clone());
            if field.embedded {
                let inner = pointed_at(reading.tables, reading.tables.ty(&field.ty)?)?;
                pending.push((inner, depth + 1));
            }
        }
    }
    Ok(found)
}

/// A type as spelled once any pointers or references to it are followed.
fn spelled(reading: &Reading<'_>, ty: &str) -> Result<String, String> {
    let tables = reading.tables;
    Ok(pointed_at(tables, tables.ty(ty)?)?.spelling.clone())
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
    lineage: &mut Lineage<'_, '_>,
    call: &Node,
    (callee, name): (&str, &str),
) -> Result<Vec<Argument>, String> {
    let reading = lineage.reading;
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
    let available = available(reading, &point)?;
    let (line, _) = reading.sources.location(span)?;

    let mut found = Vec::new();
    for (position, argument) in passed.iter().enumerate() {
        let (variable, field) = variable_of(reading, argument)?;
        let sources = match &variable {
            Some((symbol, _)) => sources(lineage, symbol, &point)?,
            None => BTreeSet::new(),
        };
        found.push(Argument {
            callee: callee.to_owned(),
            name: name.to_owned(),
            position,
            line,
            value: call_values[position].clone(),
            text: source_text(reading, argument)?,
            ty: argument
                .ty
                .as_deref()
                .map(|ty| reading.tables.ty(ty).map(|ty| ty.spelling.clone()))
                .transpose()?,
            call: call_values.clone(),
            variable: variable.map(|(_, name)| name),
            field,
            available: available.clone(),
            sources,
            conditional: conditional(reading, call),
        });
    }
    Ok(found)
}

fn conditional(reading: &Reading<'_>, call: &Node) -> bool {
    let mut at = reading.parents.get(call.id.as_str()).copied();
    while let Some(node) = at {
        if matches!(node.kind, NodeKind::If | NodeKind::MatchArm) {
            return true;
        }
        at = reading.parents.get(node.id.as_str()).copied();
    }
    false
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

/// The variable an argument reads, through fields, indexes and references to it, and the field
/// it's read through first.
type Read = (Option<(String, String)>, Option<String>);

fn variable_of(reading: &Reading<'_>, node: &Node) -> Result<Read, String> {
    let (mut at, mut field) = (node, None);
    loop {
        match at.kind {
            NodeKind::FieldAccess | NodeKind::IndexAccess | NodeKind::Reference => {
                if at.kind == NodeKind::FieldAccess {
                    field = at.member.clone();
                }
                let Some(base) = at.children.first() else {
                    return Ok((None, None));
                };
                at = reading.body.node(base)?;
            }
            NodeKind::Identifier => return Ok((symbol_read(reading, at)?, field)),
            _ => return Ok((None, None)),
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
fn available(reading: &Reading<'_>, point: &Point<'_>) -> Result<Variables, String> {
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
            found.insert(name_of(reading, &symbol.id)?, type_of(reading, &symbol.id)?);
        }
    }
    Ok(found)
}

/// What a variable's value at the point was made of. Inside a closure the flow doesn't enter,
/// that's what the statement declaring it reads.
fn sources(
    lineage: &mut Lineage<'_, '_>,
    symbol: &str,
    point: &Point<'_>,
) -> Result<BTreeSet<String>, String> {
    let reading = lineage.reading;
    let mut found = sources_of(lineage, symbol, point.vertex)?;
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
        let reads = lineage.reads_within(statement)?;
        let reads: Vec<&str> = reads.iter().map(|&read| lineage.symbols[read]).collect();
        let pending: Vec<_> = reads.iter().map(|&read| (read, point.vertex)).collect();
        found.extend(derived(lineage, &pending)?);
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
fn available_at(reading: &Reading<'_>, vertex: Vertex) -> Result<Variables, String> {
    let mut found = Variables::new();
    for (index, definition) in reading.reaching.definitions.iter().enumerate() {
        if reading.reaching.reaches(index, vertex) {
            let symbol = definition.symbol;
            found.insert(name_of(reading, symbol)?, type_of(reading, symbol)?);
        }
    }
    Ok(found)
}

/// Variables by name, each with its type when the frontend gives one.
type Variables = BTreeMap<String, Option<String>>;

fn type_of(reading: &Reading<'_>, symbol: &str) -> Result<Option<String>, String> {
    let ty = reading.body.symbol(symbol)?.ty.as_deref();
    ty.map(|ty| spelled(reading, ty)).transpose()
}

/// What the writes of a symbol that reach the vertex read: the variables its value was made of.
/// A symbol nothing writes was made of nothing here.
fn sources_of(
    lineage: &mut Lineage<'_, '_>,
    symbol: &str,
    vertex: Vertex,
) -> Result<BTreeSet<String>, String> {
    let Some((&symbol, _)) = lineage.definitions.get_key_value(symbol) else {
        return Ok(BTreeSet::new());
    };
    derived(lineage, &[(symbol, vertex)])
}

/// Every variable the pending values were computed from, through each write before them:
/// `body := read(resp.Body)` then `rdr := wrap(body)` makes `rdr` of `resp` too.
fn derived<'r>(
    lineage: &mut Lineage<'_, 'r>,
    pending: &[(&'r str, Vertex)],
) -> Result<BTreeSet<String>, String> {
    let mut behind = Vec::new();
    for &value in pending {
        let value = lineage.value(value);
        union(&mut behind, &lineage.behind(value)?);
    }
    let reading = lineage.reading;
    members(&behind)
        .map(|symbol| name_of(reading, lineage.symbols[symbol]))
        .collect()
}

/// A symbol a write reads, and that symbol's value at the write, both by number.
type Step = (usize, usize);

/// A symbol's value at a vertex, and what it was made of once that's worked out.
struct Value<'r> {
    symbol: &'r str,
    vertex: Vertex,
    /// What the writes behind it read.
    steps: Option<Rc<[Step]>>,
    /// Every symbol behind it, once its strongly connected component is worked out.
    behind: Option<Rc<[u64]>>,
}

/// What a function's values were made of, worked out once for the whole function. Every argument
/// of every call asks after the same writes, and a long function has thousands of both, so each
/// value's sources are found once rather than walked again for each argument.
struct Lineage<'l, 'r> {
    reading: &'l Reading<'r>,
    /// Each symbol's definitions, by index, in the order the reaching definitions list them.
    definitions: HashMap<&'r str, Vec<usize>>,
    /// Each symbol a statement reads, numbered for the sets of them.
    symbols: Vec<&'r str>,
    numbers: HashMap<&'r str, usize>,
    /// What each declaration or assignment reads, by its node.
    reads: HashMap<&'r str, Rc<[usize]>>,
    /// Each value asked after, numbered.
    values: Vec<Value<'r>>,
    numbered: HashMap<(&'r str, Vertex), usize>,
}

impl<'l, 'r> Lineage<'l, 'r> {
    fn new(reading: &'l Reading<'r>) -> Self {
        let mut definitions: HashMap<&'r str, Vec<usize>> = HashMap::new();
        for (index, definition) in reading.reaching.definitions.iter().enumerate() {
            definitions
                .entry(definition.symbol)
                .or_default()
                .push(index);
        }
        Self {
            reading,
            definitions,
            symbols: Vec::new(),
            numbers: HashMap::new(),
            reads: HashMap::new(),
            values: Vec::new(),
            numbered: HashMap::new(),
        }
    }

    fn number(&mut self, symbol: &'r str) -> usize {
        let next = self.symbols.len();
        let number = *self.numbers.entry(symbol).or_insert(next);
        if number == next {
            self.symbols.push(symbol);
        }
        number
    }

    fn value(&mut self, (symbol, vertex): (&'r str, Vertex)) -> usize {
        let next = self.values.len();
        let number = *self.numbered.entry((symbol, vertex)).or_insert(next);
        if number == next {
            self.values.push(Value {
                symbol,
                vertex,
                steps: None,
                behind: None,
            });
        }
        number
    }

    fn reads_within(&mut self, statement: &'r Node) -> Result<Rc<[usize]>, String> {
        if let Some(found) = self.reads.get(statement.id.as_str()) {
            return Ok(Rc::clone(found));
        }
        let reads = reads_within(self.reading, statement)?;
        let found: Rc<[usize]> = reads.into_iter().map(|read| self.number(read)).collect();
        self.reads.insert(statement.id.as_str(), Rc::clone(&found));
        Ok(found)
    }

    /// What the writes of a value's symbol that reach its vertex read, and where each write is.
    fn steps(&mut self, value: usize) -> Result<Rc<[Step]>, String> {
        if let Some(found) = &self.values[value].steps {
            return Ok(Rc::clone(found));
        }
        let reading = self.reading;
        let (symbol, vertex) = (self.values[value].symbol, self.values[value].vertex);
        let mut found = Vec::new();
        let indices = self.definitions.get(symbol).cloned().unwrap_or_default();
        for index in indices {
            if !reading.reaching.reaches(index, vertex) {
                continue;
            }
            let written = reading.reaching.definitions[index].vertex;
            let Some(statement) = statement_of(reading, reading.flow.id(written)) else {
                continue;
            };
            for &read in self.reads_within(statement)?.iter() {
                let at = self.value((self.symbols[read], written));
                found.push((read, at));
            }
        }
        let found: Rc<[Step]> = found.into();
        self.values[value].steps = Some(Rc::clone(&found));
        Ok(found)
    }

    /// Every symbol a value was made of, by Tarjan's strongly connected components over the values
    /// it reaches that no earlier search finished. A loop's writes make each other, so every value
    /// in one component was made of the same symbols.
    fn behind(&mut self, root: usize) -> Result<Rc<[u64]>, String> {
        if let Some(done) = &self.values[root].behind {
            return Ok(Rc::clone(done));
        }
        let mut search = Search::from(root, self.steps(root)?);
        while let Some(next) = search.next(|at| self.values[at].behind.is_some()) {
            match next {
                Next::Enter(value) => search.enter(value, self.steps(value)?),
                Next::Close(members) => self.finish(&members)?,
            }
        }
        self.values[root]
            .behind
            .clone()
            .ok_or_else(|| "a value's search ended unfinished".to_owned())
    }

    /// Every symbol a component's values were made of: what its writes read, and everything
    /// behind each value outside it that they read.
    fn finish(&mut self, members: &[usize]) -> Result<(), String> {
        let inside: HashSet<usize> = members.iter().copied().collect();
        let mut behind = vec![0; self.symbols.len().div_ceil(64)];
        for &value in members {
            for &(symbol, at) in self.steps(value)?.iter() {
                behind[symbol / 64] |= 1 << (symbol % 64);
                if !inside.contains(&at) {
                    let outside = self.values[at].behind.as_deref();
                    union(&mut behind, outside.unwrap_or_default());
                }
            }
        }
        let behind: Rc<[u64]> = behind.into();
        for &value in members {
            self.values[value].behind = Some(Rc::clone(&behind));
        }
        Ok(())
    }
}

/// What a search asks of the lineage next: a value's steps, or a finished component's symbols.
enum Next {
    Enter(usize),
    Close(Vec<usize>),
}

/// A value the search is in, and which of its steps it takes next.
struct Frame {
    value: usize,
    steps: Rc<[Step]>,
    next: usize,
}

impl Frame {
    fn at(value: usize, steps: Rc<[Step]>) -> Self {
        Self {
            value,
            steps,
            next: 0,
        }
    }
}

/// When a search first met a value, the earliest open value it reaches, and whether it's still
/// open: not yet in a finished component.
#[derive(Clone, Copy)]
struct Mark {
    order: usize,
    low: usize,
    open: bool,
}

impl Mark {
    /// A value met as the search's `order`th, open and reaching nothing earlier yet.
    fn first(order: usize) -> Self {
        Self {
            order,
            low: order,
            open: true,
        }
    }
}

/// One search for strongly connected components, without recursion, since a function's values
/// can chain deeper than a thread's stack: the path it's on, the values it's met, and those open.
struct Search {
    path: Vec<Frame>,
    met: HashMap<usize, Mark>,
    open: Vec<usize>,
}

impl Search {
    fn from(root: usize, steps: Rc<[Step]>) -> Self {
        let mut search = Self {
            path: Vec::new(),
            met: HashMap::new(),
            open: Vec::new(),
        };
        search.enter(root, steps);
        search
    }

    fn enter(&mut self, value: usize, steps: Rc<[Step]>) {
        self.met.insert(value, Mark::first(self.met.len()));
        self.open.push(value);
        self.path.push(Frame::at(value, steps));
    }

    /// The next value to enter or component to close, passing over values an earlier search
    /// finished.
    fn next(&mut self, finished: impl Fn(usize) -> bool) -> Option<Next> {
        loop {
            let frame = self.path.last_mut()?;
            let (value, step) = (frame.value, frame.steps.get(frame.next).copied());
            frame.next += 1;
            let Some((_, at)) = step else {
                match self.leave(value) {
                    Some(members) => return Some(Next::Close(members)),
                    None => continue,
                }
            };
            match self.met.get(&at).copied() {
                _ if finished(at) => {}
                None => return Some(Next::Enter(at)),
                Some(mark) if mark.open => self.lower(value, mark.order),
                Some(_) => {}
            }
        }
    }

    /// Leaves a value whose steps are all taken, with the component it roots if it roots one.
    fn leave(&mut self, value: usize) -> Option<Vec<usize>> {
        self.path.pop();
        let mark = *self.met.get(&value)?;
        if let Some(parent) = self.path.last().map(|frame| frame.value) {
            self.lower(parent, mark.low);
        }
        (mark.low == mark.order).then(|| self.close(value))
    }

    fn lower(&mut self, value: usize, to: usize) {
        if let Some(mark) = self.met.get_mut(&value) {
            mark.low = mark.low.min(to);
        }
    }

    /// The component a value roots: it and every value opened after it that's still open.
    fn close(&mut self, root: usize) -> Vec<usize> {
        let mut members = Vec::new();
        while let Some(value) = self.open.pop() {
            if let Some(mark) = self.met.get_mut(&value) {
                mark.open = false;
            }
            members.push(value);
            if value == root {
                break;
            }
        }
        members
    }
}

fn union(into: &mut Vec<u64>, from: &[u64]) {
    if into.len() < from.len() {
        into.resize(from.len(), 0);
    }
    for (word, more) in into.iter_mut().zip(from) {
        *word |= more;
    }
}

fn members(bits: &[u64]) -> impl Iterator<Item = usize> + '_ {
    bits.iter().enumerate().flat_map(|(word, &bits)| {
        (0..64)
            .filter(move |bit| bits & (1 << bit) != 0)
            .map(move |bit| word * 64 + bit)
    })
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
    let known = known_fields(methods);
    let mut found = Vec::new();
    for slots in slots_by_owner(methods).values() {
        for sites in slots.values() {
            found.extend(deviations(methods, sites, limits, &known));
        }
    }

    found.sort_by(|a, b| (a.method, &a.defect).cmp(&(b.method, &b.defect)));
    found.dedup();
    found
}

/// What the run says of which types have which fields: what frontends list, and, for the types
/// they don't, what the run reads from them.
#[derive(Default)]
struct Fields<'m> {
    declared: BTreeMap<&'m str, &'m BTreeSet<String>>,
    read: BTreeSet<(&'m str, &'m str)>,
}

impl Fields<'_> {
    fn has(&self, ty: &str, field: &str) -> bool {
        match self.declared.get(ty) {
            Some(declared) => declared.contains(field),
            None => self.read.contains(&(ty, field)),
        }
    }
}

fn known_fields(methods: &[MethodArguments]) -> Fields<'_> {
    let mut known = Fields::default();
    for method in methods {
        let arguments = &method.arguments;
        known.declared.extend(
            arguments
                .declared
                .iter()
                .map(|(ty, fields)| (ty.as_str(), fields)),
        );
        known.read.extend(
            arguments
                .fields
                .iter()
                .map(|(ty, field)| (ty.as_str(), field.as_str())),
        );
    }
    known
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
    known: &Fields<'_>,
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
        if !is_a_slip((passed, majority), (sites, argument), known) {
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
    // A method that passes both values supplies its own counterexample to the sibling pattern.
    // Different branches can deliberately parse different tokens returned by the same scanner.
    if argument(deviant).conditional
        && peers
            .iter()
            .any(|site| site.method == deviant.method && argument(site).value == majority.value)
    {
        return None;
    }
    let agree = others.iter().all(|other| other.value == majority.value);
    let methods_calling: BTreeSet<usize> = peers.iter().map(|site| site.method).collect();

    let enough = methods_calling.len() >= limits.min_methods.0;
    (agree && enough && majority.value != *value).then_some(*majority)
}

/// Whether nothing but a slip explains the difference: both read a variable, a sibling makes the
/// same call but for that argument, with one of the same type, the siblings' value could be written
/// here, and the passed one isn't made of it.
fn is_a_slip<'m>(
    (passed, majority): (&Argument, &Argument),
    (sites, argument): (&[Site], impl Fn(&Site) -> &'m Argument),
    known: &Fields<'_>,
) -> bool {
    let (Some(_), Some(expected)) = (&passed.variable, &majority.variable) else {
        return false;
    };
    let twin = sites.iter().map(argument).any(|other| {
        other.value == majority.value
            && other.ty == passed.ty
            && differs_only_at(&other.call, &passed.call, passed.position)
    });
    twin && writable_here(passed, (expected, majority.field.as_deref()), known)
        && !passed.sources.contains(expected)
}

/// Whether the siblings' value could stand in the passed one's place: its variable holds a value
/// at the call, and the field it's read through is one that variable's type has. That's what the
/// frontend lists for the type, or else what the run reads from it, where a field nothing reads is
/// taken as absent.
fn writable_here(
    passed: &Argument,
    (variable, field): (&str, Option<&str>),
    known: &Fields<'_>,
) -> bool {
    let Some(ty) = passed.available.get(variable) else {
        return false;
    };
    let Some(field) = field else {
        return true;
    };
    ty.as_deref().is_some_and(|ty| known.has(ty, field))
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

#[cfg(test)]
mod reading_tests;
