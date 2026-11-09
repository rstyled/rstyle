//! Runtime-bound programs lowered from a function's document (the profile's
//! Runtime bounds). The lowering is backward: each construct is lowered so that it
//! continues at `next`, and returns its entry node.

mod calls;
mod canonical;
mod chains;
mod control;
mod dispatch;
mod extent;
pub mod facts;
mod frame;
mod iteration;
mod primitive;
pub mod report;

use super::model::{Node, NodeKind};
use crate::advisory::runtime_bounds::{Input, MAX_NODES, MAX_VARIABLES};
use crate::runtime_bounds::{Action, Arithmetic, Flow, Integer, Program, Value, Variable};
use crate::wording::Message;
pub use canonical::canonical_bounds;
use facts::Facts;
use std::collections::BTreeMap;

/// The runtime-bound program of one function.
pub fn lower(facts: &Facts<'_, '_>, arithmetic: Arithmetic) -> Result<Input, String> {
    let mut collector = Collector {
        facts,
        program: Program {
            inputs: Vec::new(),
            variables: Vec::new(),
            nodes: Vec::new(),
            entry: 0,
            loop_headers: Vec::new(),
            formals: Vec::new(),
            extents: Vec::new(),
            arithmetic,
        },
        numbers: facts.symbol_numbers(),
        locals: BTreeMap::new(),
        loops: Vec::new(),
        exhausted: false,
        depth: 0,
        work: 0,
        extents: BTreeMap::new(),
        closures: BTreeMap::new(),
        returns: Vec::new(),
        formals: BTreeMap::new(),
    };

    collector.collect()
}

pub(crate) struct Collector<'f, 'b, 'a> {
    facts: &'f Facts<'b, 'a>,
    program: Program,
    /// A stable number for each symbol, standing in for the frontend's own.
    numbers: BTreeMap<&'a str, u32>,
    locals: BTreeMap<&'a str, u32>,
    /// Enclosing modeled loops: owner node, exit and repeat destinations.
    loops: Vec<(&'a str, (u32, u32))>,
    exhausted: bool,
    depth: usize,
    work: usize,
    extents: BTreeMap<String, String>,
    closures: BTreeMap<&'a str, &'a Node>,
    returns: Vec<u32>,
    formals: BTreeMap<&'a str, u32>,
}

impl<'a> Collector<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn collect(&mut self) -> Result<Input, String> {
        let parameters = self.parameter_symbols()?;
        if parameters.len() > MAX_VARIABLES {
            return Ok(Input::BudgetExhausted {
                reason: self
                    .facts
                    .tables
                    .wording()
                    .text(Message::ParameterBudget)
                    .into(),
            });
        }
        let frame = frame::inspect(self.facts, &parameters)?;
        if frame.exhausted {
            return Ok(Input::BudgetExhausted {
                reason: self
                    .facts
                    .tables
                    .wording()
                    .text(Message::FrameBudget)
                    .into(),
            });
        }
        self.install_extents(&parameters, frame.extents)?;
        self.parameters(&parameters)?;
        let root = self.facts.body.root()?;
        match frame.reason {
            Some(reason) => self.reject_body(root, &reason),
            None => self.capture_body(root)?,
        }
        if self.exhausted {
            return Ok(Input::BudgetExhausted {
                reason: self
                    .facts
                    .tables
                    .wording()
                    .text(Message::CaptureBudget)
                    .into(),
            });
        }

        Ok(Input::Captured {
            program: std::mem::replace(&mut self.program, empty(Arithmetic::Unknown)),
        })
    }

    fn reject_body(&mut self, root: &'a Node, reason: &str) {
        let end = self.node(Action::Nop, Flow::Return, root);
        let entry = self.unsupported(root, end, reason);
        self.program.entry = entry;
    }

    fn capture_body(&mut self, root: &'a Node) -> Result<(), String> {
        let end = self.node(Action::Nop, Flow::Return, root);
        let entry = self.expr(root, end)?;
        self.program.entry = entry;
        Ok(())
    }

    fn number(&self, symbol: &str) -> u32 {
        self.numbers.get(symbol).copied().unwrap_or(u32::MAX)
    }

    fn variable(&mut self, name: String, initial: Value, integer: Integer) -> u32 {
        let variable = Variable {
            name,
            initial,
            bits: integer.bits,
            signed: integer.signed,
        };
        bounded_push(
            &mut self.program.variables,
            variable,
            MAX_VARIABLES,
            &mut self.exhausted,
        )
    }

    fn node_with(&mut self, (action, flow): (Action, Flow), at: &Node, evidence: &str) -> u32 {
        let node = crate::runtime_bounds::Node {
            action,
            flow,
            source: self.facts.source(at, evidence),
        };
        bounded_push(
            &mut self.program.nodes,
            node,
            MAX_NODES,
            &mut self.exhausted,
        )
    }

    fn node(&mut self, action: Action, flow: Flow, at: &Node) -> u32 {
        let evidence = self.facts.tables.wording().text(Message::BoundEvidence);
        self.node_with((action, flow), at, evidence)
    }

    fn unsupported(&mut self, at: &Node, next: u32, reason: &str) -> u32 {
        let action = Action::Unsupported {
            reason: reason.into(),
        };
        self.node(action, Flow::Goto(next), at)
    }

    // Blocks and statements.

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn block(&mut self, block: &'a Node, next: u32) -> Result<u32, String> {
        let (statements, tail) = self.statements(block)?;
        if !self.charge(statements.len()) {
            return Ok(next);
        }
        self.register_locals(&statements)?;
        let mut entry = match tail {
            Some(value) => self.expr(value, next)?,
            None => next,
        };
        for statement in statements.iter().rev() {
            entry = self.statement(statement, entry)?;
        }

        Ok(entry)
    }

    /// A block's statements and its tail expression.
    fn statements(&self, block: &'a Node) -> Result<(Vec<&'a Node>, Option<&'a Node>), String> {
        let mut statements = self.facts.body.children(block)?;
        let tail = match statements.last() {
            Some(last) if !is_statement(last) => statements.pop(),
            _ => None,
        };
        Ok((statements, tail))
    }

    // Register before reverse lowering, retaining lexical assignment order.
    fn register_locals(&mut self, statements: &[&'a Node]) -> Result<(), String> {
        for statement in statements {
            if statement.kind != NodeKind::LocalDeclaration {
                continue;
            }
            match self.local_closure(statement)? {
                Some((symbol, closure)) => _ = self.closures.insert(symbol, closure),
                None => self.register_local(statement)?,
            }
        }
        Ok(())
    }

    fn register_local(&mut self, local: &'a Node) -> Result<(), String> {
        let pattern = self.facts.body.child(local, 0)?;
        if let Some(symbol) = self.by_value_binding(pattern)
            && let Some(integer) = self.facts.integer(self.facts.ty(pattern))
        {
            let name = self.facts.body.symbol(symbol)?.name.clone();
            let variable = self.variable(name, Value::Unknown, integer);
            self.locals.insert(symbol, variable);
        }
        Ok(())
    }

    /// The symbol of a binding pattern that binds by value, without a subpattern.
    fn by_value_binding(&self, pattern: &'a Node) -> Option<&'a str> {
        (pattern.kind == NodeKind::PatternBinding
            && pattern.children.is_empty()
            && pattern.binding_mode.as_deref().unwrap_or("value") == "value")
            .then(|| self.declared(pattern))
            .flatten()
    }

    fn declared(&self, pattern: &Node) -> Option<&'a str> {
        let symbols = &self.facts.body.function.symbols;
        symbols
            .iter()
            .find(|symbol| symbol.node.as_deref() == Some(pattern.id.as_str()))
            .map(|symbol| symbol.id.as_str())
    }

    /// `let f = |..| ..;` with an immutable by-value binding and no else.
    fn local_closure(&self, local: &'a Node) -> Result<Option<(&'a str, &'a Node)>, String> {
        let parts = self.local_parts(local)?;
        let Some(symbol) = self.by_value_binding(parts.pattern) else {
            return Ok(None);
        };
        if self.facts.body.symbol(symbol)?.mutable || parts.otherwise.is_some() {
            return Ok(None);
        }
        Ok(parts
            .init
            .filter(|init| {
                init.kind == NodeKind::Closure && !init.modifiers.iter().any(|m| m == "coroutine")
            })
            .map(|init| (symbol, init)))
    }

    fn local_parts(&self, local: &'a Node) -> Result<Local<'a>, String> {
        let parts = self.facts.body.children(local)?;
        let pattern = *parts
            .first()
            .ok_or_else(|| format!("Declaration {} has no pattern", local.id))?;
        let rest: Vec<&'a Node> = parts[1..]
            .iter()
            .copied()
            .filter(|part| part.kind != NodeKind::TypeAnnotation)
            .collect();
        Ok(Local {
            pattern,
            init: rest.first().copied(),
            otherwise: rest.get(1).copied(),
        })
    }

    fn statement(&mut self, statement: &'a Node, next: u32) -> Result<u32, String> {
        match statement.kind {
            NodeKind::ExpressionStatement => self.expr(self.facts.body.child(statement, 0)?, next),
            NodeKind::LocalDeclaration => self.local(statement, next),
            _ => Ok(next),
        }
    }

    /// What a pattern holds besides new bindings: the places it writes, in a language
    /// whose patterns can reuse a binding.
    pub(super) fn written_places(&self, pattern: &'a Node) -> Result<Vec<&'a Node>, String> {
        let mut pending = vec![pattern];
        let mut places = Vec::new();
        while let Some(part) = pending.pop() {
            match part.kind {
                // These hold values they compare, and write nothing.
                NodeKind::PatternLiteral | NodeKind::PatternRange => {}
                kind if frame::is_pattern(kind) => {
                    pending.extend(self.facts.body.children(part)?);
                }
                _ => places.push(part),
            }
        }
        Ok(places)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// What a declaration's pattern writes, once its value is computed. Only a counter or
    /// an extent source can change a bound, so a write to anything else is no event.
    fn declaration_writes(&mut self, pattern: &'a Node, next: u32) -> Result<u32, String> {
        let mut entry = next;
        for place in self.written_places(pattern)? {
            entry = match self.local_id(place) {
                Some(variable) => {
                    let action = Action::Assign {
                        variable,
                        value: Value::Unknown,
                    };
                    self.node(action, Flow::Goto(entry), place)
                }
                None if self.is_extent_source(place) => {
                    self.unsupported(place, entry, "Declaration writes an extent source")
                }
                None => entry,
            };
        }
        Ok(entry)
    }

    fn is_extent_source(&self, place: &Node) -> bool {
        self.facts.local(place).is_some_and(|symbol| {
            let root = format!("{symbol}:");
            self.extents.keys().any(|key| key.starts_with(&root))
        })
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn local(&mut self, local: &'a Node, next: u32) -> Result<u32, String> {
        let parts = self.local_parts(local)?;
        let next = self.declaration_writes(parts.pattern, next)?;
        // Immutable local closures run only where they are directly invoked.
        if let Some(symbol) = self.binding_symbol(parts.pattern)
            && self.closures.contains_key(symbol)
        {
            return Ok(next);
        }
        if parts.otherwise.is_some() {
            return Ok(self.unsupported(local, next, "let-else control flow is unmodeled"));
        }
        let variable = self
            .binding_symbol(parts.pattern)
            .and_then(|symbol| self.locals.get(symbol).copied());

        match (variable, parts.init) {
            (Some(variable), init) => {
                let value = init.map_or(Value::Unknown, |init| self.value(init));
                let assignment =
                    self.node(Action::Assign { variable, value }, Flow::Goto(next), local);
                match init {
                    Some(init) => self.expr(init, assignment),
                    None => Ok(assignment),
                }
            }
            (None, Some(init)) => self.expr(init, next),
            (None, None) => Ok(next),
        }
    }

    /// The symbol of a binding pattern without a subpattern, whatever its mode.
    fn binding_symbol(&self, pattern: &'a Node) -> Option<&'a str> {
        (pattern.kind == NodeKind::PatternBinding && pattern.children.is_empty())
            .then(|| self.declared(pattern))
            .flatten()
    }

    // Expressions.

    fn expr(&mut self, expression: &'a Node, next: u32) -> Result<u32, String> {
        if !self.enter_expression() {
            return Ok(next);
        }
        let result = self.lower(expression, next);
        self.depth -= 1;
        result
    }

    fn enter_expression(&mut self) -> bool {
        if !self.charge(1) {
            return false;
        }
        if self.depth >= 256 {
            self.exhausted = true;
            return false;
        }
        self.depth += 1;
        true
    }

    fn charge(&mut self, work: usize) -> bool {
        self.work = self.work.saturating_add(work);
        if self.exhausted || self.work > MAX_NODES * 4 {
            self.exhausted = true;
            return false;
        }
        true
    }
}

struct Local<'a> {
    pattern: &'a Node,
    init: Option<&'a Node>,
    otherwise: Option<&'a Node>,
}

/// Appends `item` and returns its index, unless `items` is full: then the program's
/// budget is exhausted and 0 stands in for the index.
fn bounded_push<T>(items: &mut Vec<T>, item: T, limit: usize, exhausted: &mut bool) -> u32 {
    if items.len() >= limit {
        *exhausted = true;
        return 0;
    }
    items.push(item);
    items.len() as u32 - 1
}

fn is_statement(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LocalDeclaration | NodeKind::ExpressionStatement | NodeKind::ItemDeclaration
    )
}

fn empty(arithmetic: Arithmetic) -> Program {
    Program {
        inputs: Vec::new(),
        variables: Vec::new(),
        nodes: Vec::new(),
        entry: 0,
        loop_headers: Vec::new(),
        formals: Vec::new(),
        extents: Vec::new(),
        arithmetic,
    }
}
