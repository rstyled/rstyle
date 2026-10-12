//! Owned source scan before macro expansion and cfg removal.

use rustc_ast::{
    self as ast,
    visit::{self, FnKind, Visitor},
};
use rustc_span::Span;

#[derive(Clone)]
pub struct Finding {
    pub span: Span,
    pub capability: &'static str,
    pub construct: &'static str,
}

pub struct Function {
    pub span: Span,
    pub findings: Vec<Finding>,
    pub conditional_source: bool,
}

#[derive(Default)]
pub struct Scan {
    pub functions: Vec<Function>,
}

impl<'a> Visitor<'a> for Scan {
    fn visit_fn(&mut self, kind: FnKind<'a>, _: &ast::AttrVec, span: Span, _: ast::NodeId) {
        let FnKind::Fn(_, _, function) = kind else {
            return;
        };
        let Some(body) = &function.body else { return };
        let mut owned = Owned {
            findings: Vec::new(),
            conditional_source: false,
            profile: selected_profile(),
        };
        owned.visit_block(body);

        self.functions.push(Function {
            span,
            findings: owned.findings,
            conditional_source: owned.conditional_source,
        });
    }
}

struct Owned {
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    findings: Vec<Finding>,
    conditional_source: bool,
}

impl Owned {
    fn reject(&mut self, span: Span, capability: &'static str, construct: &'static str) {
        self.findings.push(Finding {
            span,
            capability,
            construct,
        });
    }
}

impl<'a> Visitor<'a> for Owned {
    fn visit_attribute(&mut self, attribute: &'a ast::Attribute) {
        if attribute.has_name(rustc_span::sym::cfg) || attribute.has_name(rustc_span::sym::cfg_attr)
        {
            self.conditional_source = true;
        }
    }

    fn visit_ty(&mut self, ty: &'a ast::Ty) {
        match &ty.kind {
            ast::TyKind::Path(..) | ast::TyKind::Infer => {}
            ast::TyKind::Tup(types) if types.is_empty() => {}
            _ => self.reject(
                ty.span,
                "rust.additional-types",
                "unsupported_type_annotation",
            ),
        }

        visit::walk_ty(self, ty);
    }

    fn visit_block(&mut self, block: &'a ast::Block) {
        if matches!(block.rules, ast::BlockCheckMode::Unsafe(_)) {
            self.reject(block.span, "rust.unsafe", "unsafe_block");
        }

        visit::walk_block(self, block);
    }

    fn visit_local(&mut self, local: &'a ast::Local) {
        if !matches!(local.kind, ast::LocalKind::Init(_)) {
            self.reject(local.span, "rust.local-bindings", "declaration_shape");
        }

        visit::walk_local(self, local);
    }

    fn visit_pat(&mut self, pattern: &'a ast::Pat) {
        if !matches!(
            pattern.kind,
            ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, _), _, None)
        ) {
            self.reject(pattern.span, "rust.scalar-interface", "binding_pattern");
        }
    }
    fn visit_stmt(&mut self, statement: &'a ast::Stmt) {
        match &statement.kind {
            ast::StmtKind::Item(item) => self.reject(item.span, "rust.blocks", "nested_item"),
            ast::StmtKind::MacCall(_) => {
                self.reject(statement.span, "rust.macros", "macro_invocation")
            }
            _ => visit::walk_stmt(self, statement),
        }
    }

    fn visit_expr(&mut self, expression: &'a ast::Expr) {
        if matches!(expression.kind, ast::ExprKind::Cast(..)) && self.profile.integer_casts() {
            visit::walk_expr(self, expression);
            return;
        }
        if let Some((capability, construct)) = deferred(&expression.kind) {
            self.reject(expression.span, capability, construct);
            return;
        }

        visit::walk_expr(self, expression);
    }
}

fn deferred(kind: &ast::ExprKind) -> Option<(&'static str, &'static str)> {
    use ast::ExprKind::*;

    match kind {
        MacCall(_) => Some(("rust.macros", "macro_invocation")),
        Call(..) | MethodCall(..) => Some(("rust.call-summaries", "call")),
        Closure(_) => Some(("rust.closures", "closure")),
        Gen(..) | Await(..) | Yield(..) => Some(("rust.async", "async_construct")),
        ForLoop(_) => Some(("rust.for-iteration", "for_loop")),
        Loop(..) | While(..) | Break(..) | Continue(..) => Some(("rust.loops", "loop_transfer")),
        Match(..) => Some(("rust.match", "match")),
        AddrOf(..) | Field(..) | Index(..) => Some(("rust.abstract-memory", "memory_operation")),
        Unary(ast::UnOp::Deref, _) => Some(("rust.abstract-memory", "deref")),
        Try(..) | TryBlock(..) | Yeet(..) => Some(("rust.unwinding-cleanup", "try_construct")),
        Cast(..) | ConstBlock(..) | Array(..) | Repeat(..) | Struct(..) | Range(..) => {
            Some(("rust.additional-types", "deferred_expression"))
        }
        Lit(literal) if !supported_literal(literal) => {
            Some(("rust.additional-types", "unsupported_literal"))
        }
        Lit(_) | Path(..) | Paren(_) | Binary(..) | Unary(..) | Block(..) | If(..) | Assign(..)
        | AssignOp(..) | Ret(..) => None,
        Tup(values) if values.is_empty() => None,
        _ => Some(("rust.additional-types", "unlisted_source_expression")),
    }
}

fn supported_literal(literal: &ast::token::Lit) -> bool {
    use ast::token::LitKind;
    match literal.kind {
        LitKind::Bool => literal.suffix.is_none(),
        LitKind::Integer => literal.suffix.is_none_or(|suffix| {
            matches!(
                suffix.as_str(),
                "i8" | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
            )
        }),
        _ => false,
    }
}

pub fn scan(krate: &ast::Crate) -> Vec<Function> {
    let mut scan = Scan::default();
    visit::walk_crate(&mut scan, krate);

    scan.functions
}

fn selected_profile() -> rstyle_pdg::extraction_profile::ExtractionProfile {
    std::env::var("RSTYLE_HIR_EXTRACTION_PROFILE")
        .ok()
        .and_then(|value| rstyle_pdg::extraction_profile::ExtractionProfile::parse(&value).ok())
        .unwrap_or_default()
}
