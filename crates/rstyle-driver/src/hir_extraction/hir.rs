//! Resolved source-facing HIR records. No graph derivation lives here.

use rstyle_pdg::{extraction_profile::ExtractionProfile, frontend::*};
use rustc_hir::{self as hir, Expr, ExprKind, HirId, PatKind, StmtKind, def::Res};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeckResults};
use rustc_span::Span;

pub struct Context<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub typeck: &'tcx TypeckResults<'tcx>,
    pub function_span: Span,
    pub source_bytes: SourceText,
    pub sources: super::sources::Sources,
    pub unit_context: Option<rstyle_pdg::nominal_boundary::UnitContext>,
    pub result: Ty<'tcx>,
    pub profile: rstyle_pdg::extraction_profile::ExtractionProfile,
}

pub fn reference(id: HirId) -> HirRef {
    HirRef {
        krate: 0,
        owner: id.owner.def_id.local_def_index.as_u32(),
        local: id.local_id.as_u32(),
    }
}

pub fn symbol(id: HirId) -> String {
    format!(
        "s:{}:{}",
        id.owner.def_id.local_def_index.as_u32(),
        id.local_id.as_u32()
    )
}

pub fn resolved(ty: Ty<'_>, spelling: String) -> ResolvedTy {
    let normalized = match ty.kind() {
        ty::Int(_) | ty::Uint(_) | ty::Bool | ty::Char => Some(ty.to_string()),
        ty::Float(ty::FloatTy::F32 | ty::FloatTy::F64) => Some(ty.to_string()),
        ty::Tuple(types) if types.is_empty() => Some("unit".to_string()),
        _ => None,
    };

    ResolvedTy {
        normalized,
        observed: ty.to_string(),
        spelling,
    }
}

impl<'tcx> Context<'tcx> {
    pub fn span(&self, span: Span) -> ByteSpan {
        let span = span.source_callsite();
        let file = self
            .tcx
            .sess
            .source_map()
            .lookup_source_file(self.function_span.lo());
        let origin = file.original_relative_byte_pos(self.function_span.lo()).0;
        ByteSpan {
            start: file
                .original_relative_byte_pos(span.lo())
                .0
                .saturating_sub(origin),
            end: file
                .original_relative_byte_pos(span.hi())
                .0
                .saturating_sub(origin),
        }
    }

    pub fn snippet(&self, span: Span) -> String {
        let span = self.span(span);
        self.source_bytes
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
            .to_string()
    }

    pub fn node(&self, id: HirId, span: Span, kind: &str) -> HirNode {
        HirNode {
            hir: reference(id),
            kind: kind.to_string(),
            span: self.span(span),
            snippet: self.snippet(span),
            ty: None,
            res: None,
            dispatch: None,
            expansion: None,
            binding: None,
            binding_mut: None,
            lit: None,
            param_index: None,
            is_unsafe_block: false,
            children: Vec::new(),
        }
    }

    pub fn pattern(&self, pat: &hir::Pat<'tcx>) -> HirNode {
        if let Some(pattern) = self.observed_pattern(pat) {
            return pattern;
        }
        let constant = self.constant_pattern(pat);
        let range_constants = self.range_constants(pat);
        let (kind, span) = match pat.kind {
            PatKind::Binding(hir::BindingMode(hir::ByRef::No, _), _, ident, None) => {
                ("pat_binding", ident.span)
            }
            PatKind::Binding(..) => ("pat_binding_pattern", pat.span),
            PatKind::Wild => ("pat_wild", pat.span),
            PatKind::Or(_) if self.scalar_or_pattern(pat) => ("pat_or", pat.span),
            PatKind::Range(..) if range_constants.is_some() && self.scalar_range_pattern(pat) => {
                ("pat_range", pat.span)
            }
            PatKind::Expr(_) if constant.is_some() => ("pat_constant", pat.span),
            PatKind::Expr(expression)
                if matches!(expression.kind, hir::PatExprKind::Lit { .. }) =>
            {
                ("pat_literal", pat.span)
            }
            _ => ("pat_unsupported", pat.span),
        };
        let node = self.node(pat.hir_id, span, kind);
        let (binding, binding_mut) = match pat.kind {
            PatKind::Binding(mode, id, _, _) => (Some(symbol(id)), Some(mode.1.is_mut())),
            _ => (None, None),
        };

        HirNode {
            ty: self
                .typeck
                .node_type_opt(pat.hir_id)
                .map(|ty| resolved(ty, String::new())),
            binding,
            binding_mut,
            lit: constant
                .as_ref()
                .and_then(|resolution| {
                    rstyle_pdg::scalar_constants::ScalarConstant::decode(resolution).ok()
                })
                .map(|constant| constant.value)
                .or_else(|| {
                    self.pattern_literal(pat, kind, range_constants.as_deref().unwrap_or_default())
                }),
            res: constant,
            children: self.pattern_children(pat, kind, range_constants),
            ..node
        }
    }

    fn pattern_children(
        &self,
        pattern: &hir::Pat<'tcx>,
        kind: &str,
        constants: Option<Vec<HirNode>>,
    ) -> Vec<HirNode> {
        match pattern.kind {
            PatKind::Range(..) if kind == "pat_range" => constants.unwrap_or_default(),
            PatKind::Or(patterns) if kind == "pat_or" => patterns
                .iter()
                .map(|pattern| self.pattern(pattern))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn scalar_or_pattern(&self, pattern: &hir::Pat<'tcx>) -> bool {
        self.profile.supports("rust.scalar-or-patterns")
            && self.typeck.node_type_opt(pattern.hir_id).is_some_and(|ty| {
                matches!(ty.kind(), ty::Int(_) | ty::Uint(_) | ty::Bool)
                    || matches!(ty.kind(), ty::Char)
                        && self.profile.supports("rust.character-scalars")
            })
    }

    fn scalar_range_pattern(&self, pattern: &hir::Pat<'tcx>) -> bool {
        let PatKind::Range(start, end, _) = pattern.kind else {
            return false;
        };

        self.profile.supports("rust.scalar-range-patterns")
            && self.typeck.node_type_opt(pattern.hir_id).is_some_and(|ty| {
                matches!(ty.kind(), ty::Int(_) | ty::Uint(_))
                    || matches!(ty.kind(), ty::Char)
                        && self.profile.supports("rust.character-literal-ranges")
            })
            && start.into_iter().chain(end).all(|bound| {
                matches!(
                    bound.kind,
                    hir::PatExprKind::Lit { .. } | hir::PatExprKind::Path(_)
                )
            })
    }

    fn pattern_literal(
        &self,
        pattern: &hir::Pat<'tcx>,
        kind: &str,
        constants: &[HirNode],
    ) -> Option<String> {
        if kind == "pat_literal" {
            return Some(self.snippet(pattern.span));
        }
        if let PatKind::Range(start, end, ending) = pattern.kind
            && kind == "pat_range"
        {
            return Some(
                rstyle_pdg::scalar_patterns::IntegerRange {
                    start: start.map(|bound| self.range_bound_text(bound, constants)),
                    end: end.map(|bound| self.range_bound_text(bound, constants)),
                    inclusive: end.is_some() && matches!(ending, hir::RangeEnd::Included),
                }
                .text(),
            );
        }
        None
    }

    fn range_bound_text(&self, bound: &hir::PatExpr<'tcx>, constants: &[HirNode]) -> String {
        constants
            .iter()
            .find(|node| node.hir == reference(bound.hir_id))
            .and_then(|node| node.lit.clone())
            .unwrap_or_else(|| self.snippet(bound.span))
    }

    pub fn expression(&self, expr: &Expr<'tcx>) -> HirNode {
        if let Some(assignment) = self.tuple_assignment(expr) {
            return assignment;
        }
        if let Some(assertion) = self.assertion(expr) {
            return assertion;
        }
        if let ExprKind::DropTemps(inner) = expr.kind {
            return self.expression(inner);
        }

        let ty = if matches!(expr.kind, ExprKind::Ret(_)) {
            self.result
        } else {
            self.typeck.expr_ty(expr)
        };
        let tag = match expr.kind {
            ExprKind::Loop(_, Some(_), _, _) => "expr_labelled_loop",
            ExprKind::Loop(_, None, hir::LoopSource::While, _) => "expr_while",
            ExprKind::Loop(_, None, hir::LoopSource::ForLoop, _) => "expr_for",
            // Retain compiler continuations for observation, without treating them as source match arms.
            ExprKind::Match(_, _, hir::MatchSource::TryDesugar(_))
                if self.profile == ExtractionProfile::DataV3 =>
            {
                "expr_try_desugar"
            }
            ExprKind::Call(..) if self.tuple_constructor(expr) => "expr_struct",
            ExprKind::Path(..) if self.unit_constructor(expr) => "expr_struct",
            _ => kind(&expr.kind),
        };
        let base = self.node(expr.hir_id, expr.span, tag);
        let expansion = expr.span.from_expansion().then(|| ExpansionSite {
            callsite: self.span(expr.span.source_callsite()),
            snippet: self.snippet(expr.span),
        });
        let node = HirNode {
            ty: Some(resolved(ty, String::new())),
            expansion,
            ..base
        };

        self.expression_fields(expr, node)
    }

    fn tuple_constructor(&self, expr: &Expr<'tcx>) -> bool {
        if !self.profile.supports("rust.tuple-struct-values") {
            return false;
        }
        let ExprKind::Call(callee, _) = expr.kind else {
            return false;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return false;
        };

        matches!(
            self.typeck.qpath_res(path, callee.hir_id),
            Res::Def(
                rustc_hir::def::DefKind::Ctor(
                    rustc_hir::def::CtorOf::Struct,
                    rustc_hir::def::CtorKind::Fn
                ),
                _
            ) | Res::SelfCtor(_)
        )
    }

    fn unit_constructor(&self, expr: &Expr<'tcx>) -> bool {
        let ExprKind::Path(ref path) = expr.kind else {
            return false;
        };
        let constructor = matches!(
            self.typeck.qpath_res(path, expr.hir_id),
            Res::Def(
                rustc_hir::def::DefKind::Ctor(
                    rustc_hir::def::CtorOf::Struct,
                    rustc_hir::def::CtorKind::Const
                ),
                _
            ) | Res::SelfCtor(_)
        );

        constructor
            && self.profile.supports("rust.unit-struct-values")
            && matches!(self.typeck.expr_ty(expr).kind(), ty::Adt(..))
    }

    fn expression_fields(&self, expr: &Expr<'tcx>, node: HirNode) -> HirNode {
        let children = self.children(expr);
        if let Some((resolution, dispatch)) = self.scalar_instance_call(expr) {
            return HirNode {
                children,
                res: Some(resolution),
                dispatch: Some(dispatch),
                ..node
            };
        }
        let res = self.expression_resolution(expr, &node.kind);
        let lit = matches!(expr.kind, ExprKind::Lit(_)).then(|| self.snippet(expr.span));
        let is_unsafe_block = matches!(expr.kind, ExprKind::Block(block, _) if matches!(block.rules, hir::BlockCheckMode::UnsafeBlock(_)));

        HirNode {
            children,
            res,
            lit,
            dispatch: self.dispatch(expr),
            is_unsafe_block,
            ..node
        }
    }

    fn expression_resolution(&self, expr: &Expr<'tcx>, kind: &str) -> Option<PathRes> {
        self.float_literal(expr)
            .or_else(|| self.option_expression(expr))
            .or_else(|| self.result_expression(expr))
            .or_else(|| self.option_propagation(expr))
            .or_else(|| self.result_propagation(expr))
            .or_else(|| match expr.kind {
                ExprKind::Call(callee, _) => self.direct_callee(callee),
                ExprKind::Field(base, _) if self.profile.supports("rust.verified-helper-calls") => {
                    self.field_resolution(expr, base)
                }
                ExprKind::Path(_) if kind == "expr_struct" => None,
                ExprKind::Path(ref path) => self.path_resolution(expr, path),
                ExprKind::MethodCall(..) => self.method_callee(expr),
                ExprKind::Index(..) if self.profile.is_data() => self.method_callee(expr),
                ExprKind::Struct(path, ..) if self.range_desugaring(expr) => {
                    match self.typeck.qpath_res(path, expr.hir_id) {
                        Res::Def(_, definition) => {
                            let mut resolution = self.library_callee(definition);
                            resolution.kind = "resolved_range_type".into();
                            Some(resolution)
                        }
                        _ => None,
                    }
                }
                ExprKind::Break(destination, _) | ExprKind::Continue(destination) => {
                    Some(PathRes {
                        kind: if destination.label.is_none() {
                            "innermost_loop"
                        } else {
                            "labelled_loop"
                        }
                        .into(),
                        target: destination.target_id.ok().map(reference),
                        description: "Compiler-resolved loop exit destination".into(),
                    })
                }
                _ => None,
            })
    }

    fn path_resolution(&self, expr: &Expr<'tcx>, path: &hir::QPath<'tcx>) -> Option<PathRes> {
        let res = self.typeck.qpath_res(path, expr.hir_id);
        self.named_option_function(expr)
            .map(|(resolution, _)| resolution)
            .or_else(|| self.fieldless_variant(res))
            .or_else(|| self.bool_array_resolution(res))
            .or_else(|| self.constant_resolution(res))
            .or_else(|| self.primitive_constant_resolution(res))
            .or_else(|| {
                Some(PathRes {
                    kind: if matches!(res, Res::Local(_)) {
                        "local"
                    } else {
                        "other"
                    }
                    .into(),
                    target: if let Res::Local(id) = res {
                        Some(reference(id))
                    } else {
                        None
                    },
                    description: format!("{res:?}"),
                })
            })
    }

    fn fieldless_variant(&self, res: Res) -> Option<PathRes> {
        if !self.profile.supports("rust.fieldless-enum-values") {
            return None;
        }

        let variant = match res {
            Res::Def(rustc_hir::def::DefKind::Variant, variant) => variant,
            Res::Def(
                rustc_hir::def::DefKind::Ctor(
                    rustc_hir::def::CtorOf::Variant,
                    rustc_hir::def::CtorKind::Const,
                ),
                constructor,
            ) => self.tcx.parent(constructor),
            _ => return None,
        };

        if !variant.is_local() {
            return None;
        }

        let mut resolution = self.library_callee(variant);
        resolution.kind = "resolved_fieldless_variant".into();
        Some(resolution)
    }

    fn field_resolution(&self, expr: &Expr<'tcx>, base: &Expr<'tcx>) -> Option<PathRes> {
        let ty = self.typeck.expr_ty_adjusted(base);
        let owner_path = match ty.kind() {
            ty::Adt(definition, _) => Some(self.tcx.def_path_str(definition.did())),
            ty::Tuple(_) => None,
            _ => return None,
        };
        let resolution = rstyle_pdg::field_locations::FieldResolution {
            position: self.typeck.field_index(expr.hir_id).as_usize(),
            owner_path,
        };

        Some(PathRes {
            kind: "resolved_field".into(),
            target: None,
            description: serde_json::to_string(&resolution).expect("plain field facts serialize"),
        })
    }

    fn while_children(&self, block: &hir::Block<'tcx>) -> Vec<HirNode> {
        match block.expr.map(|expression| &expression.kind) {
            Some(ExprKind::If(condition, body, _)) => {
                vec![self.expression(condition), self.expression(body)]
            }
            _ => Vec::new(),
        }
    }

    fn children(&self, expr: &Expr<'tcx>) -> Vec<HirNode> {
        match expr.kind {
            ExprKind::Closure(_) => self.callback_children(expr).unwrap_or_default(),
            ExprKind::Loop(block, None, hir::LoopSource::Loop, _) => vec![self.loop_block(block)],
            ExprKind::Loop(block, None, hir::LoopSource::While, _) => self.while_children(block),
            ExprKind::Break(_, value) => value.into_iter().map(|e| self.expression(e)).collect(),
            ExprKind::Block(block, _) => self.block_children(block),
            ExprKind::Binary(_, left, right)
            | ExprKind::Assign(left, right, _)
            | ExprKind::AssignOp(_, left, right)
            | ExprKind::Index(left, right, _) => {
                vec![self.expression(left), self.expression(right)]
            }
            ExprKind::Unary(_, inner)
            | ExprKind::Cast(inner, _)
            | ExprKind::AddrOf(_, _, inner) => vec![self.expression(inner)],
            ExprKind::If(cond, yes, no) => [Some(cond), Some(yes), no]
                .into_iter()
                .flatten()
                .map(|e| self.expression(e))
                .collect(),
            ExprKind::Let(condition) if self.profile.supports("rust.scalar-option-if-let") => {
                vec![self.pattern(condition.pat), self.expression(condition.init)]
            }
            ExprKind::Match(value, arms, source)
                if source == hir::MatchSource::Normal
                    || matches!(source, hir::MatchSource::TryDesugar(_))
                        && self.profile == ExtractionProfile::DataV3 =>
            {
                std::iter::once(self.expression(value))
                    .chain(arms.iter().map(|arm| self.match_arm(arm)))
                    .collect()
            }
            ExprKind::Ret(value) => value.into_iter().map(|e| self.expression(e)).collect(),
            ExprKind::Tup(values) => values.iter().map(|e| self.expression(e)).collect(),
            ExprKind::Struct(..) | ExprKind::Array(..) | ExprKind::Repeat(..) => {
                self.aggregate_children(expr)
            }
            ExprKind::MethodCall(_, receiver, args, _) => self.call_children(Some(receiver), args),
            ExprKind::Call(_, args) => self.call_children(None, args),
            ExprKind::Field(base, _) if self.profile.supports("rust.verified-helper-calls") => {
                vec![self.expression(base)]
            }
            _ => Vec::new(),
        }
    }

    fn aggregate_children(&self, expr: &Expr<'tcx>) -> Vec<HirNode> {
        match expr.kind {
            ExprKind::Struct(_, fields, tail)
                if self.profile.supports("rust.named-struct-values") =>
            {
                let base = match tail {
                    hir::StructTailExpr::None => None,
                    hir::StructTailExpr::Base(base)
                        if self.profile.supports("rust.struct-update-values") =>
                    {
                        Some(base)
                    }
                    _ => return Vec::new(),
                };
                fields
                    .iter()
                    .map(|field| field.expr)
                    .chain(base)
                    .map(|value| self.expression(value))
                    .collect()
            }
            ExprKind::Array(values) if self.profile.supports("rust.aggregate-literal-values") => {
                values.iter().map(|e| self.expression(e)).collect()
            }
            ExprKind::Repeat(value, _) if self.profile.supports("rust.repeat-array-values") => {
                vec![self.expression(value)]
            }
            _ => Vec::new(),
        }
    }

    fn call_children(
        &self,
        receiver: Option<&Expr<'tcx>>,
        arguments: &[Expr<'tcx>],
    ) -> Vec<HirNode> {
        receiver
            .into_iter()
            .chain(arguments)
            .map(|value| self.expression(value))
            .collect()
    }

    fn loop_block(&self, block: &hir::Block<'tcx>) -> HirNode {
        HirNode {
            children: self.block_children(block),
            ty: Some(resolved(self.tcx.types.unit, String::new())),
            ..self.node(block.hir_id, block.span, "expr_block")
        }
    }

    fn match_arm(&self, arm: &hir::Arm<'tcx>) -> HirNode {
        let guard = arm.guard.map(|guard| HirNode {
            children: vec![self.expression(guard)],
            ..self.node(guard.hir_id, guard.span, "match_guard")
        });
        HirNode {
            children: std::iter::once(self.pattern(arm.pat))
                .chain(guard)
                .chain(std::iter::once(self.expression(arm.body)))
                .collect(),
            ..self.node(arm.hir_id, arm.span, "match_arm")
        }
    }

    fn block_children(&self, block: &hir::Block<'tcx>) -> Vec<HirNode> {
        block
            .stmts
            .iter()
            .map(|stmt| self.statement(stmt))
            .chain(block.expr.map(|e| self.expression(e)))
            .collect()
    }

    fn statement(&self, stmt: &hir::Stmt<'tcx>) -> HirNode {
        match stmt.kind {
            StmtKind::Let(local) => self.declaration(local),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => {
                let kind = if matches!(stmt.kind, StmtKind::Semi(_)) {
                    "stmt_semi"
                } else {
                    "stmt_expr"
                };
                HirNode {
                    children: vec![self.expression(expr)],
                    ..self.node(stmt.hir_id, stmt.span, kind)
                }
            }
            StmtKind::Item(id) => {
                let item = self.tcx.hir_item(id);
                let declaration = self.const_array_item(item).or_else(|| {
                    self.native_size_constant(item)
                        .map(|res| ("stmt_size_const", res))
                });
                let node = self.node(stmt.hir_id, item.span, "stmt_item");
                match declaration {
                    Some((kind, res)) => HirNode {
                        kind: kind.into(),
                        res: Some(res),
                        ..node
                    },
                    None => node,
                }
            }
        }
    }

    fn declaration(&self, local: &hir::LetStmt<'tcx>) -> HirNode {
        let annotation = local.ty.map(|ty| HirNode {
            ty: self
                .typeck
                .node_type_opt(local.pat.hir_id)
                .map(|resolved_ty| resolved(resolved_ty, self.snippet(ty.span))),
            ..self.node(ty.hir_id, ty.span, "ty_annotation")
        });
        let children = std::iter::once(self.pattern(local.pat))
            .chain(annotation)
            .chain(local.init.map(|e| self.expression(e)))
            .chain(local.els.map(|b| self.node(b.hir_id, b.span, "let_else")))
            .collect();

        HirNode {
            children,
            ..self.node(local.hir_id, local.span, "stmt_let")
        }
    }

    pub(super) fn dispatch_operands(
        &self,
        expr: &Expr<'tcx>,
    ) -> Option<(String, Vec<&'tcx Expr<'tcx>>)> {
        Some(match expr.kind {
            ExprKind::Binary(op, left, right) => (op.node.as_str().to_string(), vec![left, right]),
            ExprKind::AssignOp(op, left, right) => {
                (op.node.as_str().to_string(), vec![left, right])
            }
            ExprKind::Unary(op, value) => (op.as_str().to_string(), vec![value]),
            ExprKind::Index(base, index, _) => ("[]".into(), vec![base, index]),
            ExprKind::Cast(value, _) => ("as".to_string(), vec![value]),
            ExprKind::Call(callee, args) => {
                let resolution = self.direct_callee(callee)?;
                let value: serde_json::Value =
                    serde_json::from_str(&resolution.description).ok()?;
                let path = if resolution.kind == rstyle_pdg::size_calls::RESOLUTION {
                    &value["callee"]["path"]
                } else {
                    &value["path"]
                };
                (path.as_str()?.to_string(), args.iter().collect())
            }
            ExprKind::MethodCall(_, receiver, args, _) => (
                self.tcx
                    .def_path_str(self.typeck.type_dependent_def_id(expr.hir_id)?),
                std::iter::once(receiver).chain(args.iter()).collect(),
            ),
            _ => return None,
        })
    }

    fn dispatch(&self, expr: &Expr<'tcx>) -> Option<Dispatch> {
        if let Some((_, dispatch)) = self.named_option_function(expr) {
            return Some(dispatch);
        }
        let (op, operands) = self.dispatch_operands(expr)?;
        let operand_tys: Vec<String> = operands
            .iter()
            .map(|e| {
                resolved(self.typeck.expr_ty(e), String::new())
                    .normalized
                    .unwrap_or_else(|| self.typeck.expr_ty(e).to_string())
            })
            .collect();
        let overloaded = self.typeck.type_dependent_def_id(expr.hir_id).is_some();

        Some(Dispatch {
            kind: if matches!(expr.kind, ExprKind::Call(..)) {
                "resolved_direct"
            } else if matches!(expr.kind, ExprKind::MethodCall(..)) {
                "resolved_method"
            } else if overloaded {
                "overloaded"
            } else {
                "builtin"
            }
            .to_string(),
            op,
            operand_tys,
            result_ty: resolved(self.typeck.expr_ty(expr), String::new())
                .normalized
                .unwrap_or_else(|| self.typeck.expr_ty(expr).to_string()),
        })
    }
}

impl Context<'_> {
    fn method_callee(&self, expression: &Expr<'_>) -> Option<PathRes> {
        let callee = self.typeck.type_dependent_def_id(expression.hir_id)?;
        if let Some(owner) = self.inherent_callee(expression, callee) {
            return Some(self.helper_callee(callee, "resolved_inherent_callee", Some(owner)));
        }
        let capture = self.profile.supports("rust.byte-iterator-position")
            && matches!(
                self.tcx.def_path_str(callee).as_str(),
                "core::iter::Iterator::position" | "std::iter::Iterator::position"
            )
            || self.profile.supports("rust.slice-array-byte-calls")
                && self
                    .tcx
                    .def_path_str(callee)
                    .ends_with("::convert::TryInto::try_into");
        Some(self.instantiated_callee(expression, callee, capture))
    }

    fn instantiated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
        capture: bool,
    ) -> PathRes {
        let mut resolution = self.library_callee(callee);
        if capture {
            let mut record: rstyle_pdg::data_library::Callee =
                serde_json::from_str(&resolution.description).expect("captured library callee");
            record.implementation = self.selected_implementation(expression, callee);
            resolution.description =
                serde_json::to_string(&record).expect("captured library callee");
        }
        resolution
    }

    fn scalar_from_capture(&self, expression: &Expr<'_>, callee: rustc_hir::def_id::DefId) -> bool {
        if !self.profile.supports("rust.scalar-from-conversions")
            || !self
                .tcx
                .def_path_str(callee)
                .ends_with("::convert::From::from")
        {
            return false;
        }
        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();

        signature.inputs() == [self.tcx.types.u16] && signature.output() == self.tcx.types.u32
            || self.profile.supports("rust.usize-from-u8")
                && signature.inputs() == [self.tcx.types.u8]
                && signature.output() == self.tcx.types.usize
            || self.profile.supports("rust.tagged-unit-classifiers")
                && signature.inputs() == [self.tcx.types.u16]
                && signature.output() == self.tcx.types.usize
    }

    fn int_conversion_capture(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> bool {
        if !self.profile.supports("rust.bitset-cardinality")
            || !self
                .tcx
                .def_path_str(callee)
                .ends_with("::convert::TryFrom::try_from")
        {
            return false;
        }
        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();
        let ty::Adt(definition, arguments) = signature.output().kind() else {
            return false;
        };

        self.tcx
            .is_diagnostic_item(rustc_span::sym::Result, definition.did())
            && (signature.inputs() == [self.tcx.types.u32]
                && arguments.type_at(0) == self.tcx.types.usize
                || self.profile.supports("rust.end-of-input")
                    && signature.inputs() == [self.tcx.types.usize]
                    && arguments.type_at(0) == self.tcx.types.u16)
    }

    // Observational selection only; this does not authorize trait calls or
    // replace independent source ownership, type, effect or drop evidence.
    fn local_from_capture(&self, expression: &Expr<'_>, callee: rustc_hir::def_id::DefId) -> bool {
        self.tcx
            .associated_item(callee)
            .trait_container(self.tcx)
            .is_some_and(|definition| {
                self.tcx
                    .is_diagnostic_item(rustc_span::sym::From, definition)
            })
            && self.tcx.item_name(callee) == rustc_span::sym::from
            && self
                .selected_callee(expression, callee)
                .is_some_and(|selected| {
                    selected.is_local() && self.tcx.generics_of(selected).count() == 0
                })
    }

    fn range_from_helper(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<PathRes> {
        if !self.profile.supports("rust.fresh-range-results")
            || !self.local_from_capture(expression, callee)
        {
            return None;
        }

        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();
        let ty::Adt(definition, arguments) = signature.output().kind() else {
            return None;
        };
        if Some(definition.did()) != self.tcx.lang_items().range_struct()
            || arguments.len() != 1
            || arguments.type_at(0) != self.tcx.types.usize
        {
            return None;
        }

        let selected = self.selected_callee(expression, callee)?;
        let mut resolution = self.helper_callee(selected, "resolved_direct_callee", None);
        let mut record: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
        let selection = self.instantiated_callee(expression, callee, true);
        record["trait_selection"] = serde_json::from_str(&selection.description).ok()?;
        resolution.description = record.to_string();

        Some(resolution)
    }

    fn selected_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<rustc_hir::def_id::DefId> {
        let environment = ty::TypingEnv::post_analysis(self.tcx, expression.hir_id.owner.def_id);
        ty::Instance::try_resolve(
            self.tcx,
            environment,
            callee,
            self.typeck.node_args(expression.hir_id),
        )
        .ok()
        .flatten()
        .map(|instance| instance.def_id())
    }

    fn selected_implementation(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<Box<rstyle_pdg::data_library::Callee>> {
        let implementation = self.library_callee(self.selected_callee(expression, callee)?);
        Some(Box::new(
            serde_json::from_str(&implementation.description).expect("captured implementation"),
        ))
    }

    fn range_desugaring(&self, expression: &Expr<'_>) -> bool {
        self.profile.is_data()
            && matches!(
                expression.span.ctxt().outer_expn_data().kind,
                rustc_span::hygiene::ExpnKind::Desugaring(
                    rustc_span::hygiene::DesugaringKind::RangeExpr
                )
            )
    }

    pub(super) fn library_callee(&self, callee: rustc_hir::def_id::DefId) -> PathRes {
        let callee = rstyle_pdg::data_library::Callee {
            path: self.tcx.def_path_str(callee),
            crate_name: self.tcx.crate_name(callee.krate).to_string(),
            crate_hash: if callee.is_local() {
                "local-unencoded".into()
            } else {
                self.tcx.crate_hash(callee.krate).to_string()
            },
            definition_hash: format!("{:?}", self.tcx.def_path_hash(callee)),
            implementation: None,
        };

        PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_value(callee)
                .expect("serialize resolved callee")
                .to_string(),
        }
    }

    fn direct_callee(&self, expression: &Expr<'_>) -> Option<PathRes> {
        if !self.profile.supports("rust.verified-helper-calls") {
            return None;
        }
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };
        let Res::Def(kind, callee) = self.typeck.qpath_res(path, expression.hir_id) else {
            return None;
        };
        if self.profile.supports("rust.scalar-enum-values")
            && callee.is_local()
            && matches!(
                kind,
                rustc_hir::def::DefKind::Ctor(rustc_hir::def::CtorOf::Variant, _)
            )
        {
            return Some(self.library_callee(callee));
        }
        if !callee.is_local() && kind == rustc_hir::def::DefKind::AssocFn && self.profile.is_data()
        {
            return Some(self.external_associated_callee(expression, callee));
        }
        if kind == rustc_hir::def::DefKind::Fn
            && let Some(resolution) = self.size_callee(expression, callee)
        {
            return Some(resolution);
        }
        let inherent = self.inherent_callee(expression, callee);
        if self.tcx.generics_of(callee).count() != 0 && inherent.is_none() {
            return None;
        }
        if kind != rustc_hir::def::DefKind::Fn && inherent.is_none() {
            return None;
        }

        let dependency = self.compiled_dependency(expression, callee, kind);
        Some(self.helper_resolution(
            callee,
            "resolved_direct_callee",
            HelperEvidence {
                owner: inherent,
                compiled_dependency: dependency,
            },
        ))
    }

    fn external_associated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> PathRes {
        if let Some(resolution) = super::trait_members::default_call(self, expression, callee) {
            return resolution;
        }
        if let Some(resolution) = self.range_from_helper(expression, callee) {
            return resolution;
        }

        self.associated_callee(expression, callee)
    }

    fn associated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> PathRes {
        let capture = self.scalar_from_capture(expression, callee)
            || self.int_conversion_capture(expression, callee)
            || self.local_from_capture(expression, callee);
        self.instantiated_callee(expression, callee, capture)
    }

    fn compiled_dependency(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
        kind: rustc_hir::def::DefKind,
    ) -> Option<super::crate_association::Dependency> {
        (super::crate_association::enabled(self.profile)
            && kind == rustc_hir::def::DefKind::Fn
            && self.typeck.node_args(expression.hir_id).is_empty())
        .then(|| super::crate_association::dependency(self.tcx, callee))
        .flatten()
    }

    fn inherent_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<rustc_hir::def_id::DefId> {
        let owner = super::inherent::implementation(self.tcx, callee, self.profile)?;
        self.typeck
            .node_args(expression.hir_id)
            .iter()
            .all(|argument| {
                matches!(
                    argument.kind(),
                    rustc_middle::ty::GenericArgKind::Lifetime(_)
                )
            })
            .then_some(owner)
    }

    pub(super) fn helper_callee(
        &self,
        callee: rustc_hir::def_id::DefId,
        kind: &str,
        owner: Option<rustc_hir::def_id::DefId>,
    ) -> PathRes {
        self.helper_resolution(
            callee,
            kind,
            HelperEvidence {
                owner,
                compiled_dependency: None,
            },
        )
    }

    fn helper_resolution(
        &self,
        callee: rustc_hir::def_id::DefId,
        kind: &str,
        evidence: HelperEvidence,
    ) -> PathRes {
        PathRes {
            kind: kind.into(),
            target: None,
            description: serde_json::to_string(&DirectCallee {
                path: self.tcx.def_path_str(callee),
                crate_name: self.tcx.crate_name(callee.krate).to_string(),
                local: callee.is_local(),
                definition_identity: format!("{:?}", self.tcx.def_path_hash(callee)),
                inherent: evidence
                    .owner
                    .map(|owner| super::inherent::call_owner(self.tcx, owner)),
                compiled_dependency: evidence.compiled_dependency,
            })
            .expect("resolved callee record"),
        }
    }
}

struct HelperEvidence {
    owner: Option<rustc_hir::def_id::DefId>,
    compiled_dependency: Option<super::crate_association::Dependency>,
}

#[derive(serde::Serialize)]
struct DirectCallee {
    path: String,
    #[serde(rename = "crate")]
    crate_name: String,
    local: bool,
    definition_identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    inherent: Option<rstyle_pdg::inherent_calls::CallOwner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiled_dependency: Option<super::crate_association::Dependency>,
}

fn kind(kind: &ExprKind<'_>) -> &'static str {
    match kind {
        ExprKind::Block(..) => "expr_block",
        ExprKind::Binary(..) => "expr_binary",
        ExprKind::Unary(..) => "expr_unary",
        ExprKind::Lit(..) => "expr_lit",
        ExprKind::Path(..) => "expr_path",
        ExprKind::Tup(..) => "expr_tup",
        ExprKind::If(..) => "expr_if",
        ExprKind::Ret(..) => "expr_ret",
        ExprKind::Assign(..) => "expr_assign",
        ExprKind::AssignOp(..) => "expr_assign_op",
        ExprKind::Call(..) => "expr_call",
        ExprKind::MethodCall(..) => "expr_method_call",
        ExprKind::Closure(..) => "expr_closure",
        ExprKind::ConstBlock(..) => "expr_const_block",
        ExprKind::AddrOf(..) => "expr_addr_of",
        ExprKind::Field(..) => "expr_field",
        ExprKind::Index(..) => "expr_index",
        ExprKind::Loop(..) => "expr_loop",
        ExprKind::Match(..) => "expr_match",
        ExprKind::Cast(..) => "expr_cast",
        ExprKind::Break(..) => "expr_break",
        ExprKind::Continue(..) => "expr_continue",
        ExprKind::Let(..) => "expr_let",
        ExprKind::Yield(..) => "expr_yield",
        ExprKind::Array(..) => "expr_array",
        ExprKind::Repeat(..) => "expr_repeat",
        ExprKind::Struct(..) => "expr_struct",
        _ => "expr_unsupported",
    }
}

pub fn bindings(node: &HirNode, scope: &str) -> Vec<BindingFact> {
    let local_scope = match node.kind.as_str() {
        "expr_block" => Some("block"),
        "expr_closure" if !node.children.is_empty() => Some("callback"),
        "match_arm"
            if node
                .children
                .first()
                .is_some_and(|pattern| pattern.kind == "pat_slice_prefix") =>
        {
            Some("slice-arm")
        }
        _ => None,
    }
    .map(|kind| format!("scope:source:{kind}:{}:{}", node.span.start, node.span.end));
    let scope = local_scope.as_deref().unwrap_or(scope);
    let binding = binding_fact(node, scope);

    binding
        .into_iter()
        .chain(
            node.children
                .iter()
                .flat_map(|child| bindings(child, scope)),
        )
        .collect()
}

fn binding_fact(node: &HirNode, scope: &str) -> Option<BindingFact> {
    let symbol = node.binding.as_ref()?;
    let ty = node.ty.as_ref()?;
    let (name, decl_span) = binding_identity(node)?;
    Some(BindingFact {
        symbol: symbol.clone(),
        name,
        decl_span,
        scope: scope.to_string(),
        hir: node.hir.clone(),
        is_param: node.param_index.is_some(),
        is_mut: node.binding_mut.unwrap_or(false),
        ty: ty.clone(),
    })
}

fn binding_identity(node: &HirNode) -> Option<(String, ByteSpan)> {
    if node.kind != "pat_binding_at" {
        return Some((node.snippet.clone(), node.span));
    }
    let observed = rstyle_pdg::slice_patterns::Observation::decode(node).ok()?;
    let rstyle_pdg::slice_patterns::Details::Binding { binding, .. } = observed.details else {
        return None;
    };
    Some((binding.name, binding.identifier_span))
}
