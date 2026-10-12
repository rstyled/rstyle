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
    pub result: Ty<'tcx>,
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
        let base = self.node(expr.hir_id, expr.span, kind(&expr.kind));
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

    fn children(&self, expr: &Expr<'tcx>) -> Vec<HirNode> {
        match expr.kind {
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
            ExprKind::Ret(value) => value.into_iter().map(|e| self.expression(e)).collect(),
            ExprKind::Tup(values) => values.iter().map(|e| self.expression(e)).collect(),
            ExprKind::Call(_, args) => args.iter().map(|e| self.expression(e)).collect(),
            _ => Vec::new(),
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
            kind: if overloaded { "overloaded" } else { "builtin" }.to_string(),
            op,
            operand_tys,
            result_ty: resolved(self.typeck.expr_ty(expr), String::new())
                .normalized
                .unwrap_or_else(|| self.typeck.expr_ty(expr).to_string()),
        })
    }
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
