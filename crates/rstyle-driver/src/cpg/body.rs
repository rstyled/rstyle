//! One HIR body as `compat.cpg` nodes, symbols and references.
//!
//! Source constructs keep their source shape: a lowered `for`, `while` or `?` is
//! one node whose source children come first, followed by the lowering's own
//! nodes as synthetic children, so every expression rustc walks has a node. A
//! destructuring assignment is one `assignment` whose target is rebuilt from the
//! lowering's pattern; the lowering's temporaries are left out.

use super::compiler;
use super::unit::Unit;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{
    self as hir, Arm, BinOpKind, Block, BlockCheckMode, Expr, ExprKind, HirId, LoopSource,
    MatchSource, Pat, PatKind, QPath, Stmt, StmtKind, UnOp, UnsafeSource,
};
use rustc_middle::hir::place::ProjectionKind;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt, TypeckResults};
use rustc_span::Span;
use rustc_span::hygiene::{DesugaringKind, ExpnKind};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub struct Builder<'u, 'tcx> {
    pub unit: &'u mut Unit<'tcx>,
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'tcx>,
    owner: LocalDefId,
    prefix: String,
    params: HashSet<HirId>,
    claimed: HashSet<HirId>,
    lowering: Option<&'static str>,
    pub emitted: HashSet<HirId>,
    bound: HashSet<HirId>,
    pub nodes: Vec<Value>,
    pub symbols: Vec<Value>,
    pub refs: Vec<Value>,
}

/// The parts of a node before it is recorded.
struct Shape {
    kind: &'static str,
    children: Vec<String>,
    fields: serde_json::Map<String, Value>,
}

fn shape(kind: &'static str, children: Vec<String>) -> Shape {
    Shape {
        kind,
        children,
        fields: serde_json::Map::new(),
    }
}

impl<'u, 'tcx> Builder<'u, 'tcx> {
    pub fn new(unit: &'u mut Unit<'tcx>, owner: LocalDefId, params: HashSet<HirId>) -> Self {
        let tcx = unit.tcx;
        Self {
            unit,
            tcx,
            typeck: tcx.typeck(owner),
            owner,
            prefix: format!("f{}", owner.local_def_index.as_u32()),
            params,
            claimed: HashSet::new(),
            lowering: None,
            emitted: HashSet::new(),
            bound: HashSet::new(),
            nodes: vec![],
            symbols: vec![],
            refs: vec![],
        }
    }

    pub fn id(&self, hir_id: HirId) -> String {
        format!("{}.{}", self.prefix, hir_id.local_id.as_u32())
    }

    pub fn symbol_id(&self, hir_id: HirId) -> String {
        format!("s{}.{}", self.prefix, hir_id.local_id.as_u32())
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// Records a node and returns its ID.
    fn node(&mut self, hir_id: HirId, span: Span, shape: Shape) -> String {
        let id = self.id(hir_id);
        let provenance = self.provenance(span);
        let own = self
            .unit
            .span(span)
            .or_else(|| self.unit.span(span.source_callsite()));
        // HIR gives a parenthesized expression the parentheses' span; a tuple's own syntax has
        // them. The span is the expression's own; `parentheses` counts the pairs around it.
        let (own, parentheses) = match (shape.kind, own) {
            ("tuple" | "pattern_tuple", own) | (_, own @ None) => (own, 0),
            (_, Some(own)) => {
                let (own, levels) = self.unit.without_parentheses(own);
                (Some(own), levels)
            }
        };
        let original = super::compiler::original_kind(self.tcx, hir_id);
        let mut record = json!({"id": id, "kind": shape.kind, "original_kind": original,
                                "children": shape.children, "provenance": provenance});
        record.as_object_mut().unwrap().extend(shape.fields);
        match own {
            Some(own) => record["span"] = own,
            None => record["synthetic"] = json!(true),
        }
        if parentheses > 0 {
            record["parentheses"] = json!(parentheses);
        }
        if self.lowering.is_some() {
            record["synthetic"] = json!(true);
        }
        if let Some(ty) = self.typeck.node_type_opt(hir_id) {
            record["type"] = json!(self.unit.ty(ty));
        }
        self.compiler_facts(hir_id, &mut record);
        self.emitted.insert(hir_id);
        self.nodes.push(record);

        id
    }

    /// What rustc decided about the node that the source does not show: implicit
    /// adjustments, and for a binding pattern how it binds.
    fn compiler_facts(&mut self, hir_id: HirId, record: &mut Value) {
        let typeck = self.typeck;
        if let Some(adjustments) = typeck.adjustments().get(hir_id).filter(|a| !a.is_empty()) {
            record["adjustments"] = compiler::adjustments(self.unit, adjustments);
        }
        if let Some(adjustments) = typeck
            .pat_adjustments()
            .get(hir_id)
            .filter(|a| !a.is_empty())
        {
            record["adjustments"] = compiler::pattern_adjustments(self.unit, adjustments);
        }
        if let Some(mode) = typeck.pat_binding_modes().get(hir_id) {
            record["binding_mode"] = json!(compiler::binding_mode(*mode));
        }
    }

    /// A restored source construct: source provenance at the construct's call site.
    fn construct(&mut self, hir_id: HirId, span: Span, shape: Shape) -> String {
        let id = self.node(hir_id, span.source_callsite(), shape);
        let record = self.nodes.last_mut().unwrap();
        record["provenance"] = json!({"origin": "source"});
        id
    }

    fn provenance(&mut self, span: Span) -> Value {
        let call_site = self.unit.span(span.source_callsite());
        // A lowering's own node is synthetic; its provenance is where its span comes from.
        if !span.from_expansion() {
            return json!({"origin": "source"});
        }
        if let Some(construct) = self.lowering {
            return json!({"origin": "desugaring", "construct": construct, "call_site": call_site});
        }

        let expansion = span.ctxt().outer_expn_data();
        let definition = expansion
            .macro_def_id
            .map(|def_id| self.unit.definition(def_id));
        match expansion.kind {
            ExpnKind::Desugaring(kind) => {
                json!({"origin": "desugaring", "construct": construct_name(kind), "call_site": call_site})
            }
            ExpnKind::Macro(_, name) => {
                let mut provenance = json!({"origin": "macro_expansion", "call_site": call_site,
                                            "macro": name.to_string()});
                if let Some(definition) = definition {
                    provenance["macro_definition"] = json!(definition);
                }
                provenance
            }
            _ => json!({"origin": "macro_expansion", "call_site": call_site}),
        }
    }

    fn lowered<T>(&mut self, construct: &'static str, build: impl FnOnce(&mut Self) -> T) -> T {
        let outer = self.lowering.replace(construct);
        let result = build(self);
        self.lowering = outer;
        result
    }

    /// Node IDs of the items `emit` records, skipping those it leaves out.
    fn each<T>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        emit: fn(&mut Self, T) -> Option<String>,
    ) -> Vec<String> {
        items
            .into_iter()
            .filter_map(|item| emit(self, item))
            .collect()
    }

    fn exprs(&mut self, exprs: impl IntoIterator<Item = &'tcx Expr<'tcx>>) -> Vec<String> {
        self.each(exprs, Self::expr)
    }

    // Expressions.

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    pub fn expr(&mut self, expr: &'tcx Expr<'tcx>) -> Option<String> {
        if self.lowering.is_some() && self.claimed.contains(&expr.hir_id) {
            return None;
        }
        match expr.kind {
            ExprKind::DropTemps(inner) => return self.expr(inner),
            // A constant block's value is a body of its own, walked in place.
            // A constant block's value is a body of its own, walked in place and
            // marked `const`.
            ExprKind::ConstBlock(constant) => {
                let block = self.expr(self.tcx.hir_body(constant.body).value)?;
                let whole = self.unit.span(expr.span);
                // The construct is the whole `const { .. }`, identified as the frontend's
                // facts identify it, not only its block.
                let id = self.id(expr.hir_id);
                let original = super::compiler::original_kind(self.tcx, expr.hir_id);
                if let Some(node) = self.nodes.iter_mut().rev().find(|node| node["id"] == block) {
                    node["modifiers"] = json!(["const"]);
                    node["id"] = json!(id);
                    node["original_kind"] = json!(original);
                    if let Some(whole) = whole {
                        node["span"] = whole;
                    }
                }
                return Some(id);
            }
            _ => {}
        }
        if self.lowering.is_none()
            && let Some(id) = self.restored(expr)
        {
            return Some(id);
        }
        let shape = self.expr_shape(expr);
        let id = self.node(expr.hir_id, expr.span, shape);
        self.reference(expr, &id);

        Some(id)
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    fn restored(&mut self, expr: &'tcx Expr<'tcx>) -> Option<String> {
        match expr.kind {
            ExprKind::Match(scrutinee, [outer], MatchSource::ForLoopDesugar) => {
                self.for_loop(expr, scrutinee, outer)
            }
            ExprKind::Match(scrutinee, arms, MatchSource::TryDesugar(_)) => {
                self.propagate(expr, scrutinee, arms)
            }
            ExprKind::Loop(block, label, LoopSource::While, _) => {
                self.while_loop(expr, block, label)
            }
            ExprKind::Block(block, None) if crate::extract::destructuring_assignment(block) => {
                Some(self.destructuring(expr, block))
            }
            _ => None,
        }
    }

    fn expr_shape(&mut self, expr: &'tcx Expr<'tcx>) -> Shape {
        if let Some(shape) = self.operation_shape(expr) {
            return shape;
        }
        if let Some(shape) = self.control_shape(expr) {
            return shape;
        }
        self.value_shape(expr)
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// Calls, operators, assignments and accesses.
    fn operation_shape(&mut self, expr: &'tcx Expr<'tcx>) -> Option<Shape> {
        Some(match expr.kind {
            ExprKind::Call(callee, args) => {
                shape("call", self.exprs(std::iter::once(callee).chain(args)))
            }
            ExprKind::MethodCall(_, receiver, args, _) => {
                shape("call", self.exprs(std::iter::once(receiver).chain(args)))
            }
            ExprKind::Binary(op, left, right) => {
                self.operator(binary(op.node), op.span, vec![left, right])
            }
            ExprKind::Unary(op, inner) => {
                self.operator(unary(op), expr.span.shrink_to_lo(), vec![inner])
            }
            ExprKind::AssignOp(op, left, right) => {
                let mut shape = self.operator(assign_op(op.node), op.span, vec![left, right]);
                shape.kind = "compound_assignment";
                shape
            }
            ExprKind::Assign(left, right, _) => shape("assignment", self.exprs([left, right])),
            ExprKind::Field(base, member) => {
                let mut shape = shape("field_access", self.exprs([base]));
                shape
                    .fields
                    .insert("member".into(), json!(member.name.as_str()));
                let position = self.typeck.field_index(expr.hir_id).as_u32();
                shape.fields.insert("member_index".into(), json!(position));
                shape
            }
            ExprKind::Index(base, index, _) => shape("index_access", self.exprs([base, index])),
            ExprKind::AddrOf(kind, mutability, inner) => {
                let mut shape = shape("reference", self.exprs([inner]));
                let modifiers: Vec<&str> = [
                    (mutability.is_mut(), "mutable"),
                    (kind == hir::BorrowKind::Raw, "raw"),
                ]
                .into_iter()
                .filter_map(|(present, modifier)| present.then_some(modifier))
                .collect();
                if !modifiers.is_empty() {
                    shape.fields.insert("modifiers".into(), json!(modifiers));
                }
                shape
            }
            ExprKind::Cast(inner, _) | ExprKind::Type(inner, _) => {
                shape("cast", self.exprs([inner]))
            }
            _ => return None,
        })
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// Blocks, branches, loops, jumps and closures.
    fn control_shape(&mut self, expr: &'tcx Expr<'tcx>) -> Option<Shape> {
        let jump = |kind, label: Option<rustc_ast::Label>, children| {
            let mut shape = shape(kind, children);
            labeled(&mut shape, label);
            shape
        };
        Some(match expr.kind {
            ExprKind::Block(block, label) => self.block_shape(block, label),
            ExprKind::If(condition, then, otherwise) => shape(
                "if",
                self.exprs([condition, then].into_iter().chain(otherwise)),
            ),
            ExprKind::Let(condition) => {
                let pattern = self.pat(condition.pat);
                let init = self.exprs([condition.init]);
                shape(
                    "pattern_condition",
                    pattern.into_iter().chain(init).collect(),
                )
            }
            ExprKind::Loop(block, label, _, _) => {
                jump("loop", label, vec![self.block(block, None)])
            }
            ExprKind::Match(scrutinee, arms, _) => {
                let mut children = self.exprs([scrutinee]);
                children.extend(arms.iter().filter_map(|arm| self.arm(arm)));
                shape("match", children)
            }
            ExprKind::Closure(closure) => self.closure(closure),
            ExprKind::Break(destination, value) => {
                jump("break", destination.label, self.exprs(value))
            }
            ExprKind::Continue(destination) => jump("continue", destination.label, vec![]),
            ExprKind::Ret(value) => shape("return", self.exprs(value)),
            ExprKind::Become(value) => shape("return", self.exprs([value])),
            ExprKind::Yield(value, _) => shape("yield", self.exprs([value])),
            _ => return None,
        })
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// Literals, paths and aggregate values.
    fn value_shape(&mut self, expr: &'tcx Expr<'tcx>) -> Shape {
        match expr.kind {
            ExprKind::Lit(literal) => {
                let mut shape = shape("literal", vec![]);
                if let Some(value) = literal_value(&literal.node) {
                    shape.fields.insert("value".into(), value);
                }
                shape
            }
            ExprKind::Path(_) => shape("identifier", vec![]),
            ExprKind::Tup(values) => shape("tuple", self.exprs(values)),
            ExprKind::Array(values) => shape("array", self.exprs(values)),
            ExprKind::Repeat(value, _) => shape("array", self.exprs([value])),
            ExprKind::Struct(_, fields, tail) => {
                let base = match tail {
                    hir::StructTailExpr::Base(base) => Some(base),
                    _ => None,
                };
                let mut shape = shape(
                    "record_construction",
                    self.exprs(fields.iter().map(|f| f.expr).chain(base)),
                );
                let members: Vec<&str> = fields.iter().map(|f| f.ident.name.as_str()).collect();
                shape.fields.insert("members".into(), json!(members));
                shape
            }
            _ => {
                let reason = match expr.kind {
                    ExprKind::InlineAsm(_) => "Inline assembly is not modeled",
                    ExprKind::UnsafeBinderCast(..) => "Unsafe binder casts are not modeled",
                    _ => "Expression kind is not modeled",
                };
                let mut shape = shape("unsupported", vec![]);
                shape.fields.insert("reason".into(), json!(reason));
                shape
            }
        }
    }

    fn operator(
        &mut self,
        operator: &'static str,
        token: Span,
        operands: Vec<&'tcx Expr<'tcx>>,
    ) -> Shape {
        let mut shape = shape("operator", self.exprs(operands));
        shape.fields.insert("operator".into(), json!(operator));
        if let Some(token) = self.unit.span(token).filter(|_| !token.is_empty()) {
            shape.fields.insert("token".into(), token);
        }
        shape
    }

    fn closure(&mut self, closure: &'tcx hir::Closure<'tcx>) -> Shape {
        let body = self.tcx.hir_body(closure.body);
        let mut children: Vec<String> = body
            .params
            .iter()
            .filter_map(|param| self.pat(param.pat))
            .collect();
        children.extend(self.expr(body.value));
        let mut shape = shape("closure", children);
        if !matches!(closure.kind, hir::ClosureKind::Closure) {
            shape
                .fields
                .insert("modifiers".into(), json!(["coroutine"]));
        }
        shape
            .fields
            .insert("captures".into(), json!(self.captures(closure)));

        shape
    }

    /// The places a closure captures, by root binding, with how it holds each.
    fn captures(&mut self, closure: &'tcx hir::Closure<'tcx>) -> Vec<Value> {
        self.tcx
            .closure_captures(closure.def_id)
            .iter()
            .map(|capture| {
                let mode = match capture.info.capture_kind {
                    ty::UpvarCapture::ByValue => "value",
                    ty::UpvarCapture::ByUse => "use",
                    ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) => "shared",
                    ty::UpvarCapture::ByRef(ty::BorrowKind::Mutable) => "exclusive",
                    ty::UpvarCapture::ByRef(ty::BorrowKind::UniqueImmutable) => "unique",
                };
                let path: Vec<Value> = capture
                    .place
                    .projections
                    .iter()
                    .map(|projection| {
                        let mut step = json!({"type": self.unit.ty(projection.ty)});
                        step["kind"] = json!(match projection.kind {
                            ProjectionKind::Deref => "deref",
                            ProjectionKind::Field(..) => "field",
                            ProjectionKind::Index => "index",
                            ProjectionKind::Subslice => "rust:subslice",
                            ProjectionKind::OpaqueCast => "rust:opaque-cast",
                            ProjectionKind::UnwrapUnsafeBinder => "rust:unsafe-binder",
                        });
                        if let ProjectionKind::Field(field, _) = projection.kind {
                            step["member_index"] = json!(field.as_u32());
                        }
                        step
                    })
                    .collect();
                json!({"symbol": self.symbol_id(capture.get_root_variable()), "mode": mode,
                       "type": self.unit.ty(capture.place.ty()), "path": path})
            })
            .collect()
    }

    // Restored constructs.

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// `match IntoIterator::into_iter(head) { mut iter => loop { match next(&mut iter) {
    /// None => break, Some(pattern) => body } } }` as `for pattern in head { body }`.
    fn for_loop(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        scrutinee: &'tcx Expr<'tcx>,
        outer: &'tcx Arm<'tcx>,
    ) -> Option<String> {
        let (head, pattern, body) = for_parts(scrutinee, outer)?;
        self.claimed
            .extend([head.hir_id, pattern.hir_id, body.hir_id]);
        let pattern = self.pat(pattern);
        let head = self.expr(head);
        let iterable = self.lowered("for", |b| b.wrapper(scrutinee, head));
        let body = self.expr(body);
        let iterator = self.lowered("for", |b| b.pat(outer.pat));
        let iteration = self.lowered("for", |b| b.expr(outer.body));
        let children = [pattern, Some(iterable), body, iterator, iteration]
            .into_iter()
            .flatten()
            .collect();
        let mut shape = shape("for", children);
        if let ExprKind::Loop(_, label, _, _) = outer.body.kind {
            labeled(&mut shape, label);
        }

        Some(self.construct(expr.hir_id, expr.span, shape))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// `match Try::branch(operand) { Continue(v) => v, Break(r) => return from_residual(r) }`
    /// as `operand?`.
    fn propagate(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        scrutinee: &'tcx Expr<'tcx>,
        arms: &'tcx [Arm<'tcx>],
    ) -> Option<String> {
        let ExprKind::Call(_, [operand]) = scrutinee.kind else {
            return None;
        };
        self.claimed.insert(operand.hir_id);
        let operand = self.expr(operand);
        let branch = self.lowered("propagate", |b| b.wrapper(scrutinee, operand));
        let mut children = vec![branch];
        children.extend(self.lowered("propagate", |b| {
            arms.iter().filter_map(|arm| b.arm(arm)).collect::<Vec<_>>()
        }));
        let mut shape = shape("propagate", children);
        let span = expr.span.source_callsite();
        if let Some(token) = self
            .unit
            .span(span.with_lo(span.hi() - rustc_span::BytePos(1)))
        {
            shape.fields.insert("token".into(), token);
        }

        Some(self.construct(expr.hir_id, expr.span, shape))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// `loop { if condition { body } else { break } }` as `while condition { body }`.
    fn while_loop(
        &mut self,
        expr: &'tcx Expr<'tcx>,
        block: &'tcx Block<'tcx>,
        label: Option<rustc_ast::Label>,
    ) -> Option<String> {
        let Some(ExprKind::If(condition, body, _)) = block.expr.map(|tail| &tail.kind) else {
            return None;
        };
        self.claimed.extend([condition.hir_id, body.hir_id]);
        let condition = self.expr(condition);
        let body = self.expr(body);
        let lowering = self.lowered("while", |b| b.block(block, None));
        let mut shape = shape(
            "while",
            [condition, body, Some(lowering)]
                .into_iter()
                .flatten()
                .collect(),
        );
        labeled(&mut shape, label);

        Some(self.construct(expr.hir_id, expr.span, shape))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// `{ let (lhs, lhs) = value; a = lhs; b = lhs; }` as `(a, b) = value`.
    fn destructuring(&mut self, expr: &'tcx Expr<'tcx>, block: &'tcx Block<'tcx>) -> String {
        let places: HashMap<HirId, &'tcx Expr<'tcx>> =
            block.stmts.iter().filter_map(assigned_place).collect();
        let (pattern, value) = match block.stmts[0].kind {
            StmtKind::Let(local) => (local.pat, local.init),
            _ => unreachable!("a destructuring assignment starts with its let"),
        };
        let target = self.target(pattern, &places);
        let value = value.and_then(|value| self.expr(value));
        let children = [target, value].into_iter().flatten().collect();

        self.node(expr.hir_id, expr.span, shape("assignment", children))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// A destructuring target in source shape: each binding of the lowering's pattern
    /// stands for the place it is assigned to.
    fn target(
        &mut self,
        pat: &'tcx Pat<'tcx>,
        places: &HashMap<HirId, &'tcx Expr<'tcx>>,
    ) -> Option<String> {
        let (kind, parts): (_, Vec<&'tcx Pat<'tcx>>) = match pat.kind {
            PatKind::Binding(_, hir_id, _, None) => {
                return places.get(&hir_id).and_then(|place| self.expr(place));
            }
            PatKind::Tuple(pats, _) => ("tuple", pats.iter().collect()),
            PatKind::Slice(before, middle, after) => {
                ("array", before.iter().chain(middle).chain(after).collect())
            }
            PatKind::TupleStruct(_, pats, _) => ("record_construction", pats.iter().collect()),
            PatKind::Struct(_, fields, _) => (
                "record_construction",
                fields.iter().map(|field| field.pat).collect(),
            ),
            _ => return self.pat(pat),
        };
        let children = parts
            .into_iter()
            .filter_map(|part| self.target(part, places))
            .collect();

        Some(self.node(pat.hir_id, pat.span, shape(kind, children)))
    }

    /// A lowering's call that wraps a source operand, such as `into_iter(head)`.
    fn wrapper(&mut self, call: &'tcx Expr<'tcx>, operand: Option<String>) -> String {
        let callee = match call.kind {
            ExprKind::Call(callee, _) => self.expr(callee),
            _ => None,
        };
        let children = callee.into_iter().chain(operand).collect();
        let id = self.node(call.hir_id, call.span, shape("call", children));
        self.reference(call, &id);

        id
    }

    // Blocks, statements, arms and patterns.

    pub fn block(&mut self, block: &'tcx Block<'tcx>, label: Option<rustc_ast::Label>) -> String {
        let shape = self.block_shape(block, label);
        self.node(block.hir_id, block.span, shape)
    }

    fn block_shape(&mut self, block: &'tcx Block<'tcx>, label: Option<rustc_ast::Label>) -> Shape {
        let mut children: Vec<String> = block
            .stmts
            .iter()
            .filter_map(|stmt| self.stmt(stmt))
            .collect();
        children.extend(block.expr.and_then(|tail| self.expr(tail)));
        let mut shape = shape("block", children);
        labeled(&mut shape, label);
        match block.rules {
            BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided) => {
                shape.fields.insert("modifiers".into(), json!(["unsafe"]));
            }
            BlockCheckMode::UnsafeBlock(UnsafeSource::CompilerGenerated) => {
                shape
                    .fields
                    .insert("modifiers".into(), json!(["rust:compiler-unsafe"]));
            }
            BlockCheckMode::DefaultBlock => {}
        }

        shape
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    fn stmt(&mut self, stmt: &'tcx Stmt<'tcx>) -> Option<String> {
        let shape = match stmt.kind {
            StmtKind::Let(local) => {
                let mut children: Vec<String> = self.pat(local.pat).into_iter().collect();
                if let Some(ty) = local.ty {
                    children.push(self.node(ty.hir_id, ty.span, shape("type_annotation", vec![])));
                }
                children.extend(local.init.and_then(|init| self.expr(init)));
                children.extend(local.els.map(|els| self.block(els, None)));
                shape("local_declaration", children)
            }
            StmtKind::Item(_) => shape("item_declaration", vec![]),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => {
                shape("expression_statement", self.exprs([expr]))
            }
        };
        // A declaration is the `let` itself, as the frontend's facts identify it.
        let (id, span) = match stmt.kind {
            StmtKind::Let(local) => (local.hir_id, local.span),
            _ => (stmt.hir_id, stmt.span),
        };
        Some(self.node(id, span, shape))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    fn arm(&mut self, arm: &'tcx Arm<'tcx>) -> Option<String> {
        let mut children: Vec<String> = self.pat(arm.pat).into_iter().collect();
        if let Some(guard) = arm.guard {
            let condition = self.exprs([guard]);
            let span = guard.span;
            let id = format!("{}.guard", self.id(guard.hir_id));
            let provenance = self.provenance(span);
            let mut record = json!({"id": id, "kind": "match_guard", "original_kind": "match_guard",
                                    "children": condition, "provenance": provenance});
            match self.unit.span(span) {
                Some(own) => record["span"] = own,
                None => record["synthetic"] = json!(true),
            }
            if self.lowering.is_some() {
                record["synthetic"] = json!(true);
            }
            self.nodes.push(record);
            children.push(id);
        }
        children.extend(self.expr(arm.body));
        Some(self.node(arm.hir_id, arm.span, shape("match_arm", children)))
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    pub fn pat(&mut self, pat: &'tcx Pat<'tcx>) -> Option<String> {
        if self.lowering.is_some() && self.claimed.contains(&pat.hir_id) {
            return None;
        }
        let shape = match pat.kind {
            PatKind::Binding(mode, hir_id, ident, sub) => {
                let children = sub.and_then(|sub| self.pat(sub)).into_iter().collect();
                self.binding(hir_id, ident, mode.1.is_mut());
                shape("pattern_binding", children)
            }
            PatKind::Wild => shape("pattern_wildcard", vec![]),
            PatKind::Expr(_) => shape("pattern_literal", vec![]),
            PatKind::Range(start, end, ending) => {
                let bounds = start.into_iter().chain(end);
                let children = bounds.map(|bound| self.range_bound(bound)).collect();
                let mut shape = shape("pattern_range", children);
                if end.is_some() && matches!(ending, rustc_hir::RangeEnd::Included) {
                    shape
                        .fields
                        .insert("modifiers".into(), json!(["inclusive"]));
                }
                shape
            }
            PatKind::Or(pats) => shape("pattern_or", self.each(pats, Self::pat)),
            PatKind::Tuple(pats, _) => shape("pattern_tuple", self.each(pats, Self::pat)),
            PatKind::TupleStruct(_, pats, _) => {
                shape("pattern_constructor", self.each(pats, Self::pat))
            }
            PatKind::Struct(_, fields, _) => shape(
                "pattern_constructor",
                self.each(fields.iter().map(|field| field.pat), Self::pat),
            ),
            PatKind::Slice(before, middle, after) => {
                let children = self.each(before.iter().chain(middle).chain(after), Self::pat);
                // The element matching the rest of the sequence.
                if let Some(middle) = middle {
                    let id = self.id(middle.hir_id);
                    if let Some(record) = self
                        .nodes
                        .iter_mut()
                        .rev()
                        .find(|record| record["id"] == id)
                    {
                        record["modifiers"] = json!(["rest"]);
                    }
                }
                shape("pattern_sequence", children)
            }
            PatKind::Ref(inner, ..) => {
                let mut shape = shape("extension", self.each([inner], Self::pat));
                shape
                    .fields
                    .insert("extension".into(), json!("rust:reference-pattern"));
                shape
            }
            _ => {
                let (reason, children) = match pat.kind {
                    // The guarded pattern and its guard stay, so their statements still count.
                    PatKind::Guard(inner, guard) => {
                        let children = self.pat(inner).into_iter().chain(self.expr(guard));
                        ("Pattern guards are not modeled", children.collect())
                    }
                    PatKind::Deref(_) => ("Dereference patterns are not modeled", vec![]),
                    _ => ("Pattern kind is not modeled", vec![]),
                };
                let mut shape = shape("unsupported", children);
                shape.fields.insert("reason".into(), json!(reason));
                shape
            }
        };
        let id = self.node(pat.hir_id, pat.span, shape);
        self.pattern_reference(pat, &id);
        Some(id)
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// A range pattern's bound: a literal, or a path with what it resolves to.
    fn range_bound(&mut self, bound: &'tcx rustc_hir::PatExpr<'tcx>) -> String {
        let id = self.node(bound.hir_id, bound.span, shape("pattern_literal", vec![]));
        if let rustc_hir::PatExprKind::Path(ref qpath) = bound.kind
            && let Some(mut record) =
                self.path_reference(self.typeck.qpath_res(qpath, bound.hir_id))
        {
            record["node"] = json!(id);
            self.refs.push(record);
        }
        id
    }

    /// What a constructor, struct or path pattern's path resolves to.
    fn pattern_reference(&mut self, pat: &'tcx Pat<'tcx>, id: &str) {
        let qpath = match pat.kind {
            PatKind::TupleStruct(ref qpath, ..) | PatKind::Struct(ref qpath, ..) => qpath,
            PatKind::Expr(expr) => match expr.kind {
                rustc_hir::PatExprKind::Path(ref qpath) => qpath,
                rustc_hir::PatExprKind::Lit { .. } => return,
            },
            _ => return,
        };
        let res = self.typeck.qpath_res(qpath, pat.hir_id);
        if let Some(mut record) = self.path_reference(res) {
            record["node"] = json!(id);
            self.refs.push(record);
        }
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    /// The alternatives of an or-pattern share one binding, recorded at its first.
    fn binding(&mut self, hir_id: HirId, ident: rustc_span::Ident, mutable: bool) {
        if !self.bound.insert(hir_id) {
            return;
        }
        let kind = match () {
            _ if ident.name == rustc_span::symbol::kw::SelfLower
                && self.params.contains(&hir_id) =>
            {
                "receiver"
            }
            _ if self.params.contains(&hir_id) => "parameter",
            _ => "local",
        };
        let ty = self.unit.ty(self.typeck.node_type(hir_id));
        let mut symbol = json!({"id": self.symbol_id(hir_id), "name": ident.name.as_str(), "kind": kind,
                                "scope": self.prefix, "mutable": mutable, "node": self.id(hir_id), "type": ty});
        if let Some(declared) = self.unit.span(ident.span) {
            symbol["declared"] = declared;
        }

        self.symbols.push(symbol);
    }

    /// A parameter's symbol, and the symbols of every binding its pattern declares.
    /// Parameter patterns are not body nodes, so these symbols name no node.
    pub fn parameter(&mut self, pat: &'tcx Pat<'tcx>, position: usize) -> (String, Vec<String>) {
        let mut simple = None;
        let mut bindings = Vec::new();
        pat.each_binding(|mode, hir_id, _, ident| {
            if !self.bound.insert(hir_id) {
                return;
            }
            let kind = if ident.name == rustc_span::symbol::kw::SelfLower { "receiver" } else { "parameter" };
            let ty = self.unit.ty(self.typeck.node_type(hir_id));
            let mut symbol = json!({"id": self.symbol_id(hir_id), "name": ident.name.as_str(), "kind": kind,
                                    "scope": self.prefix, "mutable": mode.1.is_mut(), "type": ty});
            // A parameter a library macro declares is declared where the macro is invoked.
            let declared = self.unit.span(ident.span).or_else(|| self.unit.span(ident.span.source_callsite()));
            if let Some(declared) = declared {
                symbol["declared"] = declared;
            }
            self.symbols.push(symbol);
            bindings.push(self.symbol_id(hir_id));
            if pat.hir_id == hir_id {
                simple = Some(self.symbol_id(hir_id));
            }
        });
        let symbol = simple.unwrap_or_else(|| {
            let id = format!("s{}.param{position}", self.prefix);
            let ty = self.unit.ty(self.typeck.pat_ty(pat));
            self.symbols.push(
                json!({"id": id, "name": "_", "kind": "parameter", "scope": self.prefix,
                                     "mutable": false, "type": ty}),
            );
            id
        });

        (symbol, bindings)
    }

    // References.

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    fn reference(&mut self, expr: &'tcx Expr<'tcx>, id: &str) {
        let record = match expr.kind {
            ExprKind::Path(ref qpath) => {
                self.path_reference(self.typeck.qpath_res(qpath, expr.hir_id))
            }
            ExprKind::Call(callee, _) => {
                self.type_callee(callee);
                Some(self.call_reference(callee))
            }
            ExprKind::MethodCall(_, receiver, _, _) => Some(self.method_reference(expr, receiver)),
            ExprKind::Binary(..)
            | ExprKind::Unary(..)
            | ExprKind::Index(..)
            | ExprKind::AssignOp(..) => {
                let operator = self.typeck.type_dependent_def_id(expr.hir_id);
                let args = self.typeck.node_args(expr.hir_id);
                operator.map(|method| self.selected_call(method, args, "overloaded"))
            }
            _ => None,
        };
        if let Some(mut record) = record {
            record["node"] = json!(id);
            self.refs.push(record);
        }
    }

    // rstyle-allow(mutation_per_target): the builder accumulates nodes, symbols and references in self by design
    fn path_reference(&mut self, res: Res) -> Option<Value> {
        let target = match res {
            Res::Local(hir_id) => json!({"kind": "symbol", "id": self.symbol_id(hir_id)}),
            Res::Def(DefKind::Ctor(..), def_id) => {
                json!({"kind": "constructor", "id": self.unit.definition(def_id)})
            }
            // `Self(..)` in an impl names the self type's constructor.
            Res::SelfCtor(implementation) => {
                let owner = self
                    .tcx
                    .type_of(implementation)
                    .instantiate_identity()
                    .skip_norm_wip();
                let ctor = owner
                    .ty_adt_def()
                    .and_then(|adt| adt.non_enum_variant().ctor_def_id())?;
                json!({"kind": "constructor", "id": self.unit.definition(ctor)})
            }
            Res::Def(DefKind::Variant, def_id) => {
                json!({"kind": "variant", "id": self.unit.definition(def_id)})
            }
            Res::Def(_, def_id) => {
                json!({"kind": "definition", "id": self.unit.definition(def_id)})
            }
            _ => return None,
        };
        Some(json!({"status": "resolved", "target": target}))
    }

    fn call_reference(&mut self, callee: &'tcx Expr<'tcx>) -> Value {
        let callee_ty = self.typeck.expr_ty(callee);
        match *callee_ty.kind() {
            ty::FnDef(def_id, args) => self.selected_call(def_id, args.skip_binder(), "static"),
            ty::Closure(def_id, _) | ty::CoroutineClosure(def_id, _) => {
                let closure = def_id
                    .as_local()
                    .map(|local| self.id(self.tcx.local_def_id_to_hir_id(local)));
                let target = json!({"kind": "definition", "id": self.unit.value_definition()});
                json!({"status": "resolved", "dispatch": "function_value", "target": target, "closure": closure})
            }
            _ => {
                let target = json!({"kind": "definition", "id": self.unit.value_definition()});
                json!({"status": "resolved", "dispatch": "function_value", "target": target})
            }
        }
    }

    fn method_reference(&mut self, expr: &'tcx Expr<'tcx>, receiver: &'tcx Expr<'tcx>) -> Value {
        let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id) else {
            return json!({"status": "unresolved", "reason": "No method resolution"});
        };

        let receiver_ty = self.typeck.expr_ty_adjusted(receiver).peel_refs();
        let dispatch = if matches!(receiver_ty.kind(), ty::Dynamic(..)) {
            "dynamic"
        } else {
            "method"
        };
        let mut record = self.selected_call(def_id, self.typeck.node_args(expr.hir_id), dispatch);
        if compiler::iterator_member(self.tcx, def_id) {
            let receiver = self.typeck.expr_ty_adjusted(receiver);
            if let Some(item) = compiler::iterator_item(self.tcx, self.owner, receiver) {
                record["item"] = json!(self.unit.ty(item));
            }
        }
        record
    }

    /// A call's declared callee, and the implementation rustc selects for it when known.
    fn selected_call(
        &mut self,
        callee: hir::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
        dispatch: &str,
    ) -> Value {
        let declared = self.unit.definition(callee);
        let trait_item = self
            .tcx
            .opt_associated_item(callee)
            .and_then(|item| item.trait_container(self.tcx));
        let types: Vec<String> = args.types().map(|ty| self.unit.ty(ty)).collect();
        let mut record = if trait_item.is_some()
            && (args.has_non_region_param() || args.has_infer() || args.has_escaping_bound_vars())
        {
            json!({"status": "dependent", "dispatch": dispatch, "via": declared, "bounds": []})
        } else {
            json!({"status": "resolved", "dispatch": dispatch,
                   "target": {"kind": "definition", "id": declared}})
        };
        if !types.is_empty() {
            record["instance_arguments"] = json!(types);
        }
        if record["status"] == "dependent" {
            return record;
        }
        // An interface member runs the implementation static selection finds, if any.
        let (runs, runs_args) = match trait_item {
            Some(_) => match self.selection(callee, args) {
                Some(instance) => {
                    let selected = self.unit.definition(instance.def_id());
                    record["selected"] = json!({"kind": "definition", "id": selected});
                    (instance.def_id(), instance.args)
                }
                None => return record,
            },
            None => (callee, args),
        };
        if compiler::standard(self.tcx, runs) {
            let value = super::sysroot::calls_back(self.tcx, runs, runs_args);
            record["calls_back"] = json!({"value": value,
                "reason": "callable arguments, or bounds on types outside the standard library"});
        }

        record
    }

    /// The implementation static selection finds for an interface member with concrete
    /// arguments; none when the arguments do not instantiate the member fully.
    fn selection(
        &self,
        callee: hir::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<ty::Instance<'tcx>> {
        (args.len() == self.tcx.generics_of(callee).count()).then_some(())?;
        crate::ids::selected_instance(self.tcx, self.owner, callee, args)
            .filter(|instance| matches!(instance.def, ty::InstanceKind::Item(_)))
    }

    /// The emitted node of `hir_id`, or of its nearest emitted enclosing expression.
    pub fn nearest_node(&self, tcx: TyCtxt<'tcx>, hir_id: HirId) -> String {
        let found = std::iter::once(hir_id)
            .chain(tcx.hir_parent_id_iter(hir_id))
            .find(|candidate| self.emitted.contains(candidate));
        self.id(found.unwrap_or(hir_id))
    }

    /// The type of a called value, so core can tell function pointers from other callables.
    fn type_callee(&mut self, callee: &'tcx Expr<'tcx>) {
        let ty = self.unit.ty(self.typeck.expr_ty(callee));
        let id = self.id(callee.hir_id);
        if let Some(node) = self.nodes.iter_mut().rev().find(|node| node["id"] == id) {
            node["type"] = json!(ty);
        }
    }
}

fn labeled(shape: &mut Shape, label: Option<rustc_ast::Label>) {
    if let Some(label) = label {
        shape
            .fields
            .insert("label".into(), json!(label.ident.name.as_str()));
    }
}

fn for_parts<'tcx>(
    scrutinee: &'tcx Expr<'tcx>,
    outer: &'tcx Arm<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Pat<'tcx>, &'tcx Expr<'tcx>)> {
    let ExprKind::Call(_, [head]) = scrutinee.kind else {
        return None;
    };
    let ExprKind::Loop(block, _, LoopSource::ForLoop, _) = outer.body.kind else {
        return None;
    };
    let tail = block.expr.or_else(|| match block.stmts.last()?.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        _ => None,
    })?;
    let ExprKind::Match(_, [_, some], _) = tail.kind else {
        return None;
    };
    let pattern = match some.pat.kind {
        PatKind::TupleStruct(_, [pattern], _) => pattern,
        PatKind::Struct(_, [field], _) => field.pat,
        _ => return None,
    };

    Some((head, pattern, some.body))
}

/// The binding a destructuring assignment's statement reads, and the place it writes.
fn assigned_place<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<(HirId, &'tcx Expr<'tcx>)> {
    let (StmtKind::Semi(assign) | StmtKind::Expr(assign)) = stmt.kind else {
        return None;
    };
    let ExprKind::Assign(place, value, _) = assign.kind else {
        return None;
    };
    match value.kind {
        ExprKind::Path(QPath::Resolved(None, path)) => match path.res {
            Res::Local(binding) => Some((binding, place)),
            _ => None,
        },
        _ => None,
    }
}

/// A literal's value as the language parses it, for text, numbers, bools and chars.
fn literal_value(literal: &rustc_ast::LitKind) -> Option<Value> {
    use rustc_ast::LitKind;
    Some(match literal {
        LitKind::Str(text, _) => json!({"kind": "string", "value": text.as_str()}),
        LitKind::Int(value, _) => json!({"kind": "integer", "value": value.get().to_string()}),
        LitKind::Byte(value) => json!({"kind": "integer", "value": value.to_string()}),
        LitKind::Float(text, _) => json!({"kind": "float", "value": text.as_str()}),
        LitKind::Bool(value) => json!({"kind": "bool", "value": value.to_string()}),
        LitKind::Char(value) => json!({"kind": "char", "value": value.to_string()}),
        LitKind::ByteStr(..) | LitKind::CStr(..) => json!({"kind": "string"}),
        LitKind::Err(_) => return None,
    })
}

fn construct_name(kind: DesugaringKind) -> &'static str {
    match kind {
        DesugaringKind::ForLoop => "for",
        DesugaringKind::QuestionMark => "propagate",
        DesugaringKind::WhileLoop => "while",
        DesugaringKind::Await => "rust:await",
        DesugaringKind::RangeExpr => "rust:range",
        _ => "rust:desugaring",
    }
}

fn binary(op: BinOpKind) -> &'static str {
    match op {
        BinOpKind::Add => "add",
        BinOpKind::Sub => "sub",
        BinOpKind::Mul => "mul",
        BinOpKind::Div => "div",
        BinOpKind::Rem => "rem",
        BinOpKind::And => "and",
        BinOpKind::Or => "or",
        BinOpKind::BitXor => "bit_xor",
        BinOpKind::BitAnd => "bit_and",
        BinOpKind::BitOr => "bit_or",
        BinOpKind::Shl => "shl",
        BinOpKind::Shr => "shr",
        BinOpKind::Eq => "eq",
        BinOpKind::Lt => "lt",
        BinOpKind::Le => "le",
        BinOpKind::Ne => "ne",
        BinOpKind::Ge => "ge",
        BinOpKind::Gt => "gt",
    }
}

fn assign_op(op: hir::AssignOpKind) -> &'static str {
    use hir::AssignOpKind::*;
    match op {
        AddAssign => "add",
        SubAssign => "sub",
        MulAssign => "mul",
        DivAssign => "div",
        RemAssign => "rem",
        BitXorAssign => "bit_xor",
        BitAndAssign => "bit_and",
        BitOrAssign => "bit_or",
        ShlAssign => "shl",
        ShrAssign => "shr",
    }
}

fn unary(op: UnOp) -> &'static str {
    match op {
        UnOp::Deref => "deref",
        UnOp::Not => "not",
        UnOp::Neg => "neg",
    }
}
