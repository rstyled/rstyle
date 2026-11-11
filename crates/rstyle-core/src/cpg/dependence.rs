//! Dependence graphs of function bodies (the profile's Pattern facts): operands give
//! data edges, a binding links to its uses, and the enclosing branch, match, loop or
//! iteration gives control edges. There are no execution-order edges.

use super::model::{
    DefinitionKind, Dispatch, Literal, Node, NodeKind, Ref, RefStatus, RefTarget, Scope,
    TargetKind, TypeKind,
};
use super::shapes::{callable_shape, shape};
use super::source::Sources;
use super::tree::{Body, Tables, generated, line, scope_end};
use crate::clip::clip;
use crate::facts::{NodeKind as Kind, Pdg, PdgNode, TyShape};
use crate::pdg_builder::{GraphBuilder, Lowering};
use crate::versioned::ByNamespace;
use serde::Deserialize;
use std::collections::BTreeMap;

const SCHEMA: &str = "rstyle.dependence-roles/1";
/// Role files compiled into the engine, one per frontend namespace.
const EMBEDDED: &[&str] = &[include_str!("contracts/rust.toml")];

/// Methods with a dependence-graph role, for one frontend namespace.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    schema: String,
    namespace: String,
    #[serde(default)]
    views: Vec<String>,
    #[serde(default)]
    per_item: Vec<String>,
}

impl Roles {
    /// Argument-free library methods the graph looks through to their receiver.
    pub fn views(&self) -> &[String] {
        &self.views
    }

    /// The roles of a frontend namespace; none for a namespace without a file.
    pub fn of(namespace: &str) -> &'static Roles {
        static ROLES: ByNamespace<Roles> = ByNamespace::empty();
        ROLES.of(
            namespace,
            (EMBEDDED, |text| {
                Roles::parse(text).map(|roles| (roles.namespace.clone(), roles))
            }),
        )
    }

    pub(super) fn parse(text: &str) -> Result<Self, String> {
        crate::versioned::parse(text, (SCHEMA, "dependence roles"), |roles: &Self| {
            &roles.schema
        })
    }
}

/// The dependence graph of one function body.
pub fn dependence_graph(
    body: &Body<'_>,
    tables: &Tables<'_>,
    sources: &Sources<'_>,
    roles: &Roles,
) -> Result<Pdg, String> {
    let mut graph = Graph {
        body,
        tables,
        sources,
        roles,
        graph: GraphBuilder::default(),
        binds: BTreeMap::new(),
        declared: body
            .function
            .symbols
            .iter()
            .filter_map(|symbol| Some((symbol.node.as_deref()?, symbol.id.as_str())))
            .collect(),
    };
    graph.lower_body()?;

    Ok(graph.graph.finish())
}

struct Graph<'b, 'a> {
    body: &'b Body<'a>,
    tables: &'b Tables<'a>,
    sources: &'b Sources<'a>,
    roles: &'b Roles,
    graph: GraphBuilder,
    /// The graph node each symbol is bound to.
    binds: BTreeMap<&'a str, usize>,
    /// The symbol each binding pattern node declares.
    declared: BTreeMap<&'a str, &'a str>,
}

type Lowered = Result<Option<usize>, String>;

impl Lowering for Graph<'_, '_> {
    fn graph(&mut self) -> &mut GraphBuilder {
        &mut self.graph
    }
}

fn blank(kind: Kind) -> PdgNode {
    PdgNode {
        kind,
        ty: None,
        callee_id: None,
        callee_sig: None,
        macro_name: None,
        lit_kind: None,
        detail: String::new(),
        line: 0,
    }
}

impl<'a> Graph<'_, 'a> {
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn lower_body(&mut self) -> Result<(), String> {
        self.params()?;
        let root = self.body.root()?;
        let value = self.expr(root)?;
        // A unit-valued body returns nothing: `f();` and a tail `f()` give the same graph.
        if self.type_kind(root)? != TypeKind::Unit {
            let node = PdgNode {
                line: self.line(root)?,
                ..blank(Kind::Return)
            };
            let ret = self.graph.add(node);
            self.graph.link(value, ret, 0);
        }

        Ok(())
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    /// The receiver, then each parameter, bound to its own node.
    fn params(&mut self) -> Result<(), String> {
        let signature = &self.body.function.signature;
        if let Some(receiver) = &signature.receiver {
            let symbol = self.body.symbol(&receiver.symbol)?;
            let declared = symbol
                .declared
                .as_ref()
                .ok_or_else(|| format!("Receiver {} has no declaring span", symbol.id))?;
            let node = PdgNode {
                ty: Some(self.symbol_shape(&receiver.symbol)?),
                line: self.sources.location(declared)?.0,
                ..blank(Kind::Param)
            };
            let index = self.graph.add(node);
            self.binds.insert(receiver.symbol.as_str(), index);
        }
        for parameter in &signature.parameters {
            let span = parameter
                .span
                .as_ref()
                .ok_or_else(|| format!("Parameter {} has no span", parameter.position))?;
            let node = PdgNode {
                ty: Some(self.symbol_shape(&parameter.symbol)?),
                line: self.sources.location(span)?.0,
                ..blank(Kind::Param)
            };
            let index = self.graph.add(node);
            let bindings = parameter.bindings.as_deref();
            let symbols = bindings.unwrap_or(std::slice::from_ref(&parameter.symbol));
            for symbol in symbols {
                self.binds.insert(symbol.as_str(), index);
            }
        }
        Ok(())
    }

    /// The shape of a symbol's type: a parameter's pattern type, as the body sees it.
    fn symbol_shape(&self, id: &str) -> Result<TyShape, String> {
        let symbol = self.body.symbol(id)?;
        let ty = symbol
            .ty
            .as_deref()
            .ok_or_else(|| format!("Symbol {} has no type", symbol.id))?;
        shape(self.tables, ty)
    }

    fn line(&self, node: &Node) -> Result<usize, String> {
        line(self.sources, node)
    }

    fn type_of<'n>(&self, node: &'n Node) -> Result<&'n str, String> {
        node.ty
            .as_deref()
            .ok_or_else(|| format!("Node {} has no type", node.id))
    }

    fn type_kind(&self, node: &Node) -> Result<TypeKind, String> {
        Ok(self.tables.ty(self.type_of(node)?)?.kind)
    }

    fn shape_of(&self, node: &Node) -> Result<TyShape, String> {
        shape(self.tables, self.type_of(node)?)
    }

    /// A node of `kind` with the node's type and line.
    fn typed(&self, kind: Kind, node: &Node) -> Result<PdgNode, String> {
        Ok(PdgNode {
            ty: Some(self.shape_of(node)?),
            line: self.line(node)?,
            ..blank(kind)
        })
    }

    /// Binds every symbol `pattern` declares to `node`.
    fn bind(&mut self, pattern: &Node, node: usize) -> Result<(), String> {
        if let Some(symbol) = self.declared.get(pattern.id.as_str()) {
            self.binds.insert(symbol, node);
        }
        for child in self.body.children(pattern)? {
            self.bind(child, node)?;
        }
        Ok(())
    }

    fn operands(&mut self, to: usize, operands: &[&'a Node], start: usize) -> Result<(), String> {
        for (offset, operand) in operands.iter().enumerate() {
            let value = self.expr(operand)?;
            self.graph.link(value, to, start + offset);
        }
        Ok(())
    }
}

impl<'a> Graph<'_, 'a> {
    /// Lowers an expression; the result is the node that produces its value, if any.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn expr(&mut self, node: &'a Node) -> Lowered {
        // Nobody wrote what a scope's end runs, so it says nothing about the shape of the code. All
        // it can hold of theirs is the expression whose temporaries it destroys.
        if scope_end(node) {
            for child in self.body.children(node)? {
                if self.body.has_source(child)? {
                    return self.expr(child);
                }
            }
            return Ok(None);
        }
        match node.kind {
            NodeKind::For => return self.iterate(node).map(Some),
            NodeKind::Propagate => return self.propagated(node).map(Some),
            _ => {}
        }
        if let Some(name) = node.provenance.macro_name.clone() {
            return self.macro_call(node, name).map(Some);
        }
        self.expr_kind(node)
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn expr_kind(&mut self, node: &'a Node) -> Lowered {
        if let Some(lowered) = self.control(node)? {
            return Ok(lowered);
        }
        Ok(match node.kind {
            NodeKind::Identifier => self.path(node)?,
            NodeKind::Literal => Some(self.literal(node)?),
            NodeKind::Call => self.call(node)?,
            NodeKind::FieldAccess => Some(self.field(node)?),
            NodeKind::Reference => self.expr(self.body.child(node, 0)?)?,
            NodeKind::Operator if node.operator.as_deref() == Some("deref") => {
                self.expr(self.body.child(node, 0)?)?
            }
            NodeKind::Block if node.modifiers.iter().any(|m| m == "const") => {
                Some(self.graph.add(self.generic_node(node)?))
            }
            NodeKind::Block => self.block(node)?,
            NodeKind::Closure => Some(self.graph.add(self.typed(Kind::Closure, node)?)),
            NodeKind::Assignment if self.body.destructures(node)? => {
                self.destructuring(node)?;
                None
            }
            _ => Some(self.generic(node)?),
        })
    }

    /// Branches, matches, loops and jumps; `None` for any other node.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn control(&mut self, node: &'a Node) -> Result<Option<Option<usize>>, String> {
        Ok(Some(match node.kind {
            NodeKind::If => Some(self.branch(node)?),
            NodeKind::Match => Some(self.matcher(node)?),
            NodeKind::Loop => {
                let body = self.body.child(node, 0)?;
                Some(self.looped((node, "loop"), |walk| walk.expr(body).map(drop))?)
            }
            NodeKind::While => Some(self.while_loop(node)?),
            NodeKind::Return => self.returned(node)?,
            NodeKind::PatternCondition => Some(self.let_expr(node)?),
            NodeKind::Break => match node.children.first() {
                Some(value) => self.expr(self.body.node(value)?)?,
                None => None,
            },
            NodeKind::Continue => None,
            _ => return Ok(None),
        }))
    }

    fn field(&mut self, node: &'a Node) -> Result<usize, String> {
        let entry = PdgNode {
            detail: node.member.clone().unwrap_or_default(),
            ..self.typed(Kind::Field, node)?
        };
        let index = self.graph.add(entry);
        let value = self.expr(self.body.child(node, 0)?)?;

        self.graph.link(value, index, 0);
        Ok(index)
    }

    fn path(&mut self, node: &Node) -> Lowered {
        let target = self
            .body
            .reference(&node.id)
            .and_then(|r| r.target.as_ref());
        if let Some(RefTarget {
            kind: TargetKind::Symbol,
            id: Some(symbol),
            ..
        }) = target
        {
            return Ok(self.binds.get(symbol.as_str()).copied());
        }
        let function = match target {
            Some(RefTarget {
                kind: TargetKind::Definition,
                id: Some(id),
                ..
            }) => Some(self.tables.definition(id)?)
                .filter(|d| matches!(d.kind, DefinitionKind::Function | DefinitionKind::Method)),
            _ => None,
        };
        let entry = match function {
            Some(definition) => PdgNode {
                lit_kind: Some("fn".into()),
                callee_id: Some(definition.stable_identity.clone()),
                callee_sig: callable_shape(self.tables, definition.callable.as_ref())?,
                ..self.typed(Kind::Lit, node)?
            },
            None => PdgNode {
                lit_kind: Some("path".into()),
                detail: self.path_detail(node)?,
                ..self.typed(Kind::Lit, node)?
            },
        };

        Ok(Some(self.graph.add(entry)))
    }

    /// Source text of a path, empty in generated code (its spans lie there).
    fn path_detail(&self, node: &Node) -> Result<String, String> {
        match &node.span {
            Some(span) if !generated(node) => Ok(clip(self.sources.between(span, span)?, 32)),
            _ => Ok(String::new()),
        }
    }

    fn literal(&mut self, node: &Node) -> Result<usize, String> {
        let value = node.value.as_ref();
        let entry = PdgNode {
            lit_kind: Some(literal_kind(value).into()),
            detail: match generated(node) {
                true => String::new(),
                false => literal_detail(value),
            },
            ..self.typed(Kind::Lit, node)?
        };
        Ok(self.graph.add(entry))
    }

    /// A call in function form, or a method call.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn call(&mut self, node: &'a Node) -> Lowered {
        let reference = self.body.reference(&node.id);
        let dispatch = reference.and_then(|r| r.dispatch);
        if matches!(dispatch, Some(Dispatch::Method | Dispatch::Dynamic)) {
            return self.method(node);
        }
        let children = self.body.children(node)?;
        let (callee, arguments) = children
            .split_first()
            .ok_or_else(|| format!("Call {} has no callee", node.id))?;
        let target = match self.function_item(callee)? {
            Some(target) => Some(target),
            None => self.statically_called(reference)?,
        };
        let constructs = matches!(
            self.body
                .reference(&callee.id)
                .and_then(|r| r.target.as_ref()),
            Some(RefTarget {
                kind: TargetKind::Constructor,
                ..
            })
        );
        let entry = match (target, constructs) {
            (Some(_), true) => PdgNode {
                detail: "ctor".into(),
                ..self.typed(Kind::Op, node)?
            },
            _ => self.call_node(node, target)?,
        };
        let index = self.graph.add(entry);
        if target.is_none() {
            let value = self.expr(callee)?;
            self.graph.link(value, index, 0);
        }
        self.operands(index, arguments, 0)?;

        Ok(Some(index))
    }

    /// The function a call resolves to statically by its own reference, for a callee whose type
    /// names none: an overloaded Python function, such as `subprocess.run`, has no one function
    /// type, but its call still names the definition it runs, as its events read it.
    fn statically_called(&self, reference: Option<&'a Ref>) -> Result<Option<&'a str>, String> {
        let id = reference
            .filter(|reference| {
                reference.status == RefStatus::Resolved
                    && matches!(reference.dispatch, None | Some(Dispatch::Static))
            })
            .and_then(|reference| reference.target.as_ref())
            .filter(|target| matches!(target.kind, TargetKind::Definition))
            .and_then(|target| target.id.as_deref());
        let Some(id) = id else {
            return Ok(None);
        };
        let definition = self.tables.definition(id)?;
        Ok(matches!(
            definition.kind,
            DefinitionKind::Function | DefinitionKind::Method
        )
        .then_some(id))
    }

    /// The function a callee's type is the item type of, when it is one.
    fn function_item(&self, callee: &Node) -> Result<Option<&'a str>, String> {
        let Some(ty) = callee.ty.as_deref() else {
            return Ok(None);
        };
        let ty = self.tables.ty(ty)?;
        Ok(match ty.kind {
            TypeKind::Function => ty.definition.as_deref(),
            _ => None,
        })
    }

    fn call_node(&self, node: &Node, callee: Option<&str>) -> Result<PdgNode, String> {
        let definition = callee.map(|id| self.tables.definition(id)).transpose()?;
        Ok(PdgNode {
            callee_id: definition.map(|d| d.stable_identity.clone()),
            callee_sig: match definition {
                Some(definition) => callable_shape(self.tables, definition.callable.as_ref())?,
                None => None,
            },
            ..self.typed(Kind::Call, node)?
        })
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn method(&mut self, node: &'a Node) -> Lowered {
        let reference = self.body.reference(&node.id);
        let declared = reference.and_then(|r| match r.status {
            RefStatus::Dependent => r.via.as_deref(),
            _ => r.target.as_ref().and_then(|target| target.id.as_deref()),
        });
        let definition = declared.map(|id| self.tables.definition(id)).transpose()?;
        let children = self.body.children(node)?;
        let (receiver, arguments) = children
            .split_first()
            .ok_or_else(|| format!("Method call {} has no receiver", node.id))?;
        if let Some(definition) = definition {
            let name = definition.name.rsplit("::").next().unwrap_or_default();
            let library = definition.scope == Scope::Library;
            if arguments.is_empty() && library && self.roles.views().iter().any(|view| view == name)
            {
                return self.expr(receiver);
            }
            let per_item = self
                .roles
                .per_item
                .iter()
                .any(|m| definition.name.ends_with(m.as_str()));
            if let ([closure], true) = (arguments, per_item)
                && closure.kind == NodeKind::Closure
            {
                return self.for_each(node, receiver, closure).map(Some);
            }
        }
        let entry = self.call_node(node, declared)?;
        let index = self.graph.add(entry);
        self.operands(index, &children, 0)?;

        Ok(Some(index))
    }

    /// `head.for_each(|item| body)`: the same iteration as `for item in head { body }`.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn for_each(
        &mut self,
        node: &Node,
        head: &'a Node,
        closure: &'a Node,
    ) -> Result<usize, String> {
        let parts = self.body.children(closure)?;
        let (body, parameters) = parts
            .split_last()
            .ok_or_else(|| format!("Closure {} has no body", closure.id))?;
        let item = parameters.first().copied();
        let mut entry = self.typed(Kind::Iterate, node)?;
        if let Some(item) = item {
            entry.ty = Some(self.shape_of(item)?);
        }
        let index = self.graph.add(entry);
        let value = self.expr(head)?;
        self.graph.link(value, index, 0);
        if let Some(item) = item {
            self.bind(item, index)?;
        }
        self.under(index, 0, |walk| walk.expr(body))?;

        Ok(index)
    }

    /// `for pattern in head { body }`: the iteration binds the pattern to each item.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn iterate(&mut self, node: &'a Node) -> Result<usize, String> {
        let pattern = self.body.child(node, 0)?;
        let head = self.wrapped(self.body.child(node, 1)?)?;
        let body = self.body.child(node, 2)?;
        let entry = PdgNode {
            ty: Some(self.shape_of(pattern)?),
            ..self.typed(Kind::Iterate, node)?
        };
        let index = self.graph.add(entry);
        let value = self.expr(head)?;
        self.graph.link(value, index, 0);
        self.bind(pattern, index)?;
        self.under(index, 0, |walk| walk.expr(body))?;

        Ok(index)
    }

    /// The source operand of a lowering's call that wraps it, such as `into_iter(head)`.
    fn wrapped(&self, node: &'a Node) -> Result<&'a Node, String> {
        if !node.synthetic {
            return Ok(node);
        }

        let children = self.body.children(node)?;
        if let Some(source) = children.iter().rev().find(|child| !child.synthetic) {
            return Ok(source);
        }

        // A destructor wrapped around a temporary's expression can wrap another lowering.
        for child in children.into_iter().rev() {
            if self.body.has_source(child)? {
                return self.wrapped(child);
            }
        }
        Err(format!("Synthetic node {} wraps no source node", node.id))
    }

    fn propagated(&mut self, node: &'a Node) -> Result<usize, String> {
        let index = self.graph.add(self.typed(Kind::Try, node)?);
        let operand = self.wrapped(self.body.child(node, 0)?)?;
        let value = self.expr(operand)?;
        self.graph.link(value, index, 0);

        Ok(index)
    }

    /// A macro call: one node whose operands are the source expressions inside it.
    fn macro_call(&mut self, node: &'a Node, name: String) -> Result<usize, String> {
        let entry = PdgNode {
            macro_name: Some(name),
            ..self.typed(Kind::Macro, node)?
        };
        let index = self.graph.add(entry);
        let mut arguments = Vec::new();
        self.source_expressions(node, &mut arguments)?;
        self.operands(index, &arguments, 0)?;

        Ok(index)
    }

    /// Source expressions below a generated node, outermost first; closures are opaque.
    fn source_expressions(&self, node: &'a Node, found: &mut Vec<&'a Node>) -> Result<(), String> {
        if node.kind == NodeKind::Closure {
            return Ok(());
        }
        for child in self.body.children(node)? {
            if is_expression(child) && !generated(child) && !child.synthetic {
                found.push(child);
            } else {
                self.source_expressions(child, found)?;
            }
        }
        Ok(())
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn block(&mut self, node: &'a Node) -> Lowered {
        let children = self.body.children(node)?;
        let mut value = None;
        for child in children {
            match child.kind {
                NodeKind::LocalDeclaration => self.local(child)?,
                NodeKind::ExpressionStatement => {
                    self.expr(self.body.child(child, 0)?)?;
                }
                NodeKind::ItemDeclaration => {}
                _ => value = self.expr(child)?,
            }
        }
        Ok(value)
    }

    /// `let x = e` is transparent (`x` is `e`'s node); any other pattern gets a `Let` node.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn local(&mut self, node: &'a Node) -> Result<(), String> {
        let parts = self.body.children(node)?;
        let pattern = parts
            .first()
            .copied()
            .ok_or_else(|| format!("Declaration {} has no pattern", node.id))?;
        let rest: Vec<&'a Node> = parts[1..]
            .iter()
            .copied()
            .filter(|part| part.kind != NodeKind::TypeAnnotation)
            .collect();
        let init = match rest.first() {
            Some(init) => self.expr(init)?,
            None => None,
        };
        let simple = pattern.kind == NodeKind::PatternBinding && pattern.children.is_empty();
        match init {
            Some(value) if simple => self.bind(pattern, value)?,
            _ => {
                let entry = PdgNode {
                    ty: Some(self.shape_of(pattern)?),
                    line: self.line(node)?,
                    ..blank(Kind::Let)
                };
                let index = self.graph.add(entry);
                self.graph.link(init, index, 0);
                self.bind(pattern, index)?;
            }
        }
        if let Some(otherwise) = rest.get(1) {
            self.block(otherwise)?;
        }

        Ok(())
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn let_expr(&mut self, node: &'a Node) -> Result<usize, String> {
        let index = self.graph.add(self.typed(Kind::Let, node)?);
        let value = self.expr(self.body.child(node, 1)?)?;
        self.graph.link(value, index, 0);
        self.bind(self.body.child(node, 0)?, index)?;

        Ok(index)
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn branch(&mut self, node: &'a Node) -> Result<usize, String> {
        let index = self.graph.add(self.typed(Kind::Branch, node)?);
        let value = self.expr(self.body.child(node, 0)?)?;
        self.graph.link(value, index, 0);
        let then = self.body.child(node, 1)?;
        self.under(index, 0, |walk| walk.expr(then))?;
        if node.children.len() > 2 {
            let otherwise = self.body.child(node, 2)?;
            self.under(index, 1, |walk| walk.expr(otherwise))?;
        }

        Ok(index)
    }

    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn matcher(&mut self, node: &'a Node) -> Result<usize, String> {
        let index = self.graph.add(self.typed(Kind::Match, node)?);
        let children = self.body.children(node)?;
        let (scrutinee, arms) = children
            .split_first()
            .ok_or_else(|| format!("Match {} has no scrutinee", node.id))?;
        let value = self.expr(scrutinee)?;
        self.graph.link(value, index, 0);
        for (position, arm) in arms.iter().enumerate() {
            let parts = self.body.children(arm)?;
            let (pattern, rest) = parts
                .split_first()
                .ok_or_else(|| format!("Arm {} has no pattern", arm.id))?;
            self.bind(pattern, index)?;
            self.under(index, position, |walk| -> Result<(), String> {
                for part in rest {
                    match part.kind {
                        NodeKind::MatchGuard => {
                            walk.expr(walk.body.child(part, 0)?)?;
                        }
                        _ => {
                            walk.expr(part)?;
                        }
                    }
                }
                Ok(())
            })?;
        }

        Ok(index)
    }

    /// A loop node whose body `lower` walks under it.
    fn looped(
        &mut self,
        (node, detail): (&'a Node, &str),
        lower: impl FnOnce(&mut Self) -> Result<(), String>,
    ) -> Result<usize, String> {
        let entry = PdgNode {
            detail: detail.into(),
            ..self.typed(Kind::Loop, node)?
        };
        let index = self.graph.add(entry);

        self.under(index, 0, lower)?;
        Ok(index)
    }

    /// `while condition { body }`: a loop whose body branches on the condition.
    fn while_loop(&mut self, node: &'a Node) -> Result<usize, String> {
        let condition = self.body.child(node, 0)?;
        let body = self.body.child(node, 1)?;
        self.looped((node, "while"), |walk| {
            let branch = walk.graph.add(walk.typed(Kind::Branch, node)?);
            let value = walk.expr(condition)?;

            walk.graph.link(value, branch, 0);
            walk.under(branch, 0, |walk| walk.expr(body).map(drop))
        })
    }

    fn returned(&mut self, node: &'a Node) -> Lowered {
        let index = self.graph.add(self.typed(Kind::Return, node)?);
        if let Some(value) = node.children.first() {
            let value = self.expr(self.body.node(value)?)?;
            self.graph.link(value, index, 0);
        }
        Ok(None)
    }

    /// `target = value` with a destructuring target: a `Let` of the value, then one
    /// assignment from it per place the target names.
    // rstyle-allow(mutation_per_target): the graph accumulates nodes and edges in self by design
    fn destructuring(&mut self, node: &'a Node) -> Result<(), String> {
        let target = self.body.child(node, 0)?;
        let value = self.expr(self.body.child(node, 1)?)?;
        let entry = PdgNode {
            ty: Some(self.shape_of(target)?),
            line: self.line(node)?,
            ..blank(Kind::Let)
        };
        let binding = self.graph.add(entry);
        self.graph.link(value, binding, 0);
        for place in self.body.places(target)? {
            let assignment = PdgNode {
                detail: "=".into(),
                ty: Some(TyShape::Tuple(vec![])),
                line: self.line(place)?,
                ..blank(Kind::Op)
            };
            let index = self.graph.add(assignment);
            self.graph.link(Some(binding), index, 0);
            let written = self.expr(place)?;
            self.graph.link(written, index, 1);
        }

        Ok(())
    }

    fn generic(&mut self, node: &'a Node) -> Result<usize, String> {
        let index = self.graph.add(self.generic_node(node)?);
        let mut operands = self.body.children(node)?;
        // An assignment's value is evaluated before the place it writes.
        if matches!(
            node.kind,
            NodeKind::Assignment | NodeKind::CompoundAssignment
        ) {
            operands.reverse();
        }
        self.operands(index, &operands, 0)?;

        Ok(index)
    }

    /// Operators, construction, indexing, assignment and casts.
    fn generic_node(&self, node: &Node) -> Result<PdgNode, String> {
        let operator = node.operator.as_deref().unwrap_or_default();
        let (kind, detail) = match node.kind {
            NodeKind::Operator => (Kind::Op, symbol(operator, node.children.len()).to_string()),
            NodeKind::CompoundAssignment => (Kind::Op, format!("{}=", symbol(operator, 2))),
            NodeKind::Assignment => (Kind::Op, "=".into()),
            NodeKind::IndexAccess => (Kind::Op, "index".into()),
            NodeKind::RecordConstruction => (Kind::Op, "struct".into()),
            NodeKind::Tuple => (Kind::Op, "tuple".into()),
            NodeKind::Array => (Kind::Op, "array".into()),
            NodeKind::Cast => (Kind::Cast, String::new()),
            _ => (Kind::Op, "expr".into()),
        };
        Ok(PdgNode {
            detail,
            ..self.typed(kind, node)?
        })
    }
}

/// Kinds that are not expressions: statements, patterns and the parts of a match arm.
fn is_expression(node: &Node) -> bool {
    !matches!(
        node.kind,
        NodeKind::LocalDeclaration
            | NodeKind::ExpressionStatement
            | NodeKind::ItemDeclaration
            | NodeKind::TypeAnnotation
            | NodeKind::MatchArm
            | NodeKind::MatchGuard
            | NodeKind::PatternBinding
            | NodeKind::PatternAlias
            | NodeKind::PatternWildcard
            | NodeKind::PatternLiteral
            | NodeKind::PatternRange
            | NodeKind::PatternOr
            | NodeKind::PatternConstructor
            | NodeKind::PatternTuple
            | NodeKind::PatternSequence
    )
}

/// The token of a neutral operator, as the graph displays it.
fn symbol(operator: &str, operands: usize) -> &str {
    match (operator, operands) {
        ("add", _) => "+",
        ("sub", _) | ("neg", _) => "-",
        ("mul", _) => "*",
        ("div", _) => "/",
        ("rem", _) => "%",
        ("and", _) => "&&",
        ("or", _) => "||",
        ("bit_xor", _) => "^",
        ("bit_and", _) => "&",
        ("bit_or", _) => "|",
        ("shl", _) => "<<",
        ("shr", _) => ">>",
        ("eq", _) => "==",
        ("ne", _) => "!=",
        ("lt", _) => "<",
        ("le", _) => "<=",
        ("gt", _) => ">",
        ("ge", _) => ">=",
        ("not", _) => "!",
        (other, _) => other,
    }
}

fn literal_kind(value: Option<&Literal>) -> &'static str {
    match value.map(|value| value.kind.as_str()) {
        Some("string") => "str",
        Some("integer") => "int",
        Some("float") => "float",
        Some("bool") => "bool",
        Some("char") => "char",
        _ => "other",
    }
}

/// The literal's value, never its source text.
fn literal_detail(value: Option<&Literal>) -> String {
    let Some(Literal {
        kind,
        value: Some(text),
        ..
    }) = value
    else {
        return String::new();
    };
    match kind.as_str() {
        "string" => clip(&format!("{text:?}"), 24),
        "integer" | "bool" => text.clone(),
        "char" => text
            .chars()
            .next()
            .map(|c| format!("{c:?}"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}
