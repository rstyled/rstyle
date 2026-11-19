//! Loops with no guard of their own, and loops whose contract form puts their guard in their block.

use super::super::model::{Node, NodeKind};
use super::Collector;
use super::primitive::peeled;
use crate::runtime_bounds::Condition;
use crate::runtime_bounds::contracts::{Contracts, Guard, LoopForm};

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A loop with no guard of its own, as Rust's `loop`, Go's `for {}` and C++'s `for (;;)` are:
    /// any pass may be its last, and it leaves through a break, a return or what it raises. A loop
    /// whose form puts its guard in its block runs that guard after each pass.
    pub(super) fn unguarded(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let block = self.facts.body.child(expression, 0)?;
        let kind = format!(
            "{}:{}",
            self.facts.tables.language(),
            expression.original_kind
        );
        if let Some(LoopForm {
            guard: Guard::Last, ..
        }) = Contracts::embedded().loop_form(&kind)
        {
            return self.guarded_last(expression, block, next);
        }
        let header = self.loop_header(expression, next);
        let then_node = self.loop_body(expression, block, (next, header))?;
        self.finish_loop(header, Condition::Unknown, (then_node, next));
        Ok(header)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// A loop whose block's last statement is its guard, run after each pass, to which a
    /// `continue` goes. One whose guard is false wherever it runs runs its body once, and isn't a
    /// loop.
    fn guarded_last(&mut self, expression: &'a Node, block: &'a Node, next: u32) -> Lowered {
        let statements = self.facts.body.children(block)?;
        let guarded = statements.split_last();
        let guarded = guarded.filter(|(guard, _)| guard.kind == NodeKind::ExpressionStatement);
        let Some((guard, passes)) = guarded else {
            let reason = "Loop guard is not the last statement of its block";
            return Ok(self.unsupported(expression, next, reason));
        };
        let once = self.never(self.facts.body.child(guard, 0)?)?;
        let header = match once {
            true => next,
            false => self.loop_header(expression, next),
        };
        let tested = self.statement(guard, header)?;
        self.loops.push((expression.id.as_str(), (next, tested)));
        let entry = self.sequence((passes, None), tested);
        self.loops.pop();
        let entry = entry?;
        if once {
            return Ok(entry);
        }
        self.finish_loop(header, Condition::Unknown, (entry, next));
        Ok(header)
    }

    /// Whether a guard is false wherever it runs: a false or zero literal, a conjunction with such
    /// an operand, and a sequence whose last operand is one.
    fn never(&self, guard: &'a Node) -> Result<bool, String> {
        let guard = peeled(self.facts, guard)?;
        let operands = self.facts.body.children(guard)?;
        let builtin = self.builtin(guard);
        let operator = guard.operator.as_deref().unwrap_or_default();
        // A language's own operator is namespaced, as its contracts key it.
        let sequence = Contracts::embedded().sequence(operator);
        match (guard.kind, operands.as_slice()) {
            (NodeKind::Literal, _) => Ok(guard.value.as_ref().is_some_and(|literal| {
                matches!(
                    (literal.kind.as_str(), literal.value.as_deref()),
                    ("bool", Some("false")) | ("integer", Some("0"))
                )
            })),
            (NodeKind::Operator, [left, right]) if builtin && operator == "and" => {
                Ok(self.never(left)? || self.never(right)?)
            }
            (NodeKind::Operator, [.., last]) if builtin && sequence => self.never(last),
            _ => Ok(false),
        }
    }
}
