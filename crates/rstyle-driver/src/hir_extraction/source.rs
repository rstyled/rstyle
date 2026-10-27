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
    pub body_span: Span,
    pub span: Span,
    pub findings: Vec<Finding>,
    pub conditional_source: bool,
}

#[derive(Default)]
pub struct Scan {
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
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
            profile: self.profile,
            match_pattern: false,
            literal_message: None,
        };
        owned.visit_block(body);

        self.functions.push(Function {
            body_span: body.span,
            span,
            findings: owned.findings,
            conditional_source: owned.conditional_source,
        });
    }
}

struct Owned {
    literal_message: Option<Span>,
    match_pattern: bool,
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
            ast::TyKind::Tup(_)
            | ast::TyKind::Ref(..)
            | ast::TyKind::Array(..)
            | ast::TyKind::Slice(..)
                if self.profile.supports("rust.concrete-read-only-data") => {}
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
        if self.visit_special_local(local) {
            return;
        }

        if !matches!(local.kind, ast::LocalKind::Init(_)) {
            self.reject(local.span, "rust.local-bindings", "declaration_shape");
        }

        self.walk_binding_local(local);
    }

    fn visit_arm(&mut self, arm: &'a ast::Arm) {
        let previous = self.match_pattern;
        self.match_pattern = self.profile.supports("rust.scalar-match");
        visit::walk_arm(self, arm);
        self.match_pattern = previous;
    }

    fn visit_pat(&mut self, pattern: &'a ast::Pat) {
        if self.profile.supports("rust.byte-iterator-position")
            && let ast::PatKind::Ref(binding, ast::Pinnedness::Not, ast::Mutability::Not) =
                &pattern.kind
            && matches!(
                binding.kind,
                ast::PatKind::Ident(
                    ast::BindingMode(ast::ByRef::No, ast::Mutability::Not),
                    _,
                    None
                )
            )
        {
            return;
        }
        if self.match_pattern && source_match_pattern(&pattern.kind, self.profile) {
            return;
        }
        if !matches!(
            pattern.kind,
            ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, _), _, None)
        ) {
            self.reject(pattern.span, "rust.scalar-interface", "binding_pattern");
        }
    }
    fn visit_stmt(&mut self, statement: &'a ast::Stmt) {
        match &statement.kind {
            ast::StmtKind::Item(item)
                if self.profile.supports("rust.native-borrowed-byte-readers")
                    && matches!(item.kind, ast::ItemKind::Const(..)) => {}
            ast::StmtKind::Item(item)
                if self
                    .profile
                    .supports(rstyle_pdg::bool_constants::CAPABILITY)
                    && matches!(item.kind, ast::ItemKind::Fn(..)) => {}
            ast::StmtKind::Item(item) => self.reject(item.span, "rust.blocks", "nested_item"),
            ast::StmtKind::MacCall(mac) if selected_assertion(&mac.mac, self.profile) => {}
            ast::StmtKind::MacCall(_) => {
                self.reject(statement.span, "rust.macros", "macro_invocation")
            }
            _ => visit::walk_stmt(self, statement),
        }
    }

    fn visit_expr(&mut self, expression: &'a ast::Expr) {
        if self.conditional_let(expression) {
            return;
        }

        self.with_literal_message(expression);
    }
}

impl Owned {
    fn visit_special_local(&mut self, local: &ast::Local) -> bool {
        if self.profile.supports("rust.scalar-enum-values")
            && matches!(local.pat.kind, ast::PatKind::Wild)
            && local.ty.is_none()
            && let ast::LocalKind::Init(value) = &local.kind
        {
            self.visit_expr(value);
            return true;
        }

        if self.profile.supports("rust.named-error-validator-flow")
            && local.ty.is_none()
            && let ast::PatKind::Tuple(children) = &local.pat.kind
            && !children.is_empty()
            && children.len() <= rstyle_pdg::concrete_types::MAX_FIELDS
            && children.iter().all(|child| {
                matches!(
                    child.kind,
                    ast::PatKind::Ident(
                        ast::BindingMode(ast::ByRef::No, ast::Mutability::Not),
                        _,
                        None
                    )
                )
            })
            && let ast::LocalKind::Init(value) = &local.kind
        {
            for child in children {
                self.visit_pat(child);
            }
            self.visit_expr(value);
            return true;
        }

        false
    }

    fn with_literal_message(&mut self, expression: &ast::Expr) {
        let previous = self.literal_message;
        if let Some(span) = lossless_message(expression, self.profile)
            .or_else(|| validator_message(expression, self.profile))
        {
            self.literal_message = Some(span);
        }

        self.expression(expression);
        self.literal_message = previous;
    }

    fn expression(&mut self, expression: &ast::Expr) {
        if let Some((capability, construct)) = deferred(&expression.kind)
            && !selected_expression(&expression.kind, self.profile)
            && self.literal_message != Some(expression.span)
        {
            self.reject(expression.span, capability, construct);
            return;
        }

        visit::walk_expr(self, expression);
    }
}

/// Retain one candidate literal actual. Typed whole-source and escaping-origin
/// checks in the Data frontend remain mandatory; syntax grants no call authority.
fn validator_message(
    expression: &ast::Expr,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Option<Span> {
    if !profile.supports("rust.named-error-validator-flow") {
        return None;
    }
    let ast::ExprKind::Try(call) = &expression.kind else {
        return None;
    };

    let ast::ExprKind::Call(_, arguments) = &call.kind else {
        return None;
    };
    let [_, message] = arguments.as_slice() else {
        return None;
    };

    let ast::ExprKind::Lit(literal) = &message.kind else {
        return None;
    };
    matches!(
        literal.kind,
        ast::token::LitKind::Str | ast::token::LitKind::StrRaw(_)
    )
    .then_some(message.span)
}

/// Syntactic candidate only; independent HIR/core/source checks grant semantics.
fn lossless_message(
    expression: &ast::Expr,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Option<Span> {
    if !profile.supports("rust.lossless-usize-u64-calls") {
        return None;
    }

    let ast::ExprKind::MethodCall(method) = &expression.kind else {
        return None;
    };
    if method.seg.ident.name.as_str() != "expect" || method.seg.args.is_some() {
        return None;
    }

    let [message] = method.args.as_slice() else {
        return None;
    };

    let ast::ExprKind::Lit(literal) = &message.kind else {
        return None;
    };

    if !matches!(
        literal.kind,
        ast::token::LitKind::Str | ast::token::LitKind::StrRaw(_)
    ) || literal.suffix.is_some()
    {
        return None;
    }

    let ast::ExprKind::Call(callee, arguments) = &method.receiver.kind else {
        return None;
    };
    let ast::ExprKind::Path(None, path) = &callee.kind else {
        return None;
    };

    let names: Vec<_> = path
        .segments
        .iter()
        .map(|segment| segment.ident.name.as_str())
        .collect();
    (arguments.len() == 1
        && path.segments.iter().all(|segment| segment.args.is_none())
        && (["u64", "usize"].into_iter().any(|target| {
            (target == "u64" || profile.supports("rust.lossless-u32-usize-calls"))
                && (names == [target, "try_from"]
                    || names == ["core", "primitive", target, "try_from"]
                    || names == ["std", "primitive", target, "try_from"])
        })))
    .then_some(message.span)
}

impl Owned {
    fn conditional_let(&mut self, expression: &ast::Expr) -> bool {
        if !self.profile.supports("rust.scalar-option-if-let") {
            return false;
        }
        let ast::ExprKind::Let(pattern, initializer, ..) = &expression.kind else {
            return false;
        };

        self.conditional_pattern(pattern);
        self.visit_expr(initializer);
        true
    }

    fn conditional_pattern(&mut self, pattern: &ast::Pat) {
        let previous = self.match_pattern;
        self.match_pattern = true;
        self.visit_pat(pattern);
        self.match_pattern = previous;
    }
}

fn selected_expression(
    kind: &ast::ExprKind,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    match kind {
        ast::ExprKind::MacCall(mac) => selected_assertion(mac, profile),
        ast::ExprKind::Lit(literal) if literal.kind == ast::token::LitKind::Char => {
            profile.supports(rstyle_pdg::character_scalars::CAPABILITY) && literal.suffix.is_none()
        }
        ast::ExprKind::Lit(literal) if literal.kind == ast::token::LitKind::Float => {
            profile.supports("rust.float-kernel-scalars")
                && literal
                    .suffix
                    .is_none_or(|suffix| matches!(suffix.as_str(), "f32" | "f64"))
        }
        ast::ExprKind::Lit(literal) if literal.kind == ast::token::LitKind::Byte => {
            profile.supports(rstyle_pdg::byte_literals::CAPABILITY) && literal.suffix.is_none()
        }
        ast::ExprKind::Struct(..) => profile.supports("rust.named-struct-values"),
        ast::ExprKind::Repeat(..) => profile.supports("rust.repeat-array-values"),
        ast::ExprKind::Tup(..) | ast::ExprKind::Array(..) => {
            profile.supports("rust.aggregate-literal-values")
        }
        ast::ExprKind::Field(..) | ast::ExprKind::AddrOf(..) => {
            profile.supports("rust.concrete-read-only-data")
        }
        ast::ExprKind::Unary(ast::UnOp::Deref, _) => {
            profile.supports("rust.shared-dereference-reads")
        }
        ast::ExprKind::Range(..) => profile.supports("rust.shared-subslice-views"),
        ast::ExprKind::Index(..) => profile.supports("rust.scalar-index-reads"),
        ast::ExprKind::Call(..) => profile.supports("rust.verified-helper-calls"),
        ast::ExprKind::MethodCall(..) => profile.supports("rust.verified-primitive-summaries"),
        ast::ExprKind::Closure(_) => profile.supports("rust.immediate-option-callbacks"),
        ast::ExprKind::Let(..) => profile.supports("rust.scalar-option-if-let"),
        ast::ExprKind::Try(..) => {
            profile.supports("rust.same-error-result-propagation")
                || profile.supports("rust.concrete-option-propagation")
        }
        ast::ExprKind::While(..)
        | ast::ExprKind::Loop(..)
        | ast::ExprKind::Break(..)
        | ast::ExprKind::Continue(..) => profile.supports("rust.scalar-loops"),
        ast::ExprKind::Cast(..) => profile.supports("rust.integer-casts"),
        ast::ExprKind::Match(..) => profile.supports("rust.scalar-match"),
        _ => false,
    }
}

fn selected_assertion(
    mac: &ast::MacCall,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    let path = mac
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.name.to_string())
        .collect::<Vec<_>>()
        .join("::");
    profile.supports("rust.scalar-assertions")
        && rstyle_pdg::assertions::macro_name(&path).is_some()
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

pub fn scan(
    krate: &ast::Crate,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Vec<Function> {
    let mut scan = Scan {
        profile,
        functions: Vec::new(),
    };
    visit::walk_crate(&mut scan, krate);

    scan.functions
}

fn source_match_pattern(
    kind: &ast::PatKind,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    if profile.supports("rust.scalar-enum-values")
        && let ast::PatKind::TupleStruct(_, _, children) = kind
        && matches!(children.as_slice(), [child] if matches!(child.kind,
            ast::PatKind::Wild | ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, ast::Mutability::Not), _, None)))
    {
        return true;
    }
    if profile.supports("rust.checked-slice-option")
        && let ast::PatKind::TupleStruct(_, _, children) = kind
    {
        if profile.supports("rust.scalar-option-values")
            && matches!(children.as_slice(), [child] if matches!(child.kind,
                ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, ast::Mutability::Not), _, None)))
        {
            return true;
        }
        return matches!(children.as_slice(), [child] if matches!(&child.kind,
            ast::PatKind::Ref(binding, ast::Pinnedness::Not, ast::Mutability::Not)
                if matches!(binding.kind, ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, ast::Mutability::Not), _, None))));
    }
    match kind {
        ast::PatKind::Or(patterns) if profile.supports("rust.scalar-or-patterns") => {
            patterns.iter().all(|pattern| {
                if profile.supports("rust.nested-scalar-or-patterns") {
                    source_match_pattern(&pattern.kind, profile)
                } else {
                    source_match_leaf(&pattern.kind, profile)
                }
            })
        }
        ast::PatKind::Paren(pattern) if profile.supports("rust.nested-scalar-or-patterns") => {
            source_match_pattern(&pattern.kind, profile)
        }
        _ => source_match_leaf(kind, profile),
    }
}

fn source_match_leaf(
    kind: &ast::PatKind,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    match kind {
        ast::PatKind::Wild => true,
        ast::PatKind::Path(..) => profile.supports("rust.scalar-constant-patterns"),
        ast::PatKind::Range(start, end, _) if profile.supports("rust.scalar-range-patterns") => {
            start.iter().chain(end).all(|value| {
                source_pattern_literal(value, profile)
                    || (profile.supports("rust.scalar-constant-range-bounds")
                        && matches!(value.kind, ast::ExprKind::Path(..)))
            })
        }
        ast::PatKind::Expr(expression) => {
            source_pattern_literal(expression, profile)
                || (profile.supports("rust.scalar-constant-patterns")
                    && matches!(expression.kind, ast::ExprKind::Path(..)))
        }
        ast::PatKind::Ident(ast::BindingMode(ast::ByRef::No, ast::Mutability::Not), _, None) => {
            profile.supports("rust.scalar-constant-patterns")
        }
        _ => false,
    }
}

fn source_pattern_literal(
    expression: &ast::Expr,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> bool {
    match &expression.kind {
        ast::ExprKind::Lit(literal) => {
            supported_literal(literal) || selected_expression(&expression.kind, profile)
        }
        ast::ExprKind::Unary(ast::UnOp::Neg, value) => {
            matches!(&value.kind, ast::ExprKind::Lit(literal) if supported_literal(literal))
        }
        _ => false,
    }
}

impl Owned {
    fn walk_binding_local(&mut self, local: &ast::Local) {
        let previous = std::mem::replace(&mut self.match_pattern, false);
        visit::walk_local(self, local);
        self.match_pattern = previous;
    }
}
