//! Resolved source-facing HIR records. No graph derivation lives here.

use rstyle_pdg::frontend::*;
use rustc_hir::{self as hir, Expr, ExprKind, HirId, PatKind, StmtKind, def::Res};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeckResults};
use rustc_span::Span;

pub struct Context<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub typeck: &'tcx TypeckResults<'tcx>,
    pub function_span: Span,
    pub source_bytes: String,
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
        ty::Int(_) | ty::Uint(_) | ty::Bool => Some(ty.to_string()),
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
        let (kind, span) = match pat.kind {
            PatKind::Binding(hir::BindingMode(hir::ByRef::No, _), _, ident, None) => {
                ("pat_binding", ident.span)
            }
            PatKind::Binding(..) => ("pat_binding_pattern", pat.span),
            PatKind::Wild => ("pat_wild", pat.span),
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
            lit: (kind == "pat_literal").then(|| self.snippet(pat.span)),
            ..node
        }
    }

    pub fn expression(&self, expr: &Expr<'tcx>) -> HirNode {
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

    fn expression_fields(&self, expr: &Expr<'tcx>, node: HirNode) -> HirNode {
        let children = self.children(expr);
        let res = match expr.kind {
            ExprKind::Call(callee, _) => self.direct_callee(callee),
            ExprKind::Field(base, _) if self.profile.supports("rust.verified-helper-calls") => self.field_resolution(expr, base),
            ExprKind::Path(ref path) => {
                let res = self.typeck.qpath_res(path, expr.hir_id);
                Some(PathRes {
                    kind: if matches!(res, Res::Local(_)) {
                        "local"
                    } else {
                        "other"
                    }
                    .to_string(),
                    target: if let Res::Local(id) = res {
                        Some(reference(id))
                    } else {
                        None
                    },
                    description: format!("{res:?}"),
                })
            }
            ExprKind::MethodCall(..) => self.typeck.type_dependent_def_id(expr.hir_id).map(|callee| PathRes {
                kind: "resolved_callee".into(), target: None,
                description: serde_json::json!({"path":self.tcx.def_path_str(callee),"crate":self.tcx.crate_name(callee.krate).to_string(),"crate_hash":if callee.is_local() { "local-unencoded".to_string() } else { self.tcx.crate_hash(callee.krate).to_string() },"definition_hash":format!("{:?}", self.tcx.def_path_hash(callee))}).to_string(),
            }),
            ExprKind::Break(destination, _) | ExprKind::Continue(destination) => Some(PathRes {
                kind: if destination.label.is_none() {
                    "innermost_loop"
                } else {
                    "labelled_loop"
                }
                .into(),
                target: destination.target_id.ok().map(reference),
                description: "Compiler-resolved loop exit destination".into(),
            }),
            _ => None,
        };
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

    fn children(&self, expr: &Expr<'tcx>) -> Vec<HirNode> {
        match expr.kind {
            ExprKind::Loop(block, None, hir::LoopSource::Loop, _) => vec![self.loop_block(block)],
            ExprKind::Loop(block, None, hir::LoopSource::While, _) => {
                match block.expr.map(|e| &e.kind) {
                    Some(ExprKind::If(condition, body, _)) => {
                        vec![self.expression(condition), self.expression(body)]
                    }
                    _ => Vec::new(),
                }
            }
            ExprKind::Break(_, value) => value.into_iter().map(|e| self.expression(e)).collect(),
            ExprKind::Block(block, _) => self.block_children(block),
            ExprKind::Binary(_, left, right)
            | ExprKind::Assign(left, right, _)
            | ExprKind::AssignOp(_, left, right) => {
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
            ExprKind::Match(value, arms, hir::MatchSource::Normal) => {
                std::iter::once(self.expression(value))
                    .chain(arms.iter().map(|arm| self.match_arm(arm)))
                    .collect()
            }
            ExprKind::Ret(value) => value.into_iter().map(|e| self.expression(e)).collect(),
            ExprKind::Tup(values) => values.iter().map(|e| self.expression(e)).collect(),
            ExprKind::MethodCall(_, receiver, args, _) => {
                std::iter::once(self.expression(receiver))
                    .chain(args.iter().map(|e| self.expression(e)))
                    .collect()
            }
            ExprKind::Call(_, args) => args.iter().map(|e| self.expression(e)).collect(),
            ExprKind::Field(base, _) if self.profile.supports("rust.verified-helper-calls") => {
                vec![self.expression(base)]
            }
            _ => Vec::new(),
        }
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
            StmtKind::Item(id) => self.node(stmt.hir_id, self.tcx.hir_item(id).span, "stmt_item"),
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

    fn dispatch(&self, expr: &Expr<'tcx>) -> Option<Dispatch> {
        let (op, operands) = match expr.kind {
            ExprKind::Binary(op, left, right) => (op.node.as_str().to_string(), vec![left, right]),
            ExprKind::AssignOp(op, left, right) => {
                (op.node.as_str().to_string(), vec![left, right])
            }
            ExprKind::Unary(op, value) => (op.as_str().to_string(), vec![value]),
            ExprKind::Cast(value, _) => ("as".to_string(), vec![value]),
            ExprKind::Call(callee, args) => (
                self.direct_callee(callee).and_then(|res| {
                    serde_json::from_str::<serde_json::Value>(&res.description).ok()
                })?["path"]
                    .as_str()?
                    .to_string(),
                args.iter().collect(),
            ),
            ExprKind::MethodCall(_, receiver, args, _) => (
                self.tcx
                    .def_path_str(self.typeck.type_dependent_def_id(expr.hir_id)?),
                std::iter::once(receiver).chain(args.iter()).collect(),
            ),
            _ => return None,
        };
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
    fn direct_callee(&self, expression: &Expr<'_>) -> Option<PathRes> {
        if !self.profile.supports("rust.verified-helper-calls") {
            return None;
        }
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };
        let Res::Def(rustc_hir::def::DefKind::Fn, callee) =
            self.typeck.qpath_res(path, expression.hir_id)
        else {
            return None;
        };
        if self.tcx.generics_of(callee).count() != 0 {
            return None;
        }

        Some(PathRes {
            kind: "resolved_direct_callee".into(),
            target: None,
            description: serde_json::to_string(&DirectCallee {
                path: self.tcx.def_path_str(callee),
                crate_name: self.tcx.crate_name(callee.krate).to_string(),
                local: callee.is_local(),
                definition_identity: format!("{:?}", self.tcx.def_path_hash(callee)),
            })
            .expect("resolved callee record"),
        })
    }
}

#[derive(serde::Serialize)]
struct DirectCallee {
    path: String,
    #[serde(rename = "crate")]
    crate_name: String,
    local: bool,
    definition_identity: String,
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
        _ => "expr_unsupported",
    }
}

pub fn bindings(node: &HirNode, scope: &str) -> Vec<BindingFact> {
    let local_scope = (node.kind == "expr_block")
        .then(|| format!("scope:source:block:{}:{}", node.span.start, node.span.end));
    let scope = local_scope.as_deref().unwrap_or(scope);
    let binding = node
        .binding
        .as_ref()
        .zip(node.ty.as_ref())
        .map(|(symbol, ty)| BindingFact {
            symbol: symbol.clone(),
            name: node.snippet.clone(),
            decl_span: node.span,
            scope: scope.to_string(),
            hir: node.hir.clone(),
            is_param: node.param_index.is_some(),
            is_mut: node.binding_mut.unwrap_or(false),
            ty: ty.clone(),
        });

    binding
        .into_iter()
        .chain(
            node.children
                .iter()
                .flat_map(|child| bindings(child, scope)),
        )
        .collect()
}
