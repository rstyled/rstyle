//! Operators, aggregates and values: everything without control flow of its own.

use super::super::model::{Node, NodeKind};
use super::Collector;
use super::facts::Facts;
use crate::runtime_bounds::{Action, Condition, Flow, Value};
use crate::wording::Message;

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    pub(super) fn primitive(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let operator = expression.operator.as_deref();
        match expression.kind {
            NodeKind::Operator if matches!(operator, Some("and" | "or")) => {
                self.short_circuit(expression, next)
            }
            NodeKind::Operator if expression.children.len() == 2 => self.binary(expression, next),
            NodeKind::IndexAccess => self.binary(expression, next),
            NodeKind::Operator | NodeKind::Cast | NodeKind::FieldAccess | NodeKind::Reference => {
                self.operand(expression, next)
            }
            NodeKind::Tuple | NodeKind::Array => {
                let items = self.facts.body.children(expression)?;
                self.arguments(&items, next)
            }
            NodeKind::RecordConstruction => self.structure(expression, next),
            NodeKind::PatternCondition => self.expr(self.facts.body.child(expression, 1)?, next),
            NodeKind::Identifier => Ok(self.path(expression, next)),
            NodeKind::Literal => Ok(next),
            _ => {
                let reason = self
                    .facts
                    .tables
                    .wording()
                    .text(Message::UnmodeledExpression);
                Ok(self.unsupported(expression, next, reason))
            }
        }
    }

    fn short_circuit(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let left = self.facts.body.child(expression, 0)?;
        let right = self.expr(self.facts.body.child(expression, 1)?, next)?;
        let branch = self.choice((right, next), expression);
        self.expr(left, branch)
    }

    fn binary(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let left = self.facts.body.child(expression, 0)?;
        let right = self.facts.body.child(expression, 1)?;
        let operator = self.overloaded(expression, &[left, right], next)?;
        let right = self.expr(right, operator)?;

        self.expr(left, right)
    }

    fn operand(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let inner = self.facts.body.child(expression, 0)?;
        let operator = match expression.kind {
            NodeKind::Operator => self.overloaded(expression, &[inner], next)?,
            _ => next,
        };
        self.expr(inner, operator)
    }

    fn structure(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let parts = self.facts.body.children(expression)?;
        let fields = expression.members.as_ref().map_or(parts.len(), Vec::len);
        let base = match parts.get(fields) {
            Some(base) => self.expr(base, next)?,
            None => next,
        };
        let mut entry = base;
        for field in parts[..fields.min(parts.len())].iter().rev() {
            entry = self.expr(field, entry)?;
        }

        Ok(entry)
    }

    // A local closure value used other than by direct call could run unboundedly elsewhere.
    fn path(&mut self, expression: &'a Node, next: u32) -> u32 {
        match self.facts.local(expression) {
            Some(symbol) if self.closures.contains_key(symbol) => {
                let reason = self.facts.tables.wording().text(Message::EscapingClosure);
                self.unsupported(expression, next, reason)
            }
            _ => next,
        }
    }

    /// An operator applied without a user implementation.
    pub(super) fn builtin(&self, expression: &Node) -> bool {
        self.facts.reference(expression).is_none_or(|reference| {
            reference.dispatch != Some(super::super::model::Dispatch::Overloaded)
        })
    }

    /// Overloaded operators call their implementation after the operands are evaluated.
    pub(super) fn overloaded(
        &mut self,
        expression: &'a Node,
        operands: &[&'a Node],
        next: u32,
    ) -> Lowered {
        if self.builtin(expression) {
            return Ok(next);
        }
        let Some(reference) = self.facts.reference(expression) else {
            return Ok(next);
        };

        let callee = self
            .facts
            .declared(reference)
            .map(|definition| definition.name.clone())
            .unwrap_or_default();
        let (target, operation) = self.reference_target(expression);
        let action = Action::Call {
            callee,
            target,
            arguments: operands
                .iter()
                .map(|operand| self.argument(operand))
                .collect::<Result<_, _>>()?,
            may_modify: false,
            operation,
        };
        Ok(self.node(action, Flow::Goto(next), expression))
    }

    pub(super) fn local_id(&self, expression: &Node) -> Option<u32> {
        let symbol = self.facts.local(expression)?;
        self.locals.get(symbol).copied()
    }

    pub(super) fn value(&self, expression: &'a Node) -> Value {
        if let Some(variable) = self.local_id(expression) {
            return Value::Variable(variable);
        }
        if let Ok(Some(value)) = self.extent_value(expression) {
            return value;
        }
        match (expression.kind, expression.value.as_ref()) {
            (NodeKind::Literal, Some(literal)) if literal.kind == "integer" => literal
                .value
                .as_deref()
                .and_then(|text| text.parse::<u64>().ok())
                .map_or(Value::Unknown, Value::Const),
            _ => Value::Unknown,
        }
    }

    pub(super) fn condition(&self, expression: &'a Node) -> Result<Condition, String> {
        let expression = peeled(self.facts, expression)?;
        if expression.kind != NodeKind::Operator
            || expression.operator.as_deref() != Some("lt")
            || expression.children.len() != 2
            || !self.builtin(expression)
        {
            return Ok(Condition::Unknown);
        }
        let left = self.facts.body.child(expression, 0)?;
        let right = self.facts.body.child(expression, 1)?;
        let same = left.ty.is_some() && left.ty == right.ty;

        match self.local_id(left) {
            Some(variable) if same && self.facts.integer(self.facts.ty(left)).is_some() => {
                Ok(Condition::Less {
                    variable,
                    limit: self.value(right),
                })
            }
            _ => Ok(Condition::Unknown),
        }
    }
}

/// The expression a block of only a tail expression evaluates to.
pub(super) fn peeled<'a>(
    facts: &Facts<'_, 'a>,
    mut expression: &'a Node,
) -> Result<&'a Node, String> {
    while expression.kind == NodeKind::Block
        && let [only] = expression.children.as_slice()
    {
        let inner = facts.body.node(only)?;
        if super::is_statement(inner) {
            break;
        }
        expression = inner;
    }
    Ok(expression)
}
