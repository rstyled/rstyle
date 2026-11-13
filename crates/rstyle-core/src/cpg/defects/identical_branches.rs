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

use super::super::model::{Access, FileSpan, Node, NodeKind, Origin, Ref, RefTarget, TargetKind};
use super::super::tree;
use super::{Defect, Reading};
use std::collections::HashMap;

pub(super) const RULE: &str = "identical_branches";

const BOTH: &str = "the condition here changes nothing: both branches run the same code";
const FOLLOWS: &str =
    "the condition here changes nothing: the branch runs the same code as what follows it";

pub(super) fn find(reading: &Reading<'_>) -> Result<Vec<Defect>, String> {
    let accesses = by_node(&reading.function.accesses);
    let mut found = Vec::new();
    for node in &reading.function.nodes {
        if node.kind != NodeKind::If || node.provenance.origin != Origin::Source {
            continue;
        }
        let Some((taken, other, message)) = ways(reading, node)? else {
            continue;
        };
        if !taken.is_empty() && Twins::new(reading, &accesses).same(&taken, &other)? {
            found.push(Defect {
                rule: RULE,
                line: tree::line(reading.sources, node)?,
                message: message.into(),
            });
        }
    }
    Ok(found)
}

/// Each node's reads and writes.
type Accesses<'a> = HashMap<&'a str, Vec<&'a Access>>;

fn by_node(accesses: &[Access]) -> Accesses<'_> {
    let mut found = Accesses::new();
    for access in accesses {
        found.entry(&access.node).or_default().push(access);
    }
    found
}

/// The statements a condition runs when it holds and the ones it runs when it doesn't, where
/// they can be told: its two branches, or a branch that returns and as many statements after it.
type Ways<'a> = (Vec<&'a Node>, Vec<&'a Node>, &'static str);

fn ways<'a>(reading: &Reading<'a>, branch: &Node) -> Result<Option<Ways<'a>>, String> {
    let children = reading.body.children(branch)?;
    match children.as_slice() {
        [_, then, otherwise] => Ok(Some((
            statements(reading, then)?,
            statements(reading, otherwise)?,
            BOTH,
        ))),
        [_, then] => {
            let taken = statements(reading, then)?;
            if !returns(reading, taken.last())? {
                return Ok(None);
            }

            let mut after = following(reading, branch)?;
            after.truncate(taken.len());
            Ok(Some((taken, after, FOLLOWS)))
        }
        _ => Ok(None),
    }
}

/// What a branch runs: a block's statements, or the one statement written without a block.
fn statements<'a>(reading: &Reading<'a>, node: &'a Node) -> Result<Vec<&'a Node>, String> {
    match node.kind {
        NodeKind::Block => reading.body.children(node),
        _ => Ok(vec![node]),
    }
}

fn returns(reading: &Reading<'_>, statement: Option<&&Node>) -> Result<bool, String> {
    let Some(&statement) = statement else {
        return Ok(false);
    };
    let inner = match statement.kind {
        NodeKind::ExpressionStatement => reading.body.children(statement)?.pop(),
        _ => Some(statement),
    };
    Ok(inner.is_some_and(|node| node.kind == NodeKind::Return))
}

/// The statements after a condition in the block that holds it.
fn following<'a>(reading: &Reading<'a>, branch: &Node) -> Result<Vec<&'a Node>, String> {
    let parent = |id: &str| reading.parents.get(id).copied();
    let statement = match parent(&branch.id) {
        Some(wrapper) if wrapper.kind == NodeKind::ExpressionStatement => wrapper.id.as_str(),
        _ => branch.id.as_str(),
    };
    let Some(block) = parent(statement).filter(|node| node.kind == NodeKind::Block) else {
        return Ok(Vec::new());
    };

    let after = block
        .children
        .iter()
        .skip_while(|child| *child != statement);
    after.skip(1).map(|id| reading.body.node(id)).collect()
}

/// The nodes of two pieces of code, matched up as both are walked together.
struct Twins<'r, 'a> {
    reading: &'r Reading<'a>,
    accesses: &'r Accesses<'a>,
    twin: HashMap<&'a str, &'a str>,
}

impl<'r, 'a> Twins<'r, 'a> {
    fn new(reading: &'r Reading<'a>, accesses: &'r Accesses<'a>) -> Self {
        Self {
            reading,
            accesses,
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
            if !self.resolve_alike(left, right)? {
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
    /// or its name, with no whitespace.
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
            if child.start >= at {
                text.push_str(self.text(span, at, child.start)?);
                at = child.end;
            }
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

    /// Whether two paired nodes read and write the same bindings and name the same things.
    fn resolve_alike(&self, left: &str, right: &str) -> Result<bool, String> {
        let none = Vec::new();
        let accesses = |node: &str| self.accesses.get(node).unwrap_or(&none);
        let (reads, others) = (accesses(left), accesses(right));
        if reads.len() != others.len() {
            return Ok(false);
        }
        for (read, other) in reads.iter().zip(others) {
            if read.mode != other.mode || !self.same_symbol(&read.symbol, &other.symbol)? {
                return Ok(false);
            }
        }

        let body = &self.reading.body;
        match (body.reference(left), body.reference(right)) {
            (None, None) => Ok(true),
            (Some(reference), Some(other)) => self.same_reference(reference, other),
            _ => Ok(false),
        }
    }

    /// A closure is another function, whose body isn't walked here, so two of them are never
    /// known to be the same.
    fn same_reference(&self, reference: &Ref, other: &Ref) -> Result<bool, String> {
        let alike = reference.status == other.status
            && reference.dispatch == other.dispatch
            && reference.closure.is_none()
            && other.closure.is_none();
        Ok(alike
            && self.same_target(reference.target.as_ref(), other.target.as_ref())?
            && self.same_target(reference.selected.as_ref(), other.selected.as_ref())?)
    }

    fn same_target(
        &self,
        target: Option<&RefTarget>,
        other: Option<&RefTarget>,
    ) -> Result<bool, String> {
        let (Some(target), Some(other)) = (target, other) else {
            return Ok(target.is_none() && other.is_none());
        };
        if target.kind != other.kind || target.operation != other.operation {
            return Ok(false);
        }
        match (target.kind, target.id.as_deref(), other.id.as_deref()) {
            (TargetKind::Symbol, Some(symbol), Some(other)) => self.same_symbol(symbol, other),
            (_, id, other) => Ok(id == other),
        }
    }

    /// One binding, or two that paired nodes declare: each piece of code's own copy of a local.
    fn same_symbol(&self, symbol: &str, other: &str) -> Result<bool, String> {
        if symbol == other {
            return Ok(true);
        }
        let body = &self.reading.body;
        let declared = body.symbol(symbol)?.node.as_deref();
        let twin = declared.and_then(|node| self.twin.get(node).copied());
        Ok(twin.is_some() && twin == body.symbol(other)?.node.as_deref())
    }
}

fn within(inner: &FileSpan, outer: &FileSpan) -> bool {
    inner.file == outer.file && inner.start >= outer.start && inner.end <= outer.end
}
