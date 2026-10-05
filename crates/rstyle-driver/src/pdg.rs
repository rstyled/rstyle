//! HIR dependence graph of one function body (see `rstyle_core::facts::Pdg`).
//!
//! Operand expressions give data edges, `Res::Local` links a binding to its uses, and the
//! enclosing branch/match/loop gives control edges. `for` loops, `?` and macros are single
//! nodes (recovered from expansion data), closures are opaque except the body of
//! `Iterator::for_each`, which is treated like a `for` body so both forms align.
//! No policy lives here: labels and thresholds are applied by `rstyle-core`.

use crate::extract::line_of;
use crate::{ids, sig};
use rstyle_core::clip::clip;
use rstyle_core::facts::{EdgeKind, NodeKind, Pdg, PdgEdge, PdgNode, TyShape};
use rustc_ast::LitKind;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{
    Arm, Body, Expr, ExprKind, HirId, LoopSource, MatchSource, Pat, PatKind, QPath, StmtKind,
};
use rustc_middle::ty::{self, TyCtxt, TypeckResults};
use rustc_span::{ExpnKind, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Method names that only change how a collection is viewed, not what is iterated.
const TRANSPARENT: [&str; 3] = ["iter", "iter_mut", "into_iter"];

/// Direct sub-expressions of an expression (nested bodies excluded).
#[derive(Default)]
struct Kids<'tcx> {
    found: Vec<&'tcx Expr<'tcx>>,
}

impl<'tcx> Visitor<'tcx> for Kids<'tcx> {
    // rstyle-allow(side_effect_density): visitor accumulates results in self by design
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.found.push(expr);
    }
}

fn children<'tcx>(expr: &'tcx Expr<'tcx>) -> Vec<&'tcx Expr<'tcx>> {
    let mut kids = Kids::default();
    intravisit::walk_expr(&mut kids, expr);
    kids.found
}

/// Expressions the user wrote inside a macro call (their spans are not from an expansion).
#[derive(Default)]
struct UserArgs<'tcx> {
    found: Vec<&'tcx Expr<'tcx>>,
}

impl<'tcx> Visitor<'tcx> for UserArgs<'tcx> {
    // rstyle-allow(side_effect_density): visitor accumulates results in self by design
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            intravisit::walk_expr(self, expr);
        } else {
            self.found.push(expr);
        }
    }
}

fn macro_name(expr: &Expr<'_>) -> Option<String> {
    if !expr.span.from_expansion() {
        return None;
    }
    match expr.span.ctxt().outer_expn_data().kind {
        ExpnKind::Macro(_, name) => Some(name.to_string()),
        _ => None,
    }
}

/// Strips `DropTemps` and single-expression blocks.
fn peeled<'tcx>(mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    loop {
        expr = match expr.kind {
            ExprKind::DropTemps(inner) => inner,
            ExprKind::Block(block, _) if block.stmts.is_empty() && block.expr.is_some() => {
                block.expr.unwrap_or(expr)
            }
            _ => return expr,
        };
    }
}

/// The expression a loop body block evaluates to: its tail, or its last statement.
fn last_expr<'tcx>(block: &'tcx rustc_hir::Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    block.expr.or_else(|| match block.stmts.last()?.kind {
        StmtKind::Expr(e) | StmtKind::Semi(e) => Some(e),
        _ => None,
    })
}

/// `(head, pattern, body)` of a desugared `for` loop.
fn for_parts<'tcx>(
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Pat<'tcx>, &'tcx Expr<'tcx>)> {
    let ExprKind::Match(scrutinee, [outer], MatchSource::ForLoopDesugar) = peeled(expr).kind else {
        return None;
    };
    let ExprKind::Call(_, [head]) = scrutinee.kind else {
        return None;
    };
    let ExprKind::Loop(block, ..) = peeled(outer.body).kind else {
        return None;
    };
    let ExprKind::Match(_, arms, _) = peeled(last_expr(block)?).kind else {
        return None;
    };
    let some: &Arm<'_> = arms.get(1)?;
    let pat = match some.pat.kind {
        PatKind::TupleStruct(_, [pat], _) => pat,
        PatKind::Struct(_, [field], _) => field.pat,
        _ => return None,
    };
    Some((head, pat, some.body))
}

/// Operand of a desugared `?`.
fn try_operand<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) = expr.kind else {
        return None;
    };
    match scrutinee.kind {
        ExprKind::Call(_, [operand]) => Some(operand),
        _ => None,
    }
}

fn lit_kind(lit: &LitKind) -> &'static str {
    match lit {
        LitKind::Str(..) | LitKind::ByteStr(..) | LitKind::CStr(..) => "str",
        LitKind::Int(..) | LitKind::Byte(_) => "int",
        LitKind::Float(..) => "float",
        LitKind::Bool(_) => "bool",
        LitKind::Char(_) => "char",
        _ => "other",
    }
}

/// The literal's value (never source text, which inside macros spans the whole call).
fn literal_detail(lit: &LitKind, span: Span) -> String {
    if span.from_expansion() {
        return String::new();
    }
    match lit {
        LitKind::Str(sym, _) => clip(&format!("{:?}", sym.as_str()), 24),
        LitKind::Int(n, _) => n.get().to_string(),
        LitKind::Bool(b) => b.to_string(),
        LitKind::Char(c) => format!("{c:?}"),
        _ => String::new(),
    }
}

pub struct Builder<'tcx> {
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'tcx>,
    root: PathBuf,
    nodes: Vec<PdgNode>,
    edges: Vec<PdgEdge>,
    binds: HashMap<HirId, usize>,
    /// Innermost enclosing control construct and the arm/branch index being lowered.
    ctrl: Vec<(usize, usize)>,
}

/// Builds the graph of a function body.
pub fn build<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    body: &'tcx Body<'tcx>,
    root: &Path,
) -> Pdg {
    let mut builder = Builder {
        tcx,
        typeck: tcx.typeck(def_id),
        root: root.to_path_buf(),
        nodes: vec![],
        edges: vec![],
        binds: HashMap::new(),
        ctrl: vec![],
    };
    builder.lower_body(body);
    builder.finish()
}

fn blank(kind: NodeKind) -> PdgNode {
    PdgNode {
        kind,
        ty: None,
        callee_id: None,
        callee_sig: None,
        macro_name: None,
        lit_kind: None,
        detail: String::new(),
        line: 0,
    }
}

impl<'tcx> Builder<'tcx> {
    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn lower_body(&mut self, body: &'tcx Body<'tcx>) {
        self.params(body);
        let value = self.expr(body.value);
        // A unit-valued body returns nothing: `f();` and a tail `f()` must give the same graph.
        if !self.typeck.expr_ty(body.value).is_unit() {
            let line = self.line(body.value.span);
            let ret = self.add(PdgNode {
                line,
                ..blank(NodeKind::Return)
            });
            self.link(value, ret, 0);
        }
    }

    fn finish(self) -> Pdg {
        let mut edges = self.edges;
        edges.sort_by_key(|e| (e.from, e.to, e.kind, e.arg_pos));
        edges.dedup();
        Pdg {
            nodes: self.nodes,
            edges,
        }
    }

    fn line(&self, span: Span) -> usize {
        line_of(self.tcx, span)
    }

    fn snippet(&self, span: Span) -> String {
        let text = self
            .tcx
            .sess
            .source_map()
            .span_to_snippet(span)
            .unwrap_or_default();
        clip(&text, 32)
    }

    /// Source text of a non-literal path, empty inside macro expansions (spans lie there).
    fn path_detail(&self, span: Span) -> String {
        if span.from_expansion() {
            String::new()
        } else {
            self.snippet(span)
        }
    }

    /// Adds a node; inside a control construct it also gets the control edge.
    fn add(&mut self, node: PdgNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        if let Some(&(owner, position)) = self.ctrl.last() {
            self.edges.push(PdgEdge {
                from: owner,
                to: index,
                kind: EdgeKind::Ctrl,
                arg_pos: position,
            });
        }
        index
    }

    fn link(&mut self, from: Option<usize>, to: usize, arg_pos: usize) {
        if let Some(from) = from {
            self.edges.push(PdgEdge {
                from,
                to,
                kind: EdgeKind::Data,
                arg_pos,
            });
        }
    }

    fn shape_of(&self, expr: &Expr<'tcx>) -> Option<TyShape> {
        Some(sig::shape(self.tcx, self.typeck.expr_ty(expr)))
    }

    fn typed(&self, kind: NodeKind, expr: &Expr<'tcx>) -> PdgNode {
        PdgNode {
            ty: self.shape_of(expr),
            line: self.line(expr.span),
            ..blank(kind)
        }
    }

    fn bind(&mut self, pat: &Pat<'tcx>, node: usize) {
        let mut ids = Vec::new();
        pat.each_binding(|_, id, _, _| ids.push(id));
        self.binds.extend(ids.into_iter().map(|id| (id, node)));
    }

    fn params(&mut self, body: &'tcx Body<'tcx>) {
        for param in body.params {
            let mut node = blank(NodeKind::Param);
            node.ty = Some(sig::shape(self.tcx, self.typeck.pat_ty(param.pat)));
            node.line = self.line(param.pat.span);
            let index = self.add(node);
            self.bind(param.pat, index);
        }
    }

    /// Lowers `kids` as operands of `to`, numbering positions from `start`.
    fn operands(&mut self, to: usize, kids: &[&'tcx Expr<'tcx>], start: usize) {
        for (offset, kid) in kids.iter().enumerate() {
            let value = self.expr(kid);
            self.link(value, to, start + offset);
        }
    }

    fn under<R>(&mut self, owner: usize, position: usize, f: impl FnOnce(&mut Self) -> R) -> R {
        self.ctrl.push((owner, position));
        let result = f(self);
        self.ctrl.pop();
        result
    }
}

impl<'tcx> Builder<'tcx> {
    /// Lowers an expression; the result is the node that produces its value, if any.
    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn expr(&mut self, expr: &'tcx Expr<'tcx>) -> Option<usize> {
        if let Some((head, pat, body)) = for_parts(expr) {
            return Some(self.iterate(expr, head, (pat, body)));
        }
        if let Some(operand) = try_operand(expr) {
            let node = self.add(self.typed(NodeKind::Try, expr));
            let value = self.expr(operand);
            self.link(value, node, 0);
            return Some(node);
        }
        if let Some(name) = macro_name(expr) {
            return Some(self.macro_call(expr, name));
        }
        self.expr_kind(expr)
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn expr_kind(&mut self, expr: &'tcx Expr<'tcx>) -> Option<usize> {
        match expr.kind {
            ExprKind::Path(ref qpath) => self.path(expr, qpath),
            ExprKind::Lit(lit) => Some(self.literal(expr, &lit.node)),
            ExprKind::Call(callee, args) => self.call(expr, callee, args),
            ExprKind::MethodCall(_, receiver, args, _) => self.method(expr, receiver, args),
            ExprKind::Field(base, ident) => {
                let node = self.add(PdgNode {
                    detail: ident.name.to_string(),
                    ..self.typed(NodeKind::Field, expr)
                });
                let value = self.expr(base);
                self.link(value, node, 0);
                Some(node)
            }
            ExprKind::AddrOf(_, _, inner) | ExprKind::DropTemps(inner) => self.expr(inner),
            ExprKind::Unary(rustc_hir::UnOp::Deref, inner) => self.expr(inner),
            ExprKind::Block(block, _) => self.block(block),
            ExprKind::If(cond, then, otherwise) => Some(self.branch(expr, cond, (then, otherwise))),
            ExprKind::Match(scrutinee, arms, _) => Some(self.matcher(expr, scrutinee, arms)),
            ExprKind::Loop(block, _, source, _) => Some(self.looping(expr, block, source)),
            ExprKind::Closure(_) => Some(self.add(self.typed(NodeKind::Closure, expr))),
            ExprKind::Ret(value) => self.returned(expr, value),
            ExprKind::Let(let_expr) => Some(self.let_expr(expr, let_expr.pat, let_expr.init)),
            ExprKind::Break(_, value) => value.and_then(|v| self.expr(v)),
            ExprKind::Continue(_) => None,
            _ => Some(self.generic(expr)),
        }
    }

    fn path(&mut self, expr: &Expr<'tcx>, qpath: &QPath<'tcx>) -> Option<usize> {
        let res = self.typeck.qpath_res(qpath, expr.hir_id);
        match res {
            Res::Local(id) => self.binds.get(&id).copied(),
            Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) => {
                let node = PdgNode {
                    lit_kind: Some("fn".into()),
                    callee_id: Some(ids::def_id_str(self.tcx, def_id)),
                    callee_sig: sig::callee_sig(self.tcx, def_id),
                    ..self.typed(NodeKind::Lit, expr)
                };
                Some(self.add(node))
            }
            _ => {
                let node = PdgNode {
                    lit_kind: Some("path".into()),
                    detail: self.path_detail(expr.span),
                    ..self.typed(NodeKind::Lit, expr)
                };
                Some(self.add(node))
            }
        }
    }

    fn literal(&mut self, expr: &Expr<'tcx>, lit: &LitKind) -> usize {
        let node = PdgNode {
            lit_kind: Some(lit_kind(lit).into()),
            detail: literal_detail(lit, expr.span),
            ..self.typed(NodeKind::Lit, expr)
        };
        self.add(node)
    }

    fn call_node(&self, expr: &Expr<'tcx>, def_id: Option<DefId>) -> PdgNode {
        PdgNode {
            callee_id: def_id.map(|id| ids::def_id_str(self.tcx, id)),
            callee_sig: def_id.and_then(|id| sig::callee_sig(self.tcx, id)),
            ..self.typed(NodeKind::Call, expr)
        }
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn call(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        callee: &'tcx Expr<'tcx>,
        args: &'tcx [Expr<'tcx>],
    ) -> Option<usize> {
        let target = match self.typeck.expr_ty(callee).kind() {
            ty::FnDef(def_id, _) => Some(*def_id),
            _ => None,
        };
        let is_ctor = target.is_some_and(|id| matches!(self.tcx.def_kind(id), DefKind::Ctor(..)));
        let node = if is_ctor {
            PdgNode {
                detail: "ctor".into(),
                ..self.typed(NodeKind::Op, expr)
            }
        } else {
            self.call_node(expr, target)
        };
        let index = self.add(node);
        if target.is_none() {
            let value = self.expr(callee);
            self.link(value, index, 0);
        }
        let kids: Vec<&Expr<'tcx>> = args.iter().collect();
        self.operands(index, &kids, 0);
        Some(index)
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn method(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        receiver: &'tcx Expr<'tcx>,
        args: &'tcx [Expr<'tcx>],
    ) -> Option<usize> {
        let def_id = self.typeck.type_dependent_def_id(expr.hir_id);
        let name = def_id
            .map(|id| self.tcx.item_name(id).to_string())
            .unwrap_or_default();
        let outside = def_id.is_some_and(|id| !ids::in_workspace(self.tcx, id, &self.root));
        if args.is_empty() && outside && TRANSPARENT.contains(&name.as_str()) {
            return self.expr(receiver);
        }
        if let (Some(id), [closure]) = (def_id, args)
            && self.is_for_each(id)
            && let ExprKind::Closure(c) = closure.kind
        {
            return Some(self.for_each(expr, receiver, c.body));
        }
        let index = self.add(self.call_node(expr, def_id));
        let kids: Vec<&Expr<'tcx>> = std::iter::once(receiver).chain(args).collect();
        self.operands(index, &kids, 0);
        Some(index)
    }

    fn is_for_each(&self, def_id: DefId) -> bool {
        self.tcx
            .def_path_str(def_id)
            .ends_with("Iterator::for_each")
    }

    /// `head.for_each(|pat| body)`: the same abstract iteration as `for pat in head { body }`.
    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn for_each(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        head: &'tcx Expr<'tcx>,
        closure: rustc_hir::BodyId,
    ) -> usize {
        let body = self.tcx.hir_body(closure);
        let item = body.params.first().map(|p| p.pat);
        let node = PdgNode {
            ty: item.map(|p| sig::shape(self.tcx, self.typeck.pat_ty(p))),
            ..self.typed(NodeKind::Iterate, expr)
        };
        let index = self.add(node);
        let value = self.expr(head);
        self.link(value, index, 0);
        if let Some(pat) = item {
            self.bind(pat, index);
        }
        self.under(index, 0, |b| b.expr(body.value));
        index
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn iterate(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        head: &'tcx Expr<'tcx>,
        (pat, body): (&'tcx Pat<'tcx>, &'tcx Expr<'tcx>),
    ) -> usize {
        let node = PdgNode {
            ty: Some(sig::shape(self.tcx, self.typeck.pat_ty(pat))),
            ..self.typed(NodeKind::Iterate, expr)
        };
        let index = self.add(node);
        let value = self.expr(head);
        self.link(value, index, 0);
        self.bind(pat, index);
        self.under(index, 0, |b| b.expr(body));
        index
    }

    fn macro_call(&mut self, expr: &'tcx Expr<'tcx>, name: String) -> usize {
        let index = self.add(PdgNode {
            macro_name: Some(name),
            ..self.typed(NodeKind::Macro, expr)
        });
        let mut args = UserArgs::default();
        intravisit::walk_expr(&mut args, expr);
        self.operands(index, &args.found, 0);
        index
    }

    fn block(&mut self, block: &'tcx rustc_hir::Block<'tcx>) -> Option<usize> {
        for stmt in block.stmts {
            match stmt.kind {
                StmtKind::Let(local) => self.local(local),
                StmtKind::Expr(e) | StmtKind::Semi(e) => {
                    self.expr(e);
                }
                StmtKind::Item(_) => {}
            }
        }
        block.expr.and_then(|tail| self.expr(tail))
    }

    /// `let x = e` is transparent (`x` is `e`'s node); destructuring gets a `Let` node.
    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn local(&mut self, local: &'tcx rustc_hir::LetStmt<'tcx>) {
        let init = local.init.and_then(|e| self.expr(e));
        let simple = matches!(local.pat.kind, PatKind::Binding(_, _, _, None));
        match init {
            Some(node) if simple => self.bind(local.pat, node),
            _ => {
                let node = self.add(PdgNode {
                    ty: Some(sig::shape(self.tcx, self.typeck.pat_ty(local.pat))),
                    line: self.line(local.span),
                    ..blank(NodeKind::Let)
                });
                self.link(init, node, 0);
                self.bind(local.pat, node);
            }
        }
        if let Some(els) = local.els {
            self.block(els);
        }
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn let_expr(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        pat: &'tcx Pat<'tcx>,
        init: &'tcx Expr<'tcx>,
    ) -> usize {
        let node = self.add(self.typed(NodeKind::Let, expr));
        let value = self.expr(init);
        self.link(value, node, 0);
        self.bind(pat, node);
        node
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn branch(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        cond: &'tcx Expr<'tcx>,
        (then, otherwise): (&'tcx Expr<'tcx>, Option<&'tcx Expr<'tcx>>),
    ) -> usize {
        let node = self.add(self.typed(NodeKind::Branch, expr));
        let value = self.expr(cond);
        self.link(value, node, 0);
        self.under(node, 0, |b| b.expr(then));
        if let Some(other) = otherwise {
            self.under(node, 1, |b| b.expr(other));
        }
        node
    }

    // rstyle-allow(mutation_per_target): the builder accumulates the graph in self by design
    fn matcher(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        scrutinee: &'tcx Expr<'tcx>,
        arms: &'tcx [Arm<'tcx>],
    ) -> usize {
        let node = self.add(self.typed(NodeKind::Match, expr));
        let value = self.expr(scrutinee);
        self.link(value, node, 0);
        for (position, arm) in arms.iter().enumerate() {
            self.bind(arm.pat, node);
            self.under(node, position, |b| {
                if let Some(guard) = arm.guard {
                    b.expr(guard);
                }
                b.expr(arm.body)
            });
        }
        node
    }

    fn looping(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        block: &'tcx rustc_hir::Block<'tcx>,
        source: LoopSource,
    ) -> usize {
        let node = self.add(PdgNode {
            detail: format!("{source:?}").to_lowercase(),
            ..self.typed(NodeKind::Loop, expr)
        });
        self.under(node, 0, |b| b.block(block));
        node
    }

    fn returned(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        value: Option<&'tcx Expr<'tcx>>,
    ) -> Option<usize> {
        let node = self.add(self.typed(NodeKind::Return, expr));
        let value = value.and_then(|v| self.expr(v));
        self.link(value, node, 0);
        None
    }

    /// Operators, struct/tuple/array construction, indexing, assignment, casts.
    fn generic(&mut self, expr: &'tcx Expr<'tcx>) -> usize {
        let (kind, detail) = match expr.kind {
            ExprKind::Binary(op, ..) => (NodeKind::Op, op.node.as_str().to_string()),
            ExprKind::Unary(op, _) => (NodeKind::Op, op.as_str().to_string()),
            ExprKind::AssignOp(op, ..) => (NodeKind::Op, format!("{}=", op.node.as_str())),
            ExprKind::Assign(..) => (NodeKind::Op, "=".into()),
            ExprKind::Index(..) => (NodeKind::Op, "index".into()),
            ExprKind::Struct(..) => (NodeKind::Op, "struct".into()),
            ExprKind::Tup(_) => (NodeKind::Op, "tuple".into()),
            ExprKind::Array(_) | ExprKind::Repeat(..) => (NodeKind::Op, "array".into()),
            ExprKind::Cast(..) => (NodeKind::Cast, String::new()),
            _ => (NodeKind::Op, "expr".into()),
        };
        let index = self.add(PdgNode {
            detail,
            ..self.typed(kind, expr)
        });
        self.operands(index, &children(expr), 0);
        index
    }
}

/// Dependence graph for one function, for `FnFacts::pdg`.
pub fn pdg_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: LocalDefId,
    body: &'tcx Body<'tcx>,
    root: &Path,
) -> Pdg {
    build(tcx, def_id, body, root)
}
