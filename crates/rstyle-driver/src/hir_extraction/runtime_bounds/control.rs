use super::values::peeled;
use super::*;
pub(super) type Handler<'a, 'tcx> = fn(&mut Collector<'a, 'tcx>, &'tcx Expr<'tcx>, u32) -> u32;
struct WhileLoop<'tcx> {
    site: LoopSite,
    condition: &'tcx Expr<'tcx>,
    body: &'tcx Expr<'tcx>,
}

impl<'a, 'tcx> Collector<'a, 'tcx> {
    pub(super) fn lower(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        Self::handler(expression)(self, expression, next)
    }

    fn handler(expression: &Expr<'_>) -> Handler<'a, 'tcx> {
        match expression.kind {
            ExprKind::Block(_, None) => Self::block_expression,
            ExprKind::DropTemps(_) => Self::operand,
            ExprKind::If(..) => Self::conditional,
            ExprKind::Loop(_, None, hir::LoopSource::While, _) => Self::while_loop,
            ExprKind::Match(_, _, hir::MatchSource::ForLoopDesugar) => Self::for_loop,
            ExprKind::Break(_, None) | ExprKind::Continue(_) => Self::jump,
            ExprKind::Ret(_) => Self::return_expression,
            ExprKind::Assign(..) | ExprKind::AssignOp(..) => Self::assignment,
            ExprKind::Call(..) => Self::call,
            ExprKind::MethodCall(..) => Self::method_call,
            ExprKind::Match(..) => Self::matching,
            _ => Self::primitive,
        }
    }

    fn block_expression(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Block(block, _) = expression.kind else {
            unreachable!()
        };
        self.block(block, next)
    }

    fn return_expression(&mut self, expression: &'tcx Expr<'tcx>, _next: u32) -> u32 {
        let ExprKind::Ret(value) = expression.kind else {
            unreachable!()
        };
        let end = self.node(Action::Nop, Flow::Return, expression.span);
        value.map_or(end, |value| self.expr(value, end))
    }

    fn conditional(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::If(condition, yes, no) = expression.kind else {
            unreachable!()
        };
        let then_node = self.expr(yes, next);
        let else_node = no.map_or(next, |value| self.expr(value, next));
        self.conditional_branch(condition, (then_node, else_node), expression.span)
    }

    fn conditional_branch(
        &mut self,
        condition: &'tcx Expr<'tcx>,
        branches: (u32, u32),
        span: rustc_span::Span,
    ) -> u32 {
        let branch = self.node(
            Action::Nop,
            Flow::Branch {
                condition: self.condition(condition),
                then_node: branches.0,
                else_node: branches.1,
            },
            span,
        );
        self.expr(condition, branch)
    }

    fn jump(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let (destination, continuing) = match expression.kind {
            ExprKind::Break(destination, _) => (destination, false),
            ExprKind::Continue(destination) => (destination, true),
            _ => unreachable!(),
        };
        match destination
            .target_id
            .ok()
            .and_then(|id| self.loops.get(&id))
        {
            Some(&(exit, repeat)) => {
                if continuing {
                    repeat
                } else {
                    exit
                }
            }
            None => self.unsupported(
                expression.span,
                next,
                "Unmodeled break/continue destination",
            ),
        }
    }

    fn call(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Call(callee, arguments) = expression.kind else {
            unreachable!()
        };
        let resolved = match callee.kind {
            ExprKind::Path(ref path) => self.ctx.typeck.qpath_res(path, callee.hir_id),
            _ => Res::Err,
        };

        let name = match resolved {
            Res::Def(_, id) => self.ctx.tcx.def_path_str(id),
            _ => "indirect call".into(),
        };

        let call = self.node(
            Action::Call {
                callee: name,
                may_modify: false,
            },
            Flow::Goto(next),
            expression.span,
        );
        let arguments = self.arguments(arguments, call);
        self.expr(callee, arguments)
    }

    pub(super) fn arguments(&mut self, arguments: &'tcx [Expr<'tcx>], next: u32) -> u32 {
        arguments
            .iter()
            .rev()
            .fold(next, |entry, arg| self.expr(arg, entry))
    }

    fn method_call(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        if self.extent_value(expression).is_some() {
            return next;
        }
        let ExprKind::MethodCall(_, receiver, arguments, _) = expression.kind else {
            unreachable!()
        };
        let callee = self
            .ctx
            .typeck
            .type_dependent_def_id(expression.hir_id)
            .map(|id| self.ctx.tcx.def_path_str(id))
            .unwrap_or_else(|| "unresolved method".into());
        let call = self.node(
            Action::Call {
                callee,
                may_modify: false,
            },
            Flow::Goto(next),
            expression.span,
        );

        let arguments = self.arguments(arguments, call);
        self.expr(receiver, arguments)
    }

    fn assignment(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        match self.assignment_action(expression) {
            Ok((action, right)) => {
                let assignment = self.node(action, Flow::Goto(next), expression.span);
                self.expr(right, assignment)
            }
            Err(reason) => self.unsupported(expression.span, next, reason),
        }
    }

    fn assignment_action(
        &self,
        expression: &'tcx Expr<'tcx>,
    ) -> Result<(Action, &'tcx Expr<'tcx>), &'static str> {
        let (left, right, add, compound) = match expression.kind {
            ExprKind::Assign(left, right, _) => (left, right, false, false),
            ExprKind::AssignOp(operator, left, right) => (
                left,
                right,
                operator.node == hir::AssignOpKind::AddAssign,
                true,
            ),
            _ => unreachable!(),
        };

        let Some(variable) = self.local_id(left) else {
            return Err("Assignment through unmodeled place or alias");
        };
        if !self.builtin(expression) {
            return Err("Overloaded assignment");
        }
        let explicit = matches!(right.kind, ExprKind::Binary(operator, lhs, rhs) if operator.node == hir::BinOpKind::Add && self.local_id(lhs) == Some(variable) && self.value(rhs) == Value::Const(1) && self.builtin(right));

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
        Ok((action, right))
    }

    fn while_loop(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let ExprKind::Loop(block, ..) = expression.kind else {
            unreachable!()
        };
        let Some(ExprKind::If(condition, body, _)) = block.expr.map(|value| &value.kind) else {
            return self.unsupported(expression.span, next, "Unrecognized HIR while desugaring");
        };
        self.lower_while(WhileLoop {
            site: LoopSite {
                owner: expression.hir_id,
                span: expression.span,
                next,
            },
            condition,
            body,
        })
    }

    fn lower_while(&mut self, loop_: WhileLoop<'tcx>) -> u32 {
        let WhileLoop {
            site,
            condition,
            body,
        } = loop_;
        let (header, condition_entry) = self.while_header(condition, site.span, site.next);
        let then_node = self.loop_body(site.owner, body, (site.next, condition_entry));

        self.finish_loop(
            header,
            self.condition(peeled(condition)),
            (then_node, site.next),
        );
        condition_entry
    }
    fn while_header(
        &mut self,
        condition: &'tcx Expr<'tcx>,
        span: rustc_span::Span,
        next: u32,
    ) -> (u32, u32) {
        let header = self.loop_header(span, next);
        let entry = self.expr(condition, header);
        (header, entry)
    }

    pub(super) fn loop_header(&mut self, span: rustc_span::Span, next: u32) -> u32 {
        let header = self.node(Action::Nop, Flow::Goto(next), span);
        self.program.loop_headers.push(header);
        header
    }

    pub(super) fn loop_body(
        &mut self,
        owner: HirId,
        body: &'tcx Expr<'tcx>,
        destinations: (u32, u32),
    ) -> u32 {
        self.loops.insert(owner, destinations);
        let entry = self.expr(body, destinations.1);
        self.loops.remove(&owner);
        entry
    }

    pub(super) fn finish_loop(&mut self, header: u32, condition: Condition, branches: (u32, u32)) {
        self.program.nodes[header as usize].flow = Flow::Branch {
            condition,
            then_node: branches.0,
            else_node: branches.1,
        };
    }
}
