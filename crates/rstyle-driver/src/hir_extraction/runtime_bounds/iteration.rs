use super::*;

use super::super::for_loops::{Parts, parts};

struct ForLoop<'tcx> {
    site: LoopSite,
    parts: Parts<'tcx>,
}

struct Scan {
    index: u32,
    limit_variable: u32,
    start: Value,
    limit: Value,
}
impl<'tcx> Collector<'_, 'tcx> {
    pub(super) fn for_loop(&mut self, expression: &'tcx Expr<'tcx>, next: u32) -> u32 {
        let Some(parts) = parts(self.ctx, expression) else {
            return self.unsupported(
                expression.span,
                next,
                "Unrecognized resolved for-loop desugaring",
            );
        };
        self.resolved_for(ForLoop {
            site: LoopSite {
                owner: expression.hir_id,
                span: expression.span,
                next,
            },
            parts,
        })
    }

    fn resolved_for(&mut self, loop_: ForLoop<'tcx>) -> u32 {
        match self.scan_values(&loop_.parts) {
            Ok(values) => self.prepare_scan(loop_, values),
            Err(reason) => self.unsupported(loop_.site.span, loop_.site.next, reason),
        }
    }

    fn scan_values(&mut self, parts: &Parts<'tcx>) -> Result<(Value, Value, u8), &'static str> {
        if !matches!(
            parts.pattern.kind,
            PatKind::Binding(
                hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
                _,
                _,
                None
            ) | PatKind::Wild
        ) {
            return Err("Mutable or destructuring iterator binding is unmodeled");
        }
        self.iteration_values(parts.head)
            .ok_or("Only resolved primitive ranges and shared slice iteration have a size model")
    }

    fn prepare_scan(&mut self, loop_: ForLoop<'tcx>, values: (Value, Value, u8)) -> u32 {
        let (start, limit, bits) = values;
        let index = self.iteration_binding(loop_.site.owner, loop_.parts.pattern, bits);
        let limit_variable = self.variable(
            format!("for-limit-{}", loop_.site.owner.local_id.as_u32()),
            Value::Unknown,
            bits,
        );
        let scan = Scan {
            index,
            limit_variable,
            start,
            limit,
        };

        self.scan(loop_, scan)
    }

    fn iteration_binding(&mut self, owner: HirId, pattern: &hir::Pat<'_>, bits: u8) -> u32 {
        let index = self.variable(
            format!("for-index-{}", owner.local_id.as_u32()),
            Value::Unknown,
            bits,
        );
        match pattern.kind {
            PatKind::Binding(
                hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
                id,
                _,
                None,
            ) => {
                if self.bits(self.ctx.typeck.pat_ty(pattern)) == Some(bits) {
                    self.locals.insert(id, index);
                }
            }
            PatKind::Wild => {}
            _ => unreachable!(),
        }
        index
    }

    fn scan(&mut self, loop_: ForLoop<'tcx>, scan: Scan) -> u32 {
        let header = self.loop_header(loop_.site.span, loop_.site.next);
        self.scan_cycle(&loop_, &scan, header);
        self.initialize_scan(scan, header, loop_.site.span)
    }

    fn scan_cycle(&mut self, loop_: &ForLoop<'tcx>, scan: &Scan, header: u32) {
        let next = loop_.site.next;
        let increment = self.node(
            Action::Increment {
                variable: scan.index,
            },
            Flow::Goto(header),
            loop_.site.span,
        );
        let body = self.loop_body(
            loop_.parts.iteration.hir_id,
            loop_.parts.body,
            (next, increment),
        );
        self.finish_loop(
            header,
            Condition::Less {
                variable: scan.index,
                limit: Value::Variable(scan.limit_variable),
            },
            (body, next),
        );
    }

    fn initialize_scan(&mut self, scan: Scan, header: u32, span: rustc_span::Span) -> u32 {
        let limit_init = self.node(
            Action::Assign {
                variable: scan.limit_variable,
                value: scan.limit,
            },
            Flow::Goto(header),
            span,
        );
        self.node(
            Action::Assign {
                variable: scan.index,
                value: scan.start,
            },
            Flow::Goto(limit_init),
            span,
        )
    }

    fn iteration_values(&mut self, head: &'tcx Expr<'tcx>) -> Option<(Value, Value, u8)> {
        if matches!(head.kind, ExprKind::Struct(..)) {
            self.range_values(head)
        } else {
            self.slice_values(head)
        }
    }

    fn range_values(&self, head: &Expr<'tcx>) -> Option<(Value, Value, u8)> {
        let ExprKind::Struct(_, fields, hir::StructTailExpr::None) = head.kind else {
            return None;
        };

        let [start, end] = fields else {
            return None;
        };
        let ty::Adt(definition, arguments) = self.ctx.typeck.expr_ty(head).kind() else {
            return None;
        };
        if Some(definition.did()) != self.ctx.tcx.lang_items().range_struct()
            || self.ctx.tcx.crate_name(definition.did().krate).as_str() != "core"
            || start.ident.name.as_str() != "start"
            || end.ident.name.as_str() != "end"
        {
            return None;
        }

        let bits = self.bits(arguments.type_at(0))?;
        if self.ctx.typeck.expr_ty(start.expr) != arguments.type_at(0)
            || self.ctx.typeck.expr_ty(end.expr) != arguments.type_at(0)
        {
            return None;
        }

        let start = self.value(start.expr);
        let limit = self.value(end.expr);
        if matches!(start, Value::Unknown) || matches!(limit, Value::Unknown) {
            return None;
        }
        Some((start, limit, bits))
    }

    fn slice_values(&mut self, head: &'tcx Expr<'tcx>) -> Option<(Value, Value, u8)> {
        let receiver = super::extent::iteration_receiver(self.ctx, head)?;
        let place = super::extent::place(self.ctx, receiver)?;
        let label = self.extents.get(&place.key())?.clone();
        Some((
            Value::Const(0),
            Value::Input(label),
            self.ctx.tcx.data_layout.pointer_size().bits() as u8,
        ))
    }
}
