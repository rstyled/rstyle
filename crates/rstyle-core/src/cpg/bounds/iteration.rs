//! `for` loops over primitive unsigned ranges, shared sequences with a known extent, and sources a
//! loop counts: ranges a library call makes, tuples and fixed-length sequences, and integers.

use super::super::model::{Node, NodeKind, Origin, Type, TypeKind};
use super::Collector;
use super::extent::iteration_receiver;
use super::facts::Facts;
use crate::runtime_bounds::contracts::Contracts;
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

/// How a `for` counts its passes, and what its source evaluates once before the first.
pub(super) struct Counted<'a> {
    start: Value,
    limit: Value,
    integer: Integer,
    evaluated: Vec<&'a Node>,
}

impl Counted<'_> {
    /// A count from `start` up to `limit` whose source evaluates nothing.
    fn of((start, limit, bits): (Value, Value, u8)) -> Self {
        Self {
            start,
            limit,
            integer: Integer::unsigned(bits),
            evaluated: Vec::new(),
        }
    }
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
        if let Some(entry) = self.for_iterator_chain(expression, &parts, next)? {
            return Ok(entry);
        }
        match self.scan_values(&parts)? {
            Ok(mut counted) => {
                let evaluated = std::mem::take(&mut counted.evaluated);
                let entry = self.prepare_scan((expression, parts), counted, next)?;
                self.arguments(&evaluated, entry)
            }
            Err(reason) => Ok(self.unsupported(expression, next, reason)),
        }
    }

    fn scan_values(
        &mut self,
        parts: &Parts<'a>,
    ) -> Result<Result<Counted<'a>, &'static str>, String> {
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
        counted: Counted<'a>,
        next: u32,
    ) -> Lowered {
        let owner = self.node_number(expression);
        let integer = counted.integer;
        let index = self.variable(format!("for-index-{owner}"), Value::Unknown, integer);
        // A binding that can be reassigned is not the index: a write to it is no reset.
        if let Some(symbol) = self.by_value_binding(parts.pattern)
            && !self.facts.body.symbol(symbol)?.mutable
            && self.facts.integer(self.facts.ty(parts.pattern)) == Some(integer)
        {
            self.locals.insert(symbol, index);
        }
        let limit_variable = self.variable(format!("for-limit-{owner}"), Value::Unknown, integer);

        let scan = Scan {
            index,
            limit_variable,
            start: counted.start,
            limit: counted.limit,
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

    /// How a `for` counts its source, through calls that traverse an argument of theirs: those
    /// calls are evaluated as written, and the loop counts what the argument counts.
    fn iteration_values(&mut self, head: &'a Node) -> Result<Option<Counted<'a>>, String> {
        let source = traversed(self.facts, head)?;
        let counted = self.source_values(source)?;
        Ok(counted.map(|counted| match std::ptr::eq(source, head) {
            true => counted,
            false => Counted {
                evaluated: vec![head],
                ..counted
            },
        }))
    }

    fn source_values(&mut self, head: &'a Node) -> Result<Option<Counted<'a>>, String> {
        if head.kind == NodeKind::RecordConstruction {
            return Ok(self.range_values(head)?.map(Counted::of));
        }
        if let Some(counted) = self.counted(head)? {
            return Ok(Some(counted));
        }
        Ok(self.slice_values(head)?.map(Counted::of))
    }

    /// A source a `for` counts by what the document says of it: a call a range role names, which
    /// counts from its start up to its stop by a step of at least one, evaluating its arguments;
    /// a tuple, or a sequence of fixed length, by its length; and an integer, from zero up to it.
    /// Each of the last three evaluates the source itself.
    fn counted(&self, head: &'a Node) -> Result<Option<Counted<'a>>, String> {
        if head.kind == NodeKind::Call {
            return self.range_call(head);
        }
        let Some(ty) = self.facts.ty(head) else {
            return Ok(None);
        };
        let (limit, integer) = match (ty.kind, self.facts.integer(Some(ty))) {
            (_, Some(integer)) => (self.value(head), integer),
            (TypeKind::Tuple, None) => (
                Value::Const(ty.items.as_ref().map_or(0, Vec::len) as u64),
                Integer::unsigned(self.facts.pointer_bits),
            ),
            (_, None) => match fixed_length(ty) {
                Some(length) => (
                    Value::Const(length),
                    Integer::unsigned(self.facts.pointer_bits),
                ),
                None => return Ok(None),
            },
        };
        Ok(Some(Counted {
            start: Value::Const(0),
            limit,
            integer,
            evaluated: vec![head],
        }))
    }

    /// A call a range role names, by the form of its arity. A start or a step the lowering can't
    /// read, or a step below one, leaves the count unknown.
    fn range_call(&self, call: &'a Node) -> Result<Option<Counted<'a>>, String> {
        let arguments = self.facts.body.children(call)?.split_off(1);
        let operation = self
            .facts
            .reference(call)
            .and_then(|reference| self.facts.declared(reference))
            .and_then(|definition| definition.operation.as_deref());
        let forms = operation.and_then(|operation| Contracts::embedded().range(operation));
        let form = forms.and_then(|forms| {
            forms
                .iter()
                .find(|form| form.arity as usize == arguments.len())
        });
        let Some(form) = form else {
            return Ok(None);
        };
        let stop = arguments[form.stop as usize];
        let Some(integer) = self.facts.integer(self.facts.ty(stop)) else {
            return Ok(None);
        };
        let start = form.start.map_or(Value::Const(0), |start| {
            self.value(arguments[start as usize])
        });
        let counts_up = form.step.is_none_or(
            |step| matches!(self.value(arguments[step as usize]), Value::Const(step) if step >= 1),
        );
        Ok(Some(Counted {
            start,
            limit: match counts_up {
                true => self.value(stop),
                false => Value::Unknown,
            },
            integer,
            evaluated: arguments,
        }))
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
        let Some(limit) = self.sequence_extent(receiver)? else {
            return Ok(None);
        };
        Ok(Some((Value::Const(0), limit, self.facts.pointer_bits)))
    }
}

/// What a `for` over `head` iterates once calls a traversal role names, which each yield at most
/// one item per item of the argument they traverse, are looked through.
pub(super) fn traversed<'a>(facts: &Facts<'_, 'a>, head: &'a Node) -> Result<&'a Node, String> {
    let mut source = head;
    while source.kind == NodeKind::Call {
        let parts = facts.body.children(source)?;
        let role = facts
            .reference(source)
            .and_then(|reference| facts.declared(reference))
            .and_then(|definition| definition.operation.as_deref())
            .and_then(|operation| Contracts::embedded().traversal(operation));
        let arity = parts.len() as u32 - 1;
        let Some(role) = role.filter(|role| role.arities.contains(&arity)) else {
            break;
        };
        source = parts[role.over as usize + 1];
    }
    Ok(source)
}

/// The length of a sequence type that has one fixed length, as an array's.
fn fixed_length(ty: &Type) -> Option<u64> {
    (ty.kind == TypeKind::Sequence && ty.fixed == Some(true))
        .then_some(ty.length)
        .flatten()
}

/// The iterable an unlabeled `for` loop iterates, as written: the source a compiler-generated
/// `for` wrapper holds alone, or the loop's own iterable. Any other call is the iterable itself,
/// never the last of its arguments.
pub(super) fn for_head<'a>(
    facts: &Facts<'_, 'a>,
    expression: &'a Node,
) -> Result<Option<&'a Node>, String> {
    if expression.kind != NodeKind::For || expression.label.is_some() {
        return Ok(None);
    }
    let wrapper = facts.body.child(expression, 1)?;
    let generated_for = wrapper.synthetic
        && wrapper.provenance.origin == Origin::Desugaring
        && wrapper.provenance.construct.as_deref() == Some("for");
    if !generated_for {
        return Ok(Some(wrapper));
    }
    let mut sources = facts
        .sources(wrapper)?
        .into_iter()
        .filter(|child| !child.synthetic);
    let head = match (sources.next(), sources.next()) {
        (Some(head), None) => head,
        _ => wrapper,
    };

    Ok(Some(head))
}
