//! Control constructs, calls and assignments.

use super::super::model::{Dispatch, Node, NodeKind};
use super::Collector;
use crate::runtime_bounds::{Action, Callee, Condition, Flow, Value};

type Lowered = Result<u32, String>;

impl<'a> Collector<'_, '_, 'a> {
    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    pub(super) fn lower(&mut self, expression: &'a Node, next: u32) -> Lowered {
        if let Some(reason) = self.hidden_call(expression)? {
            return Ok(self.unsupported(expression, next, reason));
        }
        match expression.kind {
            NodeKind::Block if self.plain_block(expression) => self.block(expression, next),
            NodeKind::If => self.conditional(expression, next),
            NodeKind::While if expression.label.is_none() => self.while_loop(expression, next),
            NodeKind::For => self.for_loop(expression, next),
            NodeKind::Break if expression.children.is_empty() => self.jump(expression, next),
            NodeKind::Continue => self.jump(expression, next),
            NodeKind::Return => self.return_expression(expression),
            NodeKind::Assignment if self.facts.body.destructures(expression)? => {
                self.destructuring(expression, next)
            }
            NodeKind::Assignment | NodeKind::CompoundAssignment => {
                self.assignment(expression, next)
            }
            NodeKind::Call if self.method_form(expression) => self.method_call(expression, next),
            NodeKind::Call => self.call(expression, next),
            NodeKind::Match => self.matching(expression, next),
            NodeKind::Propagate => self.matching(expression, next),
            _ => self.primitive(expression, next),
        }
    }

    /// An unlabeled block that is not a `const` block.
    fn plain_block(&self, block: &Node) -> bool {
        block.label.is_none() && !block.modifiers.iter().any(|m| m == "const")
    }

    fn method_form(&self, call: &Node) -> bool {
        let dispatch = self.facts.reference(call).and_then(|r| r.dispatch);
        matches!(dispatch, Some(Dispatch::Method | Dispatch::Dynamic))
            || self
                .facts
                .reference(call)
                .is_some_and(|r| r.dispatch.is_none() && r.reason.is_some())
    }

    fn return_expression(&mut self, expression: &'a Node) -> Lowered {
        // Inlined callback returns resume at the invocation point, not the owner's exit.
        let end = match self.returns.last() {
            Some(&resume) => resume,
            None => self.node(Action::Nop, Flow::Return, expression),
        };
        // What a frontend adds after the value, such as destructors, runs before the return.
        let mut entry = end;
        for child in expression.children.iter().rev() {
            entry = self.expr(self.facts.body.node(child)?, entry)?;
        }

        Ok(entry)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn conditional(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let condition = self.facts.body.child(expression, 0)?;
        let then_node = self.expr(self.facts.body.child(expression, 1)?, next)?;
        let else_node = match expression.children.get(2) {
            Some(otherwise) => self.expr(self.facts.body.node(otherwise)?, next)?,
            None => next,
        };
        let branch = self.node(
            Action::Nop,
            Flow::Branch {
                condition: self.condition(condition)?,
                then_node,
                else_node,
            },
            expression,
        );

        self.expr(condition, branch)
    }

    fn jump(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let continuing = expression.kind == NodeKind::Continue;
        match self.loops.last().filter(|_| expression.label.is_none()) {
            Some(&(_, (exit, repeat))) => Ok(if continuing { repeat } else { exit }),
            None => Ok(self.unsupported(expression, next, "Unmodeled break/continue destination")),
        }
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn call(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let callee = self.facts.body.child(expression, 0)?;
        let arguments: Vec<&'a Node> = self.facts.body.children(expression)?[1..].to_vec();
        if let Some(symbol) = self.facts.local(callee)
            && let Some(&closure) = self.closures.get(symbol)
        {
            // Arguments are evaluated before the inlined body runs.
            let body = self.inline(closure, next)?;
            return self.arguments(&arguments, body);
        }
        // Only a path names what it calls; any other callee is a computed value.
        let path = (callee.kind == NodeKind::Identifier)
            .then(|| self.facts.resolved(callee))
            .flatten();
        let name = match path {
            Some((_, definition)) => definition.name.clone(),
            None => "indirect call".into(),
        };
        let (target, operation) = self.call_target(expression, callee);
        let action = Action::Call {
            callee: name,
            target,
            arguments: arguments
                .iter()
                .map(|argument| self.argument(argument))
                .collect::<Result<_, _>>()?,
            may_modify: false,
            operation,
        };
        let call = self.node(action, Flow::Goto(next), expression);
        let entry = self.arguments(&arguments, call)?;

        self.expr(callee, entry)
    }

    /// Lowers `items` in evaluation order, built backward from `next`.
    pub(super) fn arguments(&mut self, items: &[&'a Node], next: u32) -> Lowered {
        let mut entry = next;
        for item in items.iter().rev() {
            entry = self.expr(item, entry)?;
        }
        Ok(entry)
    }

    fn method_call(&mut self, expression: &'a Node, next: u32) -> Lowered {
        if self.extent_value(expression)?.is_some() {
            return Ok(next);
        }
        if let Some(entry) = self.iterator_chain(expression, next)? {
            return Ok(entry);
        }
        if self.unresolved_callback(expression)? {
            let reason = "Iterator callback or chain has no resolved standard model";
            return Ok(self.unsupported(expression, next, reason));
        }
        self.method_invocation(expression, next)
    }

    fn method_invocation(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let inputs = self.facts.body.children(expression)?;
        let callee = self
            .facts
            .reference(expression)
            .and_then(|reference| self.facts.declared(reference))
            .map_or_else(
                || "unresolved method".into(),
                |definition| definition.name.clone(),
            );
        let (target, operation) = self.reference_target(expression);
        let action = Action::Call {
            callee,
            target,
            arguments: inputs
                .iter()
                .map(|input| self.argument(input))
                .collect::<Result<_, _>>()?,
            may_modify: false,
            operation,
        };
        let call = self.node(action, Flow::Goto(next), expression);
        let entry = self.arguments(&inputs[1..], call)?;

        self.expr(inputs[0], entry)
    }

    fn assignment(&mut self, expression: &'a Node, next: u32) -> Lowered {
        match self.assignment_action(expression)? {
            Ok((action, right)) => {
                let assignment = self.node(action, Flow::Goto(next), expression);
                self.expr(right, assignment)
            }
            Err(reason) => Ok(self.unsupported(expression, next, reason)),
        }
    }

    fn assignment_action(
        &self,
        expression: &'a Node,
    ) -> Result<Result<(Action, &'a Node), &'static str>, String> {
        let left = self.facts.body.child(expression, 0)?;
        let right = self.facts.body.child(expression, 1)?;
        let compound = expression.kind == NodeKind::CompoundAssignment;
        let add = compound && expression.operator.as_deref() == Some("add");
        let Some(variable) = self.local_id(left) else {
            return Ok(Err("Assignment through unmodeled place or alias"));
        };
        if !self.builtin(expression) {
            return Ok(Err("Overloaded assignment"));
        }
        let explicit = right.kind == NodeKind::Operator
            && right.operator.as_deref() == Some("add")
            && right.children.len() == 2
            && self.local_id(self.facts.body.child(right, 0)?) == Some(variable)
            && self.value(self.facts.body.child(right, 1)?) == Value::Const(1)
            && self.builtin(right);
        let action = if (add && self.value(right) == Value::Const(1)) || (!compound && explicit) {
            Action::Increment { variable }
        } else {
            Action::Assign {
                variable,
                value: if compound {
                    Value::Unknown
                } else {
                    self.value(right)
                },
            }
        };

        Ok(Ok((action, right)))
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    /// `target = value` as the block that lowers it: a let of the value, then one
    /// assignment of an unknown value per place, built backward.
    fn destructuring(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let target = self.facts.body.child(expression, 0)?;
        let value = self.facts.body.child(expression, 1)?;
        let places = self.facts.body.places(target)?;
        if !self.charge(1 + places.len()) {
            return Ok(next);
        }
        let mut entry = next;
        for place in places.iter().rev() {
            if !self.enter_expression() {
                continue;
            }
            entry = match self.local_id(place) {
                Some(variable) => {
                    let action = Action::Assign {
                        variable,
                        value: Value::Unknown,
                    };
                    let assignment = self.node(action, Flow::Goto(entry), place);
                    // The binding the lowering reads is a path of its own.
                    if self.enter_expression() {
                        self.depth -= 1;
                    }
                    assignment
                }
                None => {
                    self.unsupported(place, entry, "Assignment through unmodeled place or alias")
                }
            };
            self.depth -= 1;
        }

        self.expr(value, entry)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn while_loop(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let condition = self.facts.body.child(expression, 0)?;
        let body = self.facts.body.child(expression, 1)?;
        let header = self.loop_header(expression, next);
        let condition_entry = self.expr(condition, header)?;
        let then_node = self.loop_body(expression, body, (next, condition_entry))?;
        let condition = self.condition(super::primitive::peeled(self.facts, condition)?)?;
        self.finish_loop(header, condition, (then_node, next));

        Ok(condition_entry)
    }

    pub(super) fn loop_header(&mut self, at: &Node, next: u32) -> u32 {
        let header = self.node(Action::Nop, Flow::Goto(next), at);
        self.program.loop_headers.push(header);
        header
    }

    pub(super) fn loop_body(
        &mut self,
        owner: &'a Node,
        body: &'a Node,
        destinations: (u32, u32),
    ) -> Lowered {
        self.loops.push((owner.id.as_str(), destinations));
        let entry = self.expr(body, destinations.1);
        self.loops.pop();
        entry
    }

    pub(super) fn finish_loop(&mut self, header: u32, condition: Condition, branches: (u32, u32)) {
        if let Some(node) = self.program.nodes.get_mut(header as usize) {
            node.flow = Flow::Branch {
                condition,
                then_node: branches.0,
                else_node: branches.1,
            };
        }
    }

    /// A match, or `operand?` as the match its lowering is: arms over `branch(operand)`.
    fn matching(&mut self, expression: &'a Node, next: u32) -> Lowered {
        let children = self.facts.body.children(expression)?;
        let (scrutinee, arms) = children
            .split_first()
            .ok_or_else(|| format!("Node {} has no scrutinee", expression.id))?;
        self.arms(expression, (scrutinee, arms), next)
    }

    fn arms(
        &mut self,
        expression: &'a Node,
        (scrutinee, arms): (&'a Node, &[&'a Node]),
        next: u32,
    ) -> Lowered {
        for arm in arms {
            if self.guarded_or(arm)? {
                let reason = "Guarded or-pattern reevaluation is unmodeled";
                return Ok(self.unsupported(expression, next, reason));
            }
        }
        let mut fallback = next;
        for arm in arms.iter().rev() {
            fallback = self.arm(arm, (next, fallback))?;
        }
        self.expr(scrutinee, fallback)
    }

    // rstyle-allow(mutation_per_target): the collector accumulates the program in self by design
    fn arm(&mut self, arm: &'a Node, destinations: (u32, u32)) -> Lowered {
        let parts = self.facts.body.children(arm)?;
        let body = *parts
            .last()
            .ok_or_else(|| format!("Arm {} has no body", arm.id))?;
        let guard = parts
            .iter()
            .find(|part| part.kind == NodeKind::MatchGuard)
            .copied();
        let body = self.expr(body, destinations.0)?;
        let selected = match guard {
            Some(guard) => {
                let branch = self.choice((body, destinations.1), arm);
                self.expr(self.facts.body.child(guard, 0)?, branch)?
            }
            None => body,
        };

        Ok(self.choice((selected, destinations.1), arm))
    }

    fn guarded_or(&self, arm: &'a Node) -> Result<bool, String> {
        let parts = self.facts.body.children(arm)?;
        if !parts.iter().any(|part| part.kind == NodeKind::MatchGuard) {
            return Ok(false);
        }
        let mut pending = vec![self.facts.body.child(arm, 0)?];
        while let Some(pattern) = pending.pop() {
            if pattern.kind == NodeKind::PatternOr {
                return Ok(true);
            }
            pending.extend(self.facts.body.children(pattern)?);
        }

        Ok(false)
    }

    pub(super) fn choice(&mut self, branches: (u32, u32), at: &Node) -> u32 {
        self.node(
            Action::Nop,
            Flow::Branch {
                condition: Condition::Unknown,
                then_node: branches.0,
                else_node: branches.1,
            },
            at,
        )
    }

    /// The callee of a call in function form, as the program names its target.
    fn call_target(&self, call: &'a Node, callee: &'a Node) -> (Callee, Option<String>) {
        let path = (callee.kind == NodeKind::Identifier)
            .then(|| self.facts.resolved(callee))
            .flatten();
        match path {
            Some((super::super::model::TargetKind::Constructor, _)) => (Callee::Constructor, None),
            Some((_, definition))
                if matches!(
                    definition.kind,
                    super::super::model::DefinitionKind::Function
                        | super::super::model::DefinitionKind::Method
                ) && self.function_item(callee) == Some(definition.id.as_str()) =>
            {
                self.reference_target(call)
            }
            _ => (Callee::Unresolved, None),
        }
    }

    fn function_item(&self, callee: &Node) -> Option<&'a str> {
        let ty = self.facts.ty(callee)?;
        (ty.kind == super::super::model::TypeKind::Function)
            .then_some(ty.definition.as_deref())
            .flatten()
    }
}
