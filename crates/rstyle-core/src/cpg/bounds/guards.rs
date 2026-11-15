//! A pure leading numeric conjunct bounds the work of every later predicate.

use super::super::model::{Node, NodeKind};
use super::Collector;
use super::extent::standard_adjustments;
use crate::runtime_bounds::{Condition, Value};

const MAX_GUARD_PARTS: usize = 64;

impl<'a> Collector<'_, '_, 'a> {
    /// `while i < limit && rest`: the numeric guard is the natural header, and
    /// evaluating `rest`, even on its final false result, is part of its scan.
    pub(super) fn conjunctive_loop(
        &mut self,
        owner: &'a Node,
        next: u32,
    ) -> Result<Option<u32>, String> {
        let condition = self.facts.body.child(owner, 0)?;
        let Some((condition, rest)) = self.leading_conjunction(condition)? else {
            return Ok(None);
        };
        let Some((header, body)) = self.begin_conjunction(owner, next, rest.len())? else {
            return Ok(Some(next));
        };
        let entry = self.conjunction_predicates(rest, body, next)?;
        self.finish_loop(header, condition, (entry, next));
        Ok(Some(header))
    }

    fn begin_conjunction(
        &mut self,
        owner: &'a Node,
        next: u32,
        predicates: usize,
    ) -> Result<Option<(u32, u32)>, String> {
        if !self.charge(predicates + 1) {
            return Ok(None);
        }
        let body = self.facts.body.child(owner, 1)?;
        let header = self.loop_header(owner, next);
        let entry = self.loop_body(owner, body, (next, header))?;
        Ok(Some((header, entry)))
    }

    fn conjunction_predicates(
        &mut self,
        predicates: Vec<&'a Node>,
        mut entry: u32,
        next: u32,
    ) -> Result<u32, String> {
        // The outermost right conjunct was saved first: backward lowering puts
        // its work after all the inner conjuncts, in source evaluation order.
        for predicate in predicates {
            let branch = self.choice((entry, next), predicate);
            entry = self.expr(predicate, branch)?;
        }
        Ok(entry)
    }

    fn leading_conjunction(
        &self,
        expression: &'a Node,
    ) -> Result<Option<(Condition, Vec<&'a Node>)>, String> {
        let mut current = expression;
        let mut rest = Vec::new();
        for _ in 0..MAX_GUARD_PARTS {
            let Some(expression) = self.guard_expression(current)? else {
                return Ok(None);
            };
            let conjunction = expression.kind == NodeKind::Operator
                && expression.operator.as_deref() == Some("and")
                && expression.children.len() == 2
                && self.builtin(expression);
            if !conjunction {
                return if rest.is_empty() {
                    Ok(None)
                } else {
                    Ok(self
                        .pure_less(expression)?
                        .map(|condition| (condition, rest)))
                };
            }
            rest.push(self.facts.body.child(expression, 1)?);
            current = self.facts.body.child(expression, 0)?;
        }
        Ok(None)
    }

    /// Peeling a one-expression block may not erase an implicit operation.
    fn guard_expression(&self, mut node: &'a Node) -> Result<Option<&'a Node>, String> {
        for _ in 0..MAX_GUARD_PARTS {
            if self.facts.adjusted_of(node) || self.hidden_call(node)?.is_some() {
                return Ok(None);
            }
            if node.kind != NodeKind::Block || node.children.len() != 1 {
                return Ok(Some(node));
            }
            node = self.facts.body.child(node, 0)?;
        }
        Ok(None)
    }

    fn pure_less(&self, expression: &'a Node) -> Result<Option<Condition>, String> {
        let condition = self.condition(expression)?;
        if !matches!(&condition, Condition::Less { limit, .. } if *limit != Value::Unknown) {
            return Ok(None);
        }
        for child in self.facts.body.children(expression)? {
            if !self.pure_guard_value(child)? {
                return Ok(None);
            }
        }
        Ok(Some(condition))
    }

    fn pure_guard_value(&self, expression: &'a Node) -> Result<bool, String> {
        if self.facts.adjusted_of(expression) || self.hidden_call(expression)?.is_some() {
            return Ok(false);
        }
        let scalar = matches!(expression.kind, NodeKind::Identifier | NodeKind::Literal)
            && expression.children.is_empty()
            && self.facts.integer(self.facts.ty(expression)).is_some()
            && self.value(expression) != Value::Unknown;
        if scalar {
            return Ok(true);
        }
        Ok(self.extent_value(expression)?.is_some()
            && self.pure_extent_place(self.facts.body.child(expression, 0)?)?)
    }

    fn pure_extent_place(&self, mut expression: &'a Node) -> Result<bool, String> {
        for _ in 0..MAX_GUARD_PARTS {
            if !standard_adjustments(self.facts, expression)
                || self.hidden_call(expression)?.is_some()
            {
                return Ok(false);
            }
            if expression.kind == NodeKind::Identifier {
                return Ok(expression.children.is_empty());
            }
            let through = matches!(expression.kind, NodeKind::FieldAccess | NodeKind::Reference)
                || (expression.kind == NodeKind::Operator
                    && expression.operator.as_deref() == Some("deref")
                    && self.builtin(expression));
            if !through || expression.children.len() != 1 {
                return Ok(false);
            }
            expression = self.facts.body.child(expression, 0)?;
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisory::runtime_bounds::{Input, MAX_NODES};
    use crate::cpg::bounds::facts::Facts;
    use crate::cpg::model::{Document, Unit};
    use crate::cpg::source::Sources;
    use crate::cpg::tree::{Body, Tables};
    use crate::runtime_bounds::{Action, Arithmetic, Flow};

    fn guarded_unit() -> Unit {
        let document: Document =
            serde_json::from_str(include_str!("../testdata/example.cpg.json")).unwrap();
        let mut unit = document.unit.unwrap();
        let function = unit
            .functions
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|function| function.name == "count")
            .unwrap();
        for node in &mut function.nodes {
            match node.id.as_str() {
                "count.if" => node.kind = NodeKind::While,
                "count.cond" => node.children.reverse(),
                _ => {}
            }
        }
        unit
    }

    #[test]
    fn exhaustion_during_guard_setup_cannot_deliver_a_partial_program() {
        let unit = guarded_unit();
        let function = unit
            .functions
            .as_ref()
            .unwrap()
            .iter()
            .find(|function| function.name == "count")
            .unwrap();
        let body = Body::new(function).unwrap();
        let tables = Tables::new(&unit).unwrap();
        let sources = Sources::new(&unit).unwrap();
        let facts = Facts {
            body: &body,
            tables: &tables,
            sources: &sources,
            pointer_bits: 64,
            destructors_are_calls: true,
        };
        let mut collector = Collector::new(&facts, Arithmetic::Checked);
        let (statements, _) = collector.statements(body.root().unwrap()).unwrap();
        collector.register_locals(&statements).unwrap();
        let owner = body.node("count.if").unwrap();
        let next = collector.node(Action::Nop, Flow::Return, owner);
        collector.work = MAX_NODES * 4;

        assert_eq!(collector.conjunctive_loop(owner, next).unwrap(), Some(next));
        assert!(collector.exhausted);
        assert!(collector.program.loop_headers.is_empty());
        assert!(matches!(
            collector.collect().unwrap(),
            Input::BudgetExhausted { .. }
        ));
    }
}
