//! Statement counts and complexity constructs, as the profile's Rule facts define them.

use super::model::{FileSpan, Node, NodeKind, Origin};
use super::source::Sources;
use super::tree::{self, Body};
use crate::complexity::{Kind, Node as Construct};
use crate::facts::GuardClause;

/// Generated code counts once, at its outermost node.
pub fn statements(body: &Body<'_>) -> Result<usize, String> {
    count(body, body.root()?, false)
}

/// `inside`: a macro expansion encloses the node. Source code counts, macro-generated
/// code counts once at its outermost node, and desugared nodes are not statements.
fn count(body: &Body<'_>, node: &Node, inside: bool) -> Result<usize, String> {
    let inside = inside || node.provenance.origin == Origin::MacroExpansion;
    let counts = |child: &Node| {
        usize::from(match child.provenance.origin {
            Origin::MacroExpansion => !inside,
            Origin::Desugaring => false,
            _ => true,
        })
    };
    let mut total = match node.kind {
        // A block a lowering adds holds the lowering's operands, not statements.
        NodeKind::Block if node.provenance.origin == Origin::Desugaring => 0,
        NodeKind::Block => body.children(node)?.into_iter().map(counts).sum(),
        NodeKind::MatchArm => match body.source_children(node)?.last() {
            Some(arm_body) if arm_body.kind != NodeKind::Block => counts(arm_body),
            _ => 0,
        },
        _ => 0,
    };

    for child in body.children(node)? {
        total += count(body, child, inside)?;
    }
    Ok(total)
}

/// Statements plus the tail directly in the root block; one for any other root.
pub fn top_statements(body: &Body<'_>) -> Result<usize, String> {
    let root = body.root()?;
    Ok(match root.kind {
        // What a frontend adds to the end of a block, such as destructors, nobody wrote.
        NodeKind::Block => body
            .children(root)?
            .into_iter()
            .filter(|child| !child.synthetic)
            .count(),
        _ => 1,
    })
}

/// Each `if` with a block for its `else` that ends by returning, around a block that doesn't and
/// holds something to bring out. How much is enough is a project's to say. An `else if` is left
/// alone, since it can't return early without restructuring the chain it's in.
pub fn guard_clauses(body: &Body<'_>, sources: &Sources<'_>) -> Result<Vec<GuardClause>, String> {
    let function = body.function;
    let is_else_if = |node: &Node| {
        let otherwise = |parent: &Node| parent.children.get(2) == Some(&node.id);
        let ifs = function.nodes.iter().filter(|n| n.kind == NodeKind::If);
        ifs.into_iter().any(otherwise)
    };

    let mut found = Vec::new();
    for node in function.nodes.iter().filter(|n| n.kind == NodeKind::If) {
        let [_, nested, otherwise] = body.children(node)?[..] else {
            continue;
        };
        let written = [node, nested, otherwise]
            .iter()
            .all(|n| !tree::generated(n));
        if written
            && !is_else_if(node)
            && !nested.children.is_empty()
            && only_returns(body, otherwise)?
            && !only_returns(body, nested)?
        {
            found.push(GuardClause {
                line: tree::line(sources, node)?,
                statements: nested.children.len(),
            });
        }
    }
    found.sort_by_key(|guard| guard.line);
    Ok(found)
}

/// Whether a block's last statement is a `return`.
fn only_returns(body: &Body<'_>, block: &Node) -> Result<bool, String> {
    if block.kind != NodeKind::Block {
        return Ok(false);
    }
    let Some(last) = block.children.last() else {
        return Ok(false);
    };

    let last = body.node(last)?;
    let stated = match (last.kind, last.children.first()) {
        (NodeKind::ExpressionStatement, Some(child)) => body.node(child)?,
        _ => last,
    };
    Ok(stated.kind == NodeKind::Return)
}

pub struct Constructs<'a> {
    pub body: &'a Body<'a>,
    pub sources: &'a Sources<'a>,
}

impl Constructs<'_> {
    /// The construct tree of the body; recursions are added by the caller.
    pub fn tree(&self) -> Result<Vec<Construct>, String> {
        self.visit(self.body.root()?)
    }

    fn construct(
        &self,
        kind: Kind,
        span: &FileSpan,
        children: Vec<Construct>,
    ) -> Result<Construct, String> {
        let (line, column) = self.sources.location(span)?;
        Ok(Construct {
            kind,
            line,
            column,
            children,
        })
    }

    fn wrap(&self, kind: Kind, span: &FileSpan, node: &Node) -> Result<Vec<Construct>, String> {
        Ok(vec![self.construct(kind, span, self.within(node)?)?])
    }

    fn within(&self, node: &Node) -> Result<Vec<Construct>, String> {
        let mut out = Vec::new();
        for child in self.body.children(node)? {
            out.extend(self.visit(child)?);
        }
        Ok(out)
    }

    fn visit(&self, node: &Node) -> Result<Vec<Construct>, String> {
        // Macro-expanded and desugared nodes are not the developer's, so they are looked
        // through: the source constructs below them count, as code passed to a macro or
        // wrapped by one is written like any other.
        if matches!(
            node.provenance.origin,
            Origin::MacroExpansion | Origin::Desugaring
        ) {
            return self.within(node);
        }
        let span = || own_span(node);
        match node.kind {
            NodeKind::If => self.branch(node, Kind::Branch),
            NodeKind::Match => self.selection(node),
            NodeKind::Operator if logical(node).is_some() => Ok(vec![self.construct(
                Kind::Boolean,
                span()?,
                self.boolean_parts(node)?,
            )?]),
            NodeKind::LocalDeclaration => self.declaration(node),
            NodeKind::While | NodeKind::Loop | NodeKind::For => {
                self.wrap(Kind::Loop, span()?, node)
            }
            NodeKind::Closure => self.wrap(Kind::Closure, span()?, node),
            NodeKind::Propagate => self.wrap(
                Kind::Propagate,
                node.token.as_ref().unwrap_or(span()?),
                node,
            ),
            NodeKind::Break | NodeKind::Continue if node.label.is_some() => {
                self.wrap(Kind::Jump, span()?, node)
            }
            NodeKind::ItemDeclaration => Ok(vec![]),
            _ => self.within(node),
        }
    }

    /// `if`, then each `else if` and `else` as siblings of the branch.
    fn branch(&self, node: &Node, kind: Kind) -> Result<Vec<Construct>, String> {
        let children = self.body.source_children(node)?;
        let (condition, then) = match children.as_slice() {
            [condition, then, ..] => (*condition, *then),
            _ => {
                return Err(format!(
                    "Node {}: an if needs a condition and a then-block",
                    node.id
                ));
            }
        };

        let mut inner = self.visit(condition)?;
        inner.extend(self.visit(then)?);
        let mut out = vec![self.construct(kind, own_span(node)?, inner)?];
        match children.get(2) {
            Some(otherwise)
                if otherwise.kind == NodeKind::If
                    && otherwise.provenance.origin != Origin::MacroExpansion =>
            {
                out.extend(self.branch(otherwise, Kind::ElseIf)?)
            }
            Some(otherwise) => out.push(self.construct(
                Kind::Else,
                anchor_span(otherwise)?,
                self.visit(otherwise)?,
            )?),
            None => {}
        }
        Ok(out)
    }

    fn selection(&self, node: &Node) -> Result<Vec<Construct>, String> {
        let mut inner = Vec::new();
        for child in self.body.source_children(node)? {
            if child.kind == NodeKind::MatchArm {
                inner.push(self.alternative(child)?);
            } else {
                inner.extend(self.visit(child)?);
            }
        }
        Ok(vec![self.construct(
            Kind::Selection,
            own_span(node)?,
            inner,
        )?])
    }

    fn alternative(&self, arm: &Node) -> Result<Construct, String> {
        let children = self.body.source_children(arm)?;
        let pattern = children
            .first()
            .ok_or_else(|| format!("Node {}: an arm needs a pattern", arm.id))?;

        let mut inner = Vec::new();
        for child in &children[1..] {
            if child.kind == NodeKind::MatchGuard {
                inner.push(self.construct(
                    Kind::Guard,
                    anchor_span(child)?,
                    self.within(child)?,
                )?);
            } else {
                inner.extend(self.visit(child)?);
            }
        }
        self.construct(Kind::Alternative, anchor_span(pattern)?, inner)
    }

    /// A declaration with a diverging `else` block branches on its pattern.
    fn declaration(&self, node: &Node) -> Result<Vec<Construct>, String> {
        let children = self.body.source_children(node)?;
        let rest: Vec<_> = children
            .iter()
            .skip(1)
            .filter(|child| child.kind != NodeKind::TypeAnnotation)
            .collect();
        let [value, otherwise] = rest.as_slice() else {
            return self.within(node);
        };

        let mut inner = self.visit(value)?;
        inner.extend(self.visit(otherwise)?);
        Ok(vec![self.construct(
            Kind::Branch,
            own_span(node)?,
            inner,
        )?])
    }

    /// One sequence for a chain of `and` and `or`, each operator at its token.
    fn boolean_parts(&self, node: &Node) -> Result<Vec<Construct>, String> {
        if node.provenance.origin == Origin::MacroExpansion {
            return self.within(node);
        }
        let Some(kind) = logical(node) else {
            return self.visit(node);
        };
        let children = self.body.source_children(node)?;
        let [left, right] = children.as_slice() else {
            return Err(format!(
                "Node {}: a binary operator needs two operands",
                node.id
            ));
        };

        let mut parts = self.boolean_parts(left)?;
        let token = node.token.as_ref().unwrap_or(own_span(node)?);
        parts.push(self.construct(kind, token, vec![])?);
        parts.extend(self.boolean_parts(right)?);
        Ok(parts)
    }
}

fn logical(node: &Node) -> Option<Kind> {
    match (node.kind, node.operator.as_deref()) {
        (NodeKind::Operator, Some("and")) => Some(Kind::And),
        (NodeKind::Operator, Some("or")) => Some(Kind::Or),
        _ => None,
    }
}

fn own_span(node: &Node) -> Result<&FileSpan, String> {
    node.span
        .as_ref()
        .ok_or_else(|| format!("Node {} has no span", node.id))
}

fn anchor_span(node: &Node) -> Result<&FileSpan, String> {
    super::tree::anchor(node)
}
