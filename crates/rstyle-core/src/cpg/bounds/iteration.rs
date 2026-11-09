//! `for` loops over primitive unsigned ranges and stable shared slices.

use super::super::model::{Node, NodeKind};
use super::Collector;
use super::extent::iteration_receiver;
use super::facts::Facts;
use crate::runtime_bounds::{Action, Condition, Flow, Integer, Value};
use crate::wording::Message;

type Lowered = Result<u32, String>;

/// A `for` loop's source parts.
pub(super) struct Parts<'a> {
    pub head: &'a Node,
    pub pattern: &'a Node,
    pub body: &'a Node,
}

struct Scan {
    index: u32,
    limit_variable: u32,
    start: Value,
    limit: Value,
}

impl<'a> Collector<'_, '_, 'a> {
    /// The pattern, iterable and body of an unlabeled `for`.
    pub(super) fn parts(&self, expression: &'a Node) -> Result<Option<Parts<'a>>, String> {
        let Some(head) = for_head(self.facts, expression)? else {
            return Ok(None);
        };
        Ok(Some(Parts {
            head,
            pattern: self.facts.body.child(expression, 0)?,
            body: self.facts.body.child(expression, 2)?,
        }))
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    pub(super) fn for_loop(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let Some(parts) = self.parts(expression)? else {
            let reason = "Unrecognized resolved for-loop desugaring";
            return Ok(self.unsupported(expression, next, reason));
        };
        match self.scan_values(&parts)? {
            Ok(values) => self.prepare_scan((expression, parts), values, next),
            Err(reason) => Ok(self.unsupported(expression, next, reason)),
        }
    }

    fn scan_values(
        &mut self,
        parts: &Parts<'a>,
    ) -> Result<Result<(Value, Value, u8), &'static str>, String> {
        // A pass binds its item, so a pattern of new bindings cannot change how often the
        // loop runs. One holding anything else might write a place a bound relies on.
        if !self.written_places(parts.pattern)?.is_empty() {
            return Ok(Err(self.facts.tables.wording().text(Message::LoopTarget)));
        }
        let unsized_iteration = self.facts.tables.wording().text(Message::UnsizedIteration);
        Ok(self.iteration_values(parts.head)?.ok_or(unsized_iteration))
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// The loop's index and limit variables; the pattern's binding reads the index.
    fn prepare_scan(
        &mut self,
        (expression, parts): (&'a Node, Parts<'a>),
        (start, limit, bits): (Value, Value, u8),
        next: u32,
    ) -> Lowered {
        let owner = self.node_number(expression);
        let index = self.variable(
            format!("for-index-{owner}"),
            Value::Unknown,
            Integer::unsigned(bits),
        );
        // A binding that can be reassigned is not the index: a write to it is no reset.
        if let Some(symbol) = self.by_value_binding(parts.pattern)
            && !self.facts.body.symbol(symbol)?.mutable
            && self.facts.unsigned_bits(self.facts.ty(parts.pattern)) == Some(bits)
        {
            self.locals.insert(symbol, index);
        }
        let limit_variable = self.variable(
            format!("for-limit-{owner}"),
            Value::Unknown,
            Integer::unsigned(bits),
        );

        let scan = Scan {
            index,
            limit_variable,
            start,
            limit,
        };
        self.scan_loop((expression, parts.body), scan, next)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// `index = start; limit = end; while index < limit { body; index += 1 }`.
    fn scan_loop(
        &mut self,
        (expression, body): (&'a Node, &'a Node),
        scan: Scan,
        next: u32,
    ) -> Lowered {
        let header = self.loop_header(expression, next);
        let increment = Action::Increment {
            variable: scan.index,
        };
        let increment = self.node(increment, Flow::Goto(header), expression);
        let body = self.loop_body(expression, body, (next, increment))?;
        let condition = Condition::Less {
            variable: scan.index,
            limit: Value::Variable(scan.limit_variable),
        };
        self.finish_loop(header, condition, (body, next));

        let limit = Action::Assign {
            variable: scan.limit_variable,
            value: scan.limit,
        };
        let limit_init = self.node(limit, Flow::Goto(header), expression);
        let start = Action::Assign {
            variable: scan.index,
            value: scan.start,
        };
        Ok(self.node(start, Flow::Goto(limit_init), expression))
    }

    /// A stable number for a node, standing in for the frontend's own.
    pub(super) fn node_number(&self, node: &Node) -> u32 {
        let nodes = &self.facts.body.function.nodes;
        let offset = self.facts.body.function.symbols.len() as u32;
        nodes
            .iter()
            .position(|candidate| candidate.id == node.id)
            .map_or(u32::MAX, |position| offset + position as u32)
    }

    fn iteration_values(&mut self, head: &'a Node) -> Result<Option<(Value, Value, u8)>, String> {
        if head.kind == NodeKind::RecordConstruction {
            return self.range_values(head);
        }
        self.slice_values(head)
    }

    fn range_values(&self, head: &'a Node) -> Result<Option<(Value, Value, u8)>, String> {
        let Some((start, end, bits)) = self.range_parts(head)? else {
            return Ok(None);
        };

        let start = self.value(start);
        let limit = self.value(end);
        if matches!(start, Value::Unknown) || matches!(limit, Value::Unknown) {
            return Ok(None);
        }
        Ok(Some((start, limit, bits)))
    }

    /// Operands of a primitive unsigned standard `Range` literal.
    pub(super) fn range_parts(
        &self,
        head: &'a Node,
    ) -> Result<Option<(&'a Node, &'a Node, u8)>, String> {
        if head.kind != NodeKind::RecordConstruction {
            return Ok(None);
        }

        let parts = self.facts.body.children(head)?;
        let members = head.members.as_deref().unwrap_or_default();
        let ([start, end], [first, second]) = (parts.as_slice(), members) else {
            return Ok(None);
        };
        let Some(ty) = self.facts.ty(head) else {
            return Ok(None);
        };
        if self.facts.type_key(ty) != Some("Range")
            || !self.facts.standard_definition(ty.definition.as_deref())
            || first != "start"
            || second != "end"
        {
            return Ok(None);
        }
        let Some(argument) = ty.arguments.as_ref().and_then(|a| a.first()) else {
            return Ok(None);
        };
        let Some(bits) = self.facts.unsigned_bits(self.facts.type_of(argument)) else {
            return Ok(None);
        };
        if start.ty.as_ref() != Some(argument) || end.ty.as_ref() != Some(argument) {
            return Ok(None);
        }
        Ok(Some((start, end, bits)))
    }

    fn slice_values(&mut self, head: &'a Node) -> Result<Option<(Value, Value, u8)>, String> {
        let Some(receiver) = iteration_receiver(self.facts, head)? else {
            return Ok(None);
        };
        let Some(place) = self.place(receiver)? else {
            return Ok(None);
        };
        let Some(label) = self.extents.get(&place.key()).cloned() else {
            return Ok(None);
        };
        Ok(Some((
            Value::Const(0),
            Value::Input(label),
            self.facts.pointer_bits,
        )))
    }
}

/// The iterable an unlabeled `for` loop iterates, as written.
pub(super) fn for_head<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
) -> Result<Option<&'a Node>, String> {
    if expression.kind != NodeKind::For || expression.label.is_some() {
        return Ok(None);
    }
    let wrapper = facts.body.child(expression, 1)?;
    let head = facts
        .sources(wrapper)?
        .into_iter()
        .rev()
        .find(|child| !child.synthetic)
        .unwrap_or(wrapper);

    Ok(Some(head))
}
