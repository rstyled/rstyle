//! Maps the expanded, cfg-selected AST to shared complexity constructs before HIR
//! lowering introduces branches for `for`, `?`, `.await`, and async functions.

use rstyle_core::complexity::{Kind, Node};
use rustc_ast::{
    self as ast, Expr, ExprKind,
    visit::{self, FnKind, Visitor},
};
use rustc_middle::ty::TyCtxt;
use rustc_span::{BytePos, Span};
use std::collections::BTreeMap;

pub type Functions = BTreeMap<BytePos, Vec<Node>>;

pub fn collect(tcx: TyCtxt<'_>) -> Functions {
    let mut scan = Scan {
        tcx,
        functions: BTreeMap::new(),
    };
    scan.visit_crate(&tcx.resolver_for_lowering().1.borrow());
    scan.functions
}

struct Scan<'tcx> {
    tcx: TyCtxt<'tcx>,
    functions: Functions,
}

impl<'a> Visitor<'a> for Scan<'_> {
    fn visit_fn(&mut self, kind: FnKind<'a>, _: &ast::AttrVec, span: Span, _: ast::NodeId) {
        if span.from_expansion() {
            return;
        }
        if let FnKind::Fn(_, _, function) = kind
            && let Some(body) = &function.body
        {
            let mut walker = Walker::new(self.tcx);
            walker.visit_block(body);
            self.functions.insert(function.sig.span.lo(), walker.nodes);
        }
        visit::walk_fn(self, kind);
    }
}

struct Walker<'tcx> {
    tcx: TyCtxt<'tcx>,
    nodes: Vec<Node>,
}

impl<'tcx> Walker<'tcx> {
    fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            nodes: Vec::new(),
        }
    }

    fn push(&mut self, kind: Kind, span: Span, walk: impl FnOnce(&mut Self)) {
        let mut child = Self::new(self.tcx);
        walk(&mut child);
        let location = self
            .tcx
            .sess
            .source_map()
            .lookup_char_pos(span.source_callsite().lo());
        self.nodes.push(Node {
            kind,
            line: location.line,
            column: location.col.0 + 1,
            children: child.nodes,
        });
    }

    fn branch(&mut self, expr: &Expr, kind: Kind) {
        let ExprKind::If(condition, body, otherwise) = &expr.kind else {
            return;
        };
        self.push(kind, expr.span, |child| {
            child.visit_expr(condition);
            child.visit_block(body);
        });

        let Some(otherwise) = otherwise else { return };
        if !otherwise.span.from_expansion() && matches!(otherwise.kind, ExprKind::If(..)) {
            self.branch(otherwise, Kind::ElseIf);
            return;
        }
        self.push(Kind::Else, otherwise.span, |child| {
            child.visit_expr(otherwise)
        });
    }

    fn selection(&mut self, expr: &Expr, value: &Expr, arms: &[ast::Arm]) {
        self.push(Kind::Selection, expr.span, |child| {
            child.visit_expr(value);
            for arm in arms {
                child.push(Kind::Alternative, arm.pat.span, |child| child.arm(arm));
            }
        });
    }

    fn arm(&mut self, arm: &ast::Arm) {
        if let Some(guard) = &arm.guard {
            self.push(Kind::Guard, guard.span(), |child| {
                child.visit_expr(&guard.cond)
            });
        }
        if let Some(body) = &arm.body {
            self.visit_expr(body);
        }
    }

    fn boolean_parts(&mut self, expr: &Expr) {
        if expr.span.from_expansion() {
            return;
        }
        if let ExprKind::Paren(inner) = &expr.kind {
            self.boolean_parts(inner);
            return;
        }
        if self.boolean_binary(expr) {
            return;
        }
        self.visit_expr(expr);
    }

    fn boolean_binary(&mut self, expr: &Expr) -> bool {
        if let ExprKind::Binary(operator, left, right) = &expr.kind
            && let Some(kind) = logical(operator.node)
        {
            self.boolean_parts(left);
            self.push(kind, operator.span, |_| {});
            self.boolean_parts(right);
            return true;
        }
        false
    }

    fn control(&mut self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::If(..) => self.branch(expr, Kind::Branch),
            ExprKind::Match(value, arms, _) => self.selection(expr, value, arms),
            ExprKind::Binary(operator, ..) if logical(operator.node).is_some() => {
                self.push(Kind::Boolean, expr.span, |child| child.boolean_parts(expr));
            }
            _ => return false,
        }
        true
    }
}

fn logical(operator: ast::BinOpKind) -> Option<Kind> {
    match operator {
        ast::BinOpKind::And => Some(Kind::And),
        ast::BinOpKind::Or => Some(Kind::Or),
        _ => None,
    }
}

fn construct(expr: &Expr) -> Option<Kind> {
    match &expr.kind {
        ExprKind::While(..) | ExprKind::ForLoop(..) | ExprKind::Loop(..) => Some(Kind::Loop),
        ExprKind::Closure(..) | ExprKind::Gen(..) => Some(Kind::Closure),
        ExprKind::Try(..) => Some(Kind::Propagate),
        ExprKind::Break(Some(_), _) | ExprKind::Continue(Some(_)) => Some(Kind::Jump),
        _ => None,
    }
}

fn construct_span(expr: &Expr) -> Span {
    if matches!(expr.kind, ExprKind::Try(_)) {
        return expr.span.with_lo(expr.span.hi() - BytePos(1));
    }
    expr.span
}

impl<'a> Visitor<'a> for Walker<'_> {
    // Nested named functions have their own record; closures belong to their owner.
    fn visit_item(&mut self, _: &'a ast::Item) {}

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.span.from_expansion() {
            return;
        }
        if self.control(expr) {
            return;
        }
        match construct(expr) {
            Some(kind) => self.push(kind, construct_span(expr), |child| {
                visit::walk_expr(child, expr)
            }),
            None => visit::walk_expr(self, expr),
        }
    }

    fn visit_local(&mut self, local: &'a ast::Local) {
        if local.span.from_expansion() {
            return;
        }
        let ast::LocalKind::InitElse(value, otherwise) = &local.kind else {
            visit::walk_local(self, local);
            return;
        };
        self.push(Kind::Branch, local.span, |child| {
            child.visit_expr(value);
            child.visit_block(otherwise);
        });
    }
}
