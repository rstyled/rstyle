//! A condition that changes nothing, because the code it guards is the code that runs without
//! it. Either both branches are the same code, or the branch returns exactly as the code after
//! it does:
//!
//! ```go
//! if enabled {
//!     return false
//! }
//! return false
//! ```
//!
//! Two pieces of code are the same when they're written with the same tokens and every name in
//! them resolves to the same thing. So a name the condition binds for its branch isn't the name
//! after it, however alike they're spelled.

use super::super::model::{
    Access, AccessMode, Dispatch, FileSpan, Function, Node, NodeKind, Origin, RefStatus, RefTarget,
    TargetKind,
};
use super::super::tree;
use super::{Defect, Reading};
use std::collections::HashMap;

pub(super) const RULE: &str = "identical_branches";

const BOTH: &str = "the condition here changes nothing: both branches run the same code";
const FOLLOWS: &str =
    "the condition here changes nothing: the branch runs the same code as what follows it";

pub(super) fn find(reading: &Reading<'_>) -> Result<Vec<Defect>, String> {
    let index = Index::of(reading.function);
    let mut found = Vec::new();
    for node in &reading.function.nodes {
        if node.kind != NodeKind::If || node.provenance.origin != Origin::Source {
            continue;
        }
        let (taken, other, message) = ways(reading, node)?;
        if taken.is_empty() || !Twins::new(reading, &index).same(&taken, &other)? {
            continue;
        }
        if !guard_returns_input(reading, node, &taken)?
            && !todo_placeholder(reading, node, &taken, &other)?
        {
            found.push(Defect {
                rule: RULE,
                line: tree::line(reading.sources, node)?,
                message: message.into(),
            });
        }
    }
    Ok(found)
}

/// A plain return of the binding the guard checks against Go's builtin nil is
/// routine error propagation, even when the check is redundant.
fn guard_returns_input(
    reading: &Reading<'_>,
    branch: &Node,
    taken: &[&Node],
) -> Result<bool, String> {
    let condition = reading.body.child(branch, 0)?;
    if !matches!(condition.operator.as_deref(), Some("eq" | "ne")) {
        return Ok(false);
    }
    let left = reading.body.child(condition, 0)?;
    let right = reading.body.child(condition, 1)?;
    let checked = if builtin_nil(reading, right) {
        left
    } else if builtin_nil(reading, left) {
        right
    } else {
        return Ok(false);
    };
    let Some(symbol) = reading
        .body
        .reference(&checked.id)
        .and_then(|r| r.target.as_ref())
    else {
        return Ok(false);
    };
    if symbol.kind != TargetKind::Symbol || symbol.id.is_none() {
        return Ok(false);
    }
    let Some(returned) = plain_return(reading, taken)? else {
        return Ok(false);
    };
    let values = return_values(reading, returned)?;
    if !plain_values(&values) {
        return Ok(false);
    }
    Ok(values.iter().any(|value| {
        reading
            .body
            .reference(&value.id)
            .and_then(|r| r.target.as_ref())
            .is_some_and(|target| target.kind == TargetKind::Symbol && target.id == symbol.id)
    }))
}

fn plain_values(values: &[&Node]) -> bool {
    values
        .iter()
        .all(|value| matches!(value.kind, NodeKind::Identifier | NodeKind::Literal))
}

/// Frontends may wrap multiple return values in a tuple.
fn return_values<'a>(reading: &Reading<'a>, returned: &Node) -> Result<Vec<&'a Node>, String> {
    let values = reading.body.children(returned)?;
    if let [tuple] = values.as_slice()
        && tuple.kind == NodeKind::Tuple
    {
        return reading.body.children(tuple);
    }
    Ok(values)
}

fn builtin_nil(reading: &Reading<'_>, node: &Node) -> bool {
    reading.body.reference(&node.id).is_some_and(|reference| {
        reference.status == RefStatus::Resolved
            && reference.target.as_ref().is_some_and(|target| {
                target.kind == TargetKind::Definition && target.id.as_deref() == Some("def:go::nil")
            })
    })
}

/// Only a single return is a placeholder; duplicated work still deserves a report.
fn plain_return<'a>(reading: &Reading<'a>, taken: &[&'a Node]) -> Result<Option<&'a Node>, String> {
    let [statement] = taken else {
        return Ok(None);
    };
    let node = if statement.kind == NodeKind::ExpressionStatement {
        reading.body.child(statement, 0)?
    } else {
        statement
    };
    Ok((node.kind == NodeKind::Return).then_some(node))
}

/// An explicit TODO between a guard and its repeated return marks unfinished code.
/// Search only that gap, so unrelated TODOs elsewhere cannot hide a defect.
fn todo_placeholder(
    reading: &Reading<'_>,
    branch: &Node,
    taken: &[&Node],
    other: &[&Node],
) -> Result<bool, String> {
    if branch.children.len() != 2 {
        return Ok(false);
    }
    let Some(returned) = plain_return(reading, taken)? else {
        return Ok(false);
    };
    if !plain_values(&return_values(reading, returned)?) {
        return Ok(false);
    }
    let spans = branch
        .span
        .as_ref()
        .zip(other.first().and_then(|node| node.span.as_ref()));
    let Some((branch, after)) = spans else {
        return Ok(false);
    };
    let gap = FileSpan {
        file: branch.file.clone(),
        start: branch.end,
        end: after.start,
    };
    let text = reading.sources.between(&gap, &gap)?;
    Ok(text.lines().any(|line| {
        let comment = line
            .trim()
            .strip_prefix("//")
            .or_else(|| line.trim().strip_prefix('#'));
        comment.is_some_and(|comment| {
            comment.trim().strip_prefix("TODO").is_some_and(|rest| {
                rest.chars()
                    .next()
                    .is_none_or(|c| !c.is_alphanumeric() && c != '_')
            })
        })
    }))
}

/// A function's reads and writes by node, and its locals by the node that declares each.
struct Index<'a> {
    accesses: HashMap<&'a str, Vec<&'a Access>>,
    declared: HashMap<&'a str, &'a str>,
}

impl<'a> Index<'a> {
    fn of(function: &'a Function) -> Self {
        let mut accesses: HashMap<&str, Vec<&Access>> = HashMap::new();
        for access in &function.accesses {
            accesses.entry(&access.node).or_default().push(access);
        }
        let locals = function.symbols.iter();
        let declared =
            locals.filter_map(|symbol| Some((symbol.node.as_deref()?, symbol.id.as_str())));
        Self {
            accesses,
            declared: declared.collect(),
        }
    }
}

/// The statements a condition runs when it holds and the ones it runs when it doesn't: its two
/// branches, or a branch that returns and as many statements after it. Nothing runs when it
/// doesn't hold where that can't be told.
type Ways<'a> = (Vec<&'a Node>, Vec<&'a Node>, &'static str);

fn ways<'a>(reading: &Reading<'a>, branch: &Node) -> Result<Ways<'a>, String> {
    let taken = statements(reading, reading.body.child(branch, 1)?)?;
    if let Some(otherwise) = branch.children.get(2) {
        let other = statements(reading, reading.body.node(otherwise)?)?;
        return Ok((taken, other, BOTH));
    }

    let mut after = Vec::new();
    if returns(reading, &taken)? {
        after = following(reading, branch)?;
        after.truncate(taken.len());
    }
    Ok((taken, after, FOLLOWS))
}

/// What a branch runs: a block's statements, or the one statement written without a block.
fn statements<'a>(reading: &Reading<'a>, node: &'a Node) -> Result<Vec<&'a Node>, String> {
    match node.kind {
        NodeKind::Block => reading.body.children(node),
        _ => Ok(vec![node]),
    }
}

/// Whether a branch ends by returning, bare or as the statement that wraps a return.
fn returns(reading: &Reading<'_>, taken: &[&Node]) -> Result<bool, String> {
    let Some(&last) = taken.last() else {
        return Ok(false);
    };
    let wrapped = reading.body.children(last)?;
    let returning = |node: &&Node| node.kind == NodeKind::Return;
    let wraps_one =
        last.kind == NodeKind::ExpressionStatement && wrapped.last().is_some_and(returning);
    Ok(returning(&last) || wraps_one)
}

/// The statements after a condition in the block that holds it, which is none where a block
/// doesn't hold it, as for an `else if`.
fn following<'a>(reading: &Reading<'a>, branch: &Node) -> Result<Vec<&'a Node>, String> {
    let parent = |id: &str| reading.parents.get(id).copied();
    let wrapper = parent(&branch.id).filter(|node| node.kind == NodeKind::ExpressionStatement);
    let statement = wrapper.map_or(branch.id.as_str(), |node| node.id.as_str());
    let block = parent(statement).filter(|node| node.kind == NodeKind::Block);

    let siblings = block.into_iter().flat_map(|block| &block.children);
    let after = siblings.skip_while(|child| *child != statement).skip(1);
    after.map(|id| reading.body.node(id)).collect()
}

/// What a node reads and writes, and what it refers to.
type Names<'a> = (Vec<(AccessMode, &'a str)>, Option<Named<'a>>);

/// A reference: how it resolved, the closure it makes, what it names and what it selects.
type Named<'a> = (
    RefStatus,
    Option<Dispatch>,
    Option<&'a str>,
    Target<'a>,
    Target<'a>,
);

/// What a reference names: its kind, its ID and its operation.
type Target<'a> = Option<(TargetKind, Option<&'a str>, Option<&'a str>)>;

/// The nodes of two pieces of code, matched up as both are walked together.
struct Twins<'r, 'a> {
    reading: &'r Reading<'a>,
    index: &'r Index<'a>,
    twin: HashMap<&'a str, &'a str>,
}

impl<'r, 'a> Twins<'r, 'a> {
    fn new(reading: &'r Reading<'a>, index: &'r Index<'a>) -> Self {
        Self {
            reading,
            index,
            twin: HashMap::new(),
        }
    }

    /// Whether two runs of statements are written alike and mean the same.
    fn same(mut self, left: &[&'a Node], right: &[&'a Node]) -> Result<bool, String> {
        if left.len() != right.len() {
            return Ok(false);
        }
        for (&left, &right) in left.iter().zip(right) {
            if !self.written_alike(left, right)? {
                return Ok(false);
            }
        }

        for (&left, &right) in &self.twin {
            if self.names(left, true)? != self.names(right, false)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether two trees have one shape and one set of tokens, pairing their nodes as it goes.
    /// Code a macro wrote has no tokens of its own to compare.
    fn written_alike(&mut self, left: &'a Node, right: &'a Node) -> Result<bool, String> {
        let written = |node: &Node| node.provenance.origin != Origin::MacroExpansion;
        let alike = left.kind == right.kind
            && left.children.len() == right.children.len()
            && written(left)
            && written(right)
            && self.own_text(left)? == self.own_text(right)?;
        if !alike {
            return Ok(false);
        }

        self.twin.insert(&left.id, &right.id);
        let body = &self.reading.body;
        let children = body.children(left)?.into_iter().zip(body.children(right)?);
        for (left, right) in children {
            if !self.written_alike(left, right)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// What a node is written with apart from its children, such as its keyword, its operator
    /// or its name, with no whitespace. A node a lowering made up is written with nothing.
    fn own_text(&self, node: &Node) -> Result<String, String> {
        let Some(span) = node.span.as_ref() else {
            return Ok(String::new());
        };
        let mut inside: Vec<&FileSpan> = Vec::new();
        for child in self.reading.body.children(node)? {
            inside.extend(child.span.as_ref().filter(|it| within(it, span)));
        }
        inside.sort_by_key(|child| child.start);

        let (mut text, mut at) = (String::new(), span.start);
        for child in inside {
            text.push_str(self.text(span, at, child.start.max(at))?);
            at = child.end.max(at);
        }
        text.push_str(self.text(span, at, span.end)?);
        text.retain(|character| !character.is_whitespace());
        Ok(text)
    }

    fn text(&self, span: &FileSpan, start: usize, end: usize) -> Result<&'a str, String> {
        let range = FileSpan {
            file: span.file.clone(),
            start,
            end,
        };
        self.reading.sources.between(&range, &range)
    }

    /// What a node reads, writes and refers to, in terms both pieces of code share. A closure
    /// is another function, named by its own ID, so two of them are never the same.
    fn names(&self, node: &'a str, left: bool) -> Result<Names<'a>, String> {
        let mut accesses = Vec::new();
        for access in self.index.accesses.get(node).into_iter().flatten() {
            accesses.push((access.mode, self.shared(&access.symbol, left)?));
        }
        let Some(reference) = self.reading.body.reference(node) else {
            return Ok((accesses, None));
        };

        let named = (
            reference.status,
            reference.dispatch,
            reference.closure.as_deref(),
            self.target(reference.target.as_ref(), left)?,
            self.target(reference.selected.as_ref(), left)?,
        );
        Ok((accesses, Some(named)))
    }

    fn target(&self, target: Option<&'a RefTarget>, left: bool) -> Result<Target<'a>, String> {
        let Some(target) = target else {
            return Ok(None);
        };
        let id = match (target.kind, target.id.as_deref()) {
            (TargetKind::Symbol, Some(symbol)) => Some(self.shared(symbol, left)?),
            (_, id) => id,
        };
        Ok(Some((target.kind, id, target.operation.as_deref())))
    }

    /// A binding as both pieces of code know it: a local the left one declares for itself is
    /// the right one's copy of it.
    fn shared(&self, symbol: &'a str, left: bool) -> Result<&'a str, String> {
        let declared = self.reading.body.symbol(symbol)?.node.as_deref();
        let twin = declared
            .filter(|_| left)
            .and_then(|node| self.twin.get(node));
        let copy = twin.and_then(|node| self.index.declared.get(node));
        Ok(copy.copied().unwrap_or(symbol))
    }
}

fn within(inner: &FileSpan, outer: &FileSpan) -> bool {
    inner.file == outer.file && inner.start >= outer.start && inner.end <= outer.end
}
