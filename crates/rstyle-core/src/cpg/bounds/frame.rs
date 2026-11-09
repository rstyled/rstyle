//! Safe private-local frame: unrelated calls cannot write unescaped scalars. Decides
//! the stable extents a body measures, or why the whole body is unmodeled.

use super::super::model::{Node, NodeKind, TypeKind};
use super::calls::Parameter;
use super::extent::{
    Place, extent_receiver, extent_type, has_role, is_method, iteration_receiver, place,
};
use super::facts::Facts;
use crate::advisory::runtime_bounds::{MAX_NODES, MAX_VARIABLES};
use crate::runtime_bounds::contracts::ExtentRole;
use crate::wording::{Message, Wording};
use std::collections::{BTreeMap, BTreeSet};

pub struct Frame<'a> {
    pub extents: Vec<Place<'a>>,
    pub reason: Option<String>,
    pub exhausted: bool,
}

pub(super) fn inspect<'a>(
    facts: &Facts<'_, 'a>,
    parameters: &[Parameter<'a>],
) -> Result<Frame<'a>, String> {
    Inspect::new(facts).run(parameters)
}

struct Inspect<'f, 'b, 'a> {
    facts: &'f Facts<'b, 'a>,
    numbers: BTreeMap<&'a str, u32>,
    stable: BTreeMap<&'a str, bool>,
    candidates: Vec<&'a Node>,
    // Optional extents are selected only when stable; unmodeled uses stay unknown.
    optional: Vec<&'a Node>,
    callbacks: BTreeSet<&'a str>,
    closures: usize,
    selected: Vec<Place<'a>>,
    reason: Option<String>,
    work: usize,
    depth: usize,
    loops: usize,
    validating: bool,
    exhausted: bool,
}

type Visited = Result<(), String>;

impl<'f, 'b, 'a> Inspect<'f, 'b, 'a> {
    fn new(facts: &'f Facts<'b, 'a>) -> Self {
        let signature = &facts.body.function.signature;
        let forbidden = signature
            .modifiers
            .iter()
            .any(|m| m == "unsafe" || m == "rust:foreign-abi");

        Inspect {
            facts,
            numbers: facts.symbol_numbers(),
            stable: BTreeMap::new(),
            candidates: Vec::new(),
            optional: Vec::new(),
            callbacks: BTreeSet::new(),
            closures: 0,
            selected: Vec::new(),
            reason: forbidden.then(|| "Unsafe or foreign owner signature is unmodeled".into()),
            work: 0,
            depth: 0,
            loops: 0,
            validating: false,
            exhausted: false,
        }
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    /// Observes the body, selects its stable extents, then validates the body against them.
    fn run(mut self, parameters: &[Parameter<'a>]) -> Result<Frame<'a>, String> {
        for parameter in parameters {
            self.parameter(parameter)?;
        }
        let root = self.facts.body.root()?;
        self.visit(root)?;
        if !self.exhausted {
            self.select_extents()?;
        }
        self.validating = true;
        if !self.exhausted {
            self.visit(root)?;
        }

        Ok(Frame {
            extents: self.selected,
            reason: self.reason,
            exhausted: self.exhausted,
        })
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn parameter(&mut self, parameter: &Parameter<'a>) -> Visited {
        if !self.enter() {
            return Ok(());
        }
        if let Some(symbol) = parameter.symbol {
            self.stable.insert(symbol, true);
            let ty = self
                .facts
                .body
                .symbol(symbol)?
                .ty
                .as_deref()
                .and_then(|ty| self.facts.type_of(ty));
            if ty.is_some_and(|ty| self.mutable_integer_reference(ty)) {
                self.reason = Some(self.wording().text(Message::WritableNumberReference).into());
            }
        }
        self.depth -= 1;
        Ok(())
    }

    fn select_extents(&mut self) -> Visited {
        let mut keys = BTreeSet::new();
        let required = self.candidates.iter().map(|e| (*e, true));
        let optional = self.optional.iter().map(|e| (*e, false));
        for (expression, required) in required.chain(optional).collect::<Vec<_>>() {
            let Some(place) = place(self.facts, expression, &self.numbers)? else {
                continue;
            };
            let stable = self.stable.get(place.root) == Some(&true);
            if !required && !stable {
                continue;
            }
            if !stable {
                self.reason = Some(
                    "Extent source is not a stable formal or immutable local outside loops".into(),
                );
            }
            if keys.insert(place.key()) {
                self.selected.push(place);
            }
            if self.selected.len() > MAX_VARIABLES {
                self.exhausted = true;
                return Ok(());
            }
        }

        Ok(())
    }

    fn enter(&mut self) -> bool {
        if self.exhausted {
            return false;
        }

        self.work += 1;
        if self.work > MAX_NODES * 4 || self.depth >= 256 {
            self.exhausted = true;
            return false;
        }
        self.depth += 1;
        true
    }

    fn enter_roster(&mut self, width: usize) -> bool {
        if !self.enter() {
            return false;
        }
        if width > MAX_NODES * 4 {
            self.exhausted = true;
            return false;
        }
        true
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    /// Visits a node as rustc's HIR visitor reaches the construct it stands for.
    fn visit(&mut self, node: &'a Node) -> Visited {
        match node.kind {
            NodeKind::ExpressionStatement | NodeKind::MatchArm | NodeKind::MatchGuard => {
                self.children(node)
            }
            NodeKind::ItemDeclaration => self.nested_function(node),
            NodeKind::TypeAnnotation => Ok(()),
            NodeKind::LocalDeclaration => self.local(node),
            kind if is_pattern(kind) => self.pattern(node),
            NodeKind::Extension => self.pattern(node),
            NodeKind::Unsupported
                if node
                    .reason
                    .as_deref()
                    .is_some_and(|r| r.starts_with("Pattern")) =>
            {
                self.pattern(node)
            }
            _ => self.expression(node),
        }
    }

    fn children(&mut self, node: &'a Node) -> Visited {
        for child in self.facts.body.children(node)? {
            self.visit(child)?;
        }
        Ok(())
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn expression(&mut self, expression: &'a Node) -> Visited {
        if expression.kind == NodeKind::Block && !self.bare_block(expression) {
            // A block expression is an expression around its block.
            if !self.enter_roster(0) {
                return Ok(());
            }
            self.observe(expression)?;
            let result = self.block(expression);
            self.depth -= 1;
            return result;
        }
        if expression.kind == NodeKind::Block {
            return self.block(expression);
        }
        if !self.enter_roster(width(self.facts, expression)?) {
            return Ok(());
        }

        self.observe(expression)?;
        let looping = usize::from(matches!(expression.kind, NodeKind::Loop | NodeKind::While));
        self.loops += looping;
        self.descend(expression)?;
        self.loops -= looping;
        self.depth -= 1;
        Ok(())
    }

    /// A block that is not an expression of its own: a loop body or a lowering's block.
    fn bare_block(&self, block: &Node) -> bool {
        block.synthetic
            || self
                .facts
                .body
                .function
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Loop && node.children.first() == Some(&block.id))
    }

    fn block(&mut self, block: &'a Node) -> Visited {
        let statements = self
            .facts
            .body
            .children(block)?
            .iter()
            .filter(|child| super::is_statement(child))
            .count();
        if !self.enter_roster(statements) {
            return Ok(());
        }
        self.children(block)?;
        self.depth -= 1;

        Ok(())
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn observe(&mut self, expression: &'a Node) -> Visited {
        self.admit_callbacks(expression)?;
        if self.forbidden(expression)? || self.unadmitted_closure(expression) {
            self.reason = Some(self.wording().text(Message::UnmodeledSemantics).into());
        }
        if self.validating {
            self.check_borrow(expression)
        } else {
            self.record_candidate(expression)
        }
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn descend(&mut self, expression: &'a Node) -> Visited {
        match expression.kind {
            NodeKind::Closure => self.closure_body(expression),
            NodeKind::Assignment | NodeKind::CompoundAssignment | NodeKind::PatternCondition => {
                // Rust's HIR visitor reaches the value before the place or pattern.
                let mut children = self.facts.body.children(expression)?;
                children.reverse();
                for child in children {
                    self.visit(child)?;
                }
                Ok(())
            }
            NodeKind::For => self.for_loop(expression),
            _ => self.children(expression),
        }
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    /// The lowering's order: the iterable, the iterator, then the loop with the
    /// pattern and body inside it.
    fn for_loop(&mut self, expression: &'a Node) -> Visited {
        let children = self.facts.body.children(expression)?;
        let [pattern, iterable, body, rest @ ..] = children.as_slice() else {
            return self.children(expression);
        };
        self.visit(iterable)?;
        let (iterator, lowering) = match rest {
            [iterator, lowering, ..] => (Some(*iterator), Some(*lowering)),
            [iterator] => (Some(*iterator), None),
            [] => (None, None),
        };
        if let Some(iterator) = iterator {
            self.visit(iterator)?;
        }
        self.loops += 1;
        if let Some(lowering) = lowering {
            self.visit(lowering)?;
        }
        self.visit(pattern)?;
        self.visit(body)?;
        self.loops -= 1;

        Ok(())
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn local(&mut self, local: &'a Node) -> Visited {
        if !self.enter() {
            return Ok(());
        }

        let parts = self.facts.body.children(local)?;
        let pattern = parts.first().copied();
        let rest: Vec<&'a Node> = parts
            .iter()
            .skip(1)
            .copied()
            .filter(|part| part.kind != NodeKind::TypeAnnotation)
            .collect();
        let (init, otherwise) = (rest.first().copied(), rest.get(1).copied());
        if let Some(pattern) = pattern {
            self.record_local(pattern, init, otherwise)?;
        }
        if let Some(init) = init {
            self.visit(init)?;
        }
        if let Some(pattern) = pattern {
            self.visit(pattern)?;
        }
        if let Some(otherwise) = otherwise {
            self.visit(otherwise)?;
        }
        self.depth -= 1;
        Ok(())
    }

    fn record_local(
        &mut self,
        pattern: &'a Node,
        init: Option<&'a Node>,
        otherwise: Option<&'a Node>,
    ) -> Visited {
        let Some(symbol) = self.immutable_binding(pattern)? else {
            return Ok(());
        };
        if let Some(init) = init
            && otherwise.is_none()
            && init.kind == NodeKind::Closure
        {
            self.callbacks.insert(init.id.as_str());
        }
        self.stable
            .insert(symbol, self.loops == 0 && init.is_some());
        Ok(())
    }

    fn immutable_binding(&self, pattern: &'a Node) -> Result<Option<&'a str>, String> {
        if pattern.kind != NodeKind::PatternBinding
            || !pattern.children.is_empty()
            || pattern.binding_mode.as_deref().unwrap_or("value") != "value"
        {
            return Ok(None);
        }
        let symbols = &self.facts.body.function.symbols;
        let symbol = symbols
            .iter()
            .find(|symbol| symbol.node.as_deref() == Some(pattern.id.as_str()));
        Ok(symbol
            .filter(|symbol| !symbol.mutable)
            .map(|symbol| symbol.id.as_str()))
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn pattern(&mut self, pattern: &'a Node) -> Visited {
        if !self.enter_roster(pattern_width(pattern)) {
            return Ok(());
        }

        if let Some(reason) = self.pattern_problem(pattern) {
            self.reason = Some(reason.into());
        }
        self.children(pattern)?;
        self.depth -= 1;
        Ok(())
    }

    fn pattern_problem(&self, pattern: &Node) -> Option<&'static str> {
        let embedded = matches!(
            pattern.reason.as_deref(),
            Some("Pattern guards are not modeled" | "Dereference patterns are not modeled")
        );
        if embedded {
            return Some("Embedded pattern guards and dereference-pattern effects are unmodeled");
        }
        let implicit = pattern
            .adjustments
            .iter()
            .flatten()
            .any(|adjustment| adjustment.kind != "auto_deref");
        if implicit {
            return Some("Implicit dereference-pattern effects are unmodeled");
        }
        if pattern.binding_mode.as_deref() == Some("exclusive") {
            return Some(self.wording().text(Message::WritableReferenceBinding));
        }
        if self
            .facts
            .ty(pattern)
            .is_some_and(|ty| self.mutable_integer_reference(ty))
        {
            return Some(self.wording().text(Message::WritableNumberReference));
        }

        None
    }

    fn wording(&self) -> Wording {
        self.facts.tables.wording()
    }

    fn mutable_integer_reference(&self, ty: &super::super::model::Type) -> bool {
        ty.kind == TypeKind::Reference
            && ty.mode.as_deref() == Some("exclusive")
            && ty
                .target
                .as_deref()
                .and_then(|id| self.facts.type_of(id))
                .is_some_and(|inner| self.integer_reference(inner))
    }

    fn integer_reference(&self, mut ty: &'a super::super::model::Type) -> bool {
        while ty.kind == TypeKind::Reference {
            match ty.target.as_deref().and_then(|id| self.facts.type_of(id)) {
                Some(inner) => ty = inner,
                None => return false,
            }
        }
        ty.kind == TypeKind::Integer
    }

    // Callbacks are admitted only where lowering can resolve every invocation.
    fn admit_callbacks(&mut self, expression: &'a Node) -> Visited {
        if !is_method(self.facts, expression) || !iteration_member(self.facts, expression) {
            return Ok(());
        }
        for argument in &self.facts.body.children(expression)?[1..] {
            if argument.kind == NodeKind::Closure {
                self.callbacks.insert(argument.id.as_str());
            }
        }
        Ok(())
    }

    fn unadmitted_closure(&self, expression: &Node) -> bool {
        expression.kind == NodeKind::Closure
            && (!self.callbacks.contains(expression.id.as_str())
                || expression.modifiers.iter().any(|m| m == "coroutine"))
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn closure_body(&mut self, closure: &'a Node) -> Visited {
        if self.unadmitted_closure(closure) {
            return Ok(());
        }

        if self.validating {
            self.check_captures(closure)?;
        }
        // Each invocation re-evaluates the body, so its bindings are not stable extents.
        self.loops += 1;
        self.closures += 1;
        let result = self.children(closure);
        self.loops -= 1;
        self.closures -= 1;
        result
    }

    /// A function declared here runs when it's called, and isn't lowered as part of this body, but
    /// what it can write is still written. Most declarations capture nothing.
    fn nested_function(&mut self, declaration: &'a Node) -> Visited {
        if self.validating {
            self.check_captures(declaration)?;
        }
        Ok(())
    }

    fn check_captures(&mut self, closure: &'a Node) -> Visited {
        for capture in closure.captures.iter().flatten() {
            let problem = match capture.mode.as_str() {
                "shared" => None,
                "exclusive" | "unique" => {
                    let numeric = self
                        .facts
                        .type_of(&capture.ty)
                        .is_some_and(|ty| self.integer_reference(ty));
                    let selected = self
                        .selected
                        .iter()
                        .any(|place| place.root == capture.symbol);
                    (numeric || selected).then_some(self.wording().text(Message::WrittenCapture))
                }
                _ => self
                    .facts
                    .body
                    .symbol(&capture.symbol)
                    .map_or(true, |symbol| symbol.mutable)
                    .then_some(self.wording().text(Message::CopiedCapture)),
            };
            if let Some(reason) = problem {
                self.reason = Some(reason.into());
            }
        }
        Ok(())
    }

    // rstyle-allow(mutation_per_target): the inspection accumulates frame facts in self by design
    fn record_candidate(&mut self, expression: &'a Node) -> Visited {
        if let Some(receiver) = self.candidate(expression)? {
            if self.closures > 0 {
                self.optional.push(receiver);
            } else {
                self.candidates.push(receiver);
            }
            return Ok(());
        }
        if is_method(self.facts, expression)
            && expression.children.len() == 1
            && has_role(self.facts, expression, ExtentRole::Traversal)
        {
            self.optional.push(self.facts.body.child(expression, 0)?);
        }
        // Slice and `Vec` arguments may measure a callee's size during composition.
        if expression.kind == NodeKind::Call {
            let skip = usize::from(!is_method(self.facts, expression));
            let arguments = self.facts.body.children(expression)?;
            let arguments = if is_method(self.facts, expression) {
                &arguments[1..]
            } else {
                &arguments[skip..]
            };
            for argument in arguments {
                if self
                    .facts
                    .adjusted(argument)
                    .is_some_and(|ty| extent_type(self.facts, ty))
                {
                    self.optional.push(argument);
                }
            }
        }
        Ok(())
    }

    fn candidate(&self, expression: &'a Node) -> Result<Option<&'a Node>, String> {
        if let Some(receiver) = extent_receiver(self.facts, expression)? {
            return Ok(Some(receiver));
        }
        match super::iteration::for_head(self.facts, expression)? {
            Some(head) => iteration_receiver(self.facts, head),
            None => Ok(None),
        }
    }

    fn check_borrow(&mut self, expression: &'a Node) -> Visited {
        let explicit = expression.kind == NodeKind::Reference
            && expression
                .modifiers
                .iter()
                .any(|m| m == "mutable" || m == "raw");
        let adjusted = expression.adjustments.iter().flatten().any(|adjustment| {
            matches!(
                (adjustment.kind.as_str(), adjustment.mode.as_deref()),
                ("auto_ref", Some("exclusive" | "raw"))
                    | ("pin", _)
                    | ("reborrow", Some("exclusive"))
            )
        });
        let operand = match expression.kind {
            NodeKind::Reference => self.facts.body.child(expression, 0)?,
            _ => expression,
        };
        if (explicit || adjusted) && self.sensitive(operand)? {
            self.reason = Some(self.wording().text(Message::WritableReference).into());
        }

        Ok(())
    }

    fn sensitive(&self, expression: &'a Node) -> Result<bool, String> {
        if self
            .facts
            .ty(expression)
            .is_some_and(|ty| self.integer_reference(ty))
        {
            return Ok(true);
        }
        let Some(place) = self.borrow_place(expression, 256)? else {
            return Ok(false);
        };
        Ok(self
            .selected
            .iter()
            .any(|selected| selected.overlaps(&place)))
    }

    fn borrow_place(
        &self,
        expression: &'a Node,
        remaining: usize,
    ) -> Result<Option<Place<'a>>, String> {
        if remaining == 0 {
            return Ok(None);
        }
        Ok(match expression.kind {
            NodeKind::Identifier => self.facts.local(expression).map(|root| Place {
                root,
                fields: Vec::new(),
                path: Vec::new(),
                label: String::new(),
            }),
            NodeKind::FieldAccess => {
                match self.borrow_place(self.facts.body.child(expression, 0)?, remaining - 1)? {
                    Some(mut place) => {
                        place
                            .fields
                            .push(expression.member_index.unwrap_or_default() as usize);
                        Some(place)
                    }
                    None => None,
                }
            }
            NodeKind::Operator if expression.operator.as_deref() == Some("deref") => {
                self.borrow_place(self.facts.body.child(expression, 0)?, remaining - 1)?
            }
            NodeKind::Reference | NodeKind::IndexAccess => {
                self.borrow_place(self.facts.body.child(expression, 0)?, remaining - 1)?
            }
            _ => None,
        })
    }

    fn forbidden(&self, expression: &Node) -> Result<bool, String> {
        let reason = expression.reason.as_deref();
        if matches!(
            reason,
            Some("Inline assembly is not modeled" | "Unsafe binder casts are not modeled")
        ) || expression.kind == NodeKind::Yield
        {
            return Ok(true);
        }
        if expression.kind == NodeKind::Block
            && expression
                .modifiers
                .iter()
                .any(|m| m == "unsafe" || m == "rust:compiler-unsafe")
        {
            return Ok(true);
        }
        if expression.kind != NodeKind::Call {
            return Ok(false);
        }
        if is_method(self.facts, expression) {
            let method = self
                .facts
                .reference(expression)
                .and_then(|reference| self.facts.declared(reference));
            return Ok(method.is_some_and(|method| foreign_signature(&callable_modifiers(method))));
        }
        let callee = self.facts.body.child(expression, 0)?;
        let Some(ty) = self.facts.ty(callee) else {
            return Ok(false);
        };
        if ty.kind != TypeKind::Function {
            return Ok(false);
        }

        Ok(match ty.definition.as_deref() {
            Some(id) => {
                let definition = self.facts.tables.definition(id)?;
                let modifiers = callable_modifiers(definition);
                foreign_signature(&modifiers) || modifiers.iter().any(|m| m == "foreign")
            }
            None => foreign_signature(&ty.modifiers),
        })
    }
}

fn callable_modifiers(definition: &super::super::model::Definition) -> Vec<String> {
    definition
        .callable
        .as_ref()
        .map(|callable| callable.modifiers.clone())
        .unwrap_or_default()
}

fn foreign_signature(modifiers: &[String]) -> bool {
    modifiers
        .iter()
        .any(|m| m == "unsafe" || m == "rust:foreign-abi")
}

fn iteration_member(facts: &Facts<'_, '_>, call: &Node) -> bool {
    facts
        .reference(call)
        .and_then(|reference| facts.declared(reference))
        .is_some_and(|declared| declared.iteration && declared.operation.is_some())
}

pub(super) fn is_pattern(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::PatternBinding
            | NodeKind::PatternAlias
            | NodeKind::PatternWildcard
            | NodeKind::PatternLiteral
            | NodeKind::PatternRange
            | NodeKind::PatternOr
            | NodeKind::PatternConstructor
            | NodeKind::PatternTuple
            | NodeKind::PatternSequence
    ) || false
}

fn width(facts: &Facts<'_, '_>, expression: &Node) -> Result<usize, String> {
    Ok(match expression.kind {
        NodeKind::Call => expression.children.len().saturating_sub(1),
        NodeKind::Tuple | NodeKind::Array | NodeKind::RecordConstruction => expression
            .members
            .as_ref()
            .map_or(expression.children.len(), Vec::len),
        NodeKind::Match => expression.children.len().saturating_sub(1),
        _ => {
            let _ = facts;
            0
        }
    })
}

fn pattern_width(pattern: &Node) -> usize {
    match pattern.kind {
        NodeKind::PatternTuple | NodeKind::PatternConstructor | NodeKind::PatternOr => {
            pattern.children.len()
        }
        NodeKind::PatternSequence => pattern.children.len(),
        _ => 0,
    }
}
