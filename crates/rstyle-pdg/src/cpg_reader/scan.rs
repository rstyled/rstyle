//! The frontend's source scan before macro expansion and cfg removal, derived from
//! a document's source text: which constructs of a function's own source the
//! extraction profile defers, and whether the source is conditional.
//!
//! Only the compilation's root file is parsed before expansion, so only its functions
//! are scanned; each is found by its signature-to-body span, as the frontend finds it.

use crate::extraction_profile::ExtractionProfile;
use crate::source_evidence::positions::Positions;
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// One deferred construct, at file byte offsets.
#[derive(Clone)]
pub(super) struct Finding {
    pub span: (usize, usize),
    pub capability: &'static str,
    pub construct: &'static str,
}

/// One function the scan found, at file byte offsets.
pub(super) struct Function {
    pub span: (usize, usize),
    pub body: (usize, usize),
    pub findings: Vec<Finding>,
    pub conditional: bool,
}

/// The scanned functions of a file, or none when it does not parse.
pub(super) fn scan(source: &str, profile: ExtractionProfile) -> Vec<Function> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut scan = Scan {
        source,
        positions: Positions::new(source),
        profile,
        functions: Vec::new(),
    };
    scan.visit_file(&file);
    scan.functions
}

struct Scan<'s> {
    source: &'s str,
    positions: Positions,
    profile: ExtractionProfile,
    functions: Vec<Function>,
}

impl Scan<'_> {
    fn offsets(&self, span: Span) -> (usize, usize) {
        offsets(&self.positions, span)
    }

    /// A function from its visibility or first qualifier to its body, without attributes.
    fn function(&mut self, start: Span, body: &syn::Block) {
        let span = (self.offsets(start).0, self.offsets(body.span()).1);
        let mut owned = Owned {
            positions: &self.positions,
            source: self.source,
            profile: self.profile,
            findings: Vec::new(),
            conditional: false,
            match_pattern: false,
            literal_message: None,
        };
        owned.visit_block(body);
        self.functions.push(Function {
            span,
            body: self.offsets(body.span()),
            findings: owned.findings,
            conditional: owned.conditional,
        });
    }
}

/// The first byte of `source[start..end]` after outer attributes and doc comments,
/// which a construct's own span excludes.
fn after_attributes(source: &str, mut start: usize, end: usize) -> usize {
    let bytes = source.as_bytes();
    loop {
        let rest = source.get(start..end).unwrap_or_default();
        let trimmed = rest.trim_start();
        let at = start + (rest.len() - trimmed.len());
        let next = if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
            closing_bracket(bytes, at + trimmed.find('[').unwrap_or(0)).map(|close| close + 1)
        } else if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            trimmed.find('\n').map(|line| at + line + 1)
        } else if trimmed.starts_with("/**") || trimmed.starts_with("/*!") {
            trimmed.find("*/").map(|close| at + close + 2)
        } else {
            None
        };
        match next {
            Some(next) if next < end => start = next,
            _ => return at,
        }
    }
}

/// The `]` closing the `[` at `open`, outside string literals.
fn closing_bracket(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut text = false;
    let mut index = open;
    while index < bytes.len() {
        match (bytes[index], text) {
            (b'\\', true) => index += 1,
            (b'"', _) => text = !text,
            (b'[', false) => depth += 1,
            (b']', false) => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn offsets(positions: &Positions, span: Span) -> (usize, usize) {
    (
        positions.offset(span.start()) as usize,
        positions.offset(span.end()) as usize,
    )
}

/// Where an item's own span begins: its visibility, else its signature.
fn item_start(visibility: &syn::Visibility, signature: &syn::Signature) -> Span {
    match visibility {
        syn::Visibility::Inherited => signature.span(),
        visibility => visibility.span(),
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(item_start(&item.vis, &item.sig), &item.block);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let start = match (&item.vis, &item.defaultness) {
            (syn::Visibility::Inherited, Some(default)) => default.span(),
            (visibility, _) => item_start(visibility, &item.sig),
        };
        self.function(start, &item.block);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(body) = &item.default {
            self.function(item.sig.span(), body);
        }
    }
}

struct Owned<'p> {
    positions: &'p Positions,
    source: &'p str,
    profile: ExtractionProfile,
    findings: Vec<Finding>,
    conditional: bool,
    match_pattern: bool,
    literal_message: Option<(usize, usize)>,
}

impl Owned<'_> {
    /// A finding at a construct's own span, which excludes its outer attributes.
    fn reject(&mut self, span: Span, capability: &'static str, construct: &'static str) {
        let (start, end) = offsets(self.positions, span);
        self.findings.push(Finding {
            span: (after_attributes(self.source, start, end), end),
            capability,
            construct,
        });
    }

    fn supports(&self, capability: &str) -> bool {
        self.profile.supports(capability)
    }
}

impl<'ast> Visit<'ast> for Owned<'_> {
    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        let path = attribute.path();
        if path.is_ident("cfg") || path.is_ident("cfg_attr") {
            self.conditional = true;
        }
    }

    fn visit_type(&mut self, ty: &'ast syn::Type) {
        let permitted = match ty {
            syn::Type::Path(_) | syn::Type::Infer(_) => true,
            syn::Type::Tuple(tuple) if tuple.elems.is_empty() => true,
            syn::Type::Tuple(_)
            | syn::Type::Reference(_)
            | syn::Type::Array(_)
            | syn::Type::Slice(_) => self.supports("rust.concrete-read-only-data"),
            _ => false,
        };
        if !permitted {
            self.reject(
                ty.span(),
                "rust.additional-types",
                "unsupported_type_annotation",
            );
        }
        visit::visit_type(self, ty);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if self.special_local(local) {
            return;
        }
        let initialized = local
            .init
            .as_ref()
            .is_some_and(|init| init.diverge.is_none());
        if !initialized {
            self.reject(local.span(), "rust.local-bindings", "declaration_shape");
        }

        let previous = std::mem::replace(&mut self.match_pattern, false);
        for attribute in &local.attrs {
            self.visit_attribute(attribute);
        }
        self.visit_pat(&local.pat);
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, otherwise)) = &init.diverge {
                self.visit_expr(otherwise);
            }
        }
        self.match_pattern = previous;
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let previous = self.match_pattern;
        self.match_pattern = self.supports("rust.scalar-match");
        visit::visit_arm(self, arm);
        self.match_pattern = previous;
    }

    fn visit_pat(&mut self, pattern: &'ast syn::Pat) {
        // A typed pattern is its pattern, then its type, as the compiler walks them.
        if let syn::Pat::Type(typed) = pattern {
            self.visit_pat(&typed.pat);
            self.visit_type(&typed.ty);
            return;
        }
        if self.supports("rust.byte-iterator-position")
            && let syn::Pat::Reference(reference) = pattern
            && reference.mutability.is_none()
            && plain_binding(&reference.pat)
        {
            return;
        }
        if self.match_pattern && match_pattern(pattern, self.profile) {
            return;
        }
        if !matches!(pattern, syn::Pat::Ident(ident) if ident.by_ref.is_none() && ident.subpat.is_none())
        {
            self.reject(pattern.span(), "rust.scalar-interface", "binding_pattern");
        }
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        match statement {
            syn::Stmt::Item(syn::Item::Const(_))
                if self.supports("rust.native-borrowed-byte-readers") => {}
            syn::Stmt::Item(syn::Item::Fn(_))
                if self.supports(crate::bool_constants::CAPABILITY) => {}
            syn::Stmt::Item(item) => self.reject(item.span(), "rust.blocks", "nested_item"),
            syn::Stmt::Macro(mac) if selected_assertion(&mac.mac, self.profile) => {}
            syn::Stmt::Macro(_) => {
                self.reject(statement.span(), "rust.macros", "macro_invocation");
            }
            _ => visit::visit_stmt(self, statement),
        }
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if self.conditional_let(expression) {
            return;
        }
        let previous = self.literal_message;
        if let Some(span) = lossless_message(expression, self.profile)
            .or_else(|| validator_message(expression, self.profile))
        {
            self.literal_message = Some(offsets(self.positions, span));
        }
        self.expression(expression);
        self.literal_message = previous;
    }
}

impl Owned<'_> {
    fn special_local(&mut self, local: &syn::Local) -> bool {
        let Some(init) = local.init.as_ref().filter(|init| init.diverge.is_none()) else {
            return false;
        };
        if self.supports("rust.scalar-enum-values") && matches!(local.pat, syn::Pat::Wild(_)) {
            self.visit_expr(&init.expr);
            return true;
        }
        if self.supports("rust.named-error-validator-flow")
            && let syn::Pat::Tuple(tuple) = &local.pat
            && !tuple.elems.is_empty()
            && tuple.elems.len() <= crate::concrete_types::MAX_FIELDS
            && tuple.elems.iter().all(|child| {
                matches!(child, syn::Pat::Ident(ident)
                    if ident.by_ref.is_none() && ident.mutability.is_none() && ident.subpat.is_none())
            })
        {
            for child in &tuple.elems {
                self.visit_pat(child);
            }
            self.visit_expr(&init.expr);
            return true;
        }
        false
    }

    fn expression(&mut self, expression: &syn::Expr) {
        let span = expression.span();
        if let Some((capability, construct)) = deferred(expression)
            && !selected_expression(expression, self.profile)
            && self.literal_message != Some(offsets(self.positions, span))
        {
            self.reject(span, capability, construct);
            return;
        }
        // An unsafe block is the block it marks, as the compiler records it.
        if let syn::Expr::Unsafe(unsafe_block) = expression {
            self.reject(span, "rust.unsafe", "unsafe_block");
            for attribute in &unsafe_block.attrs {
                self.visit_attribute(attribute);
            }
            visit::visit_block(self, &unsafe_block.block);
            return;
        }
        visit::visit_expr(self, expression);
    }

    fn conditional_let(&mut self, expression: &syn::Expr) -> bool {
        if !self.supports("rust.scalar-option-if-let") {
            return false;
        }
        let syn::Expr::Let(condition) = expression else {
            return false;
        };
        let previous = std::mem::replace(&mut self.match_pattern, true);
        self.visit_pat(&condition.pat);
        self.match_pattern = previous;
        self.visit_expr(&condition.expr);
        true
    }
}

fn plain_binding(pattern: &syn::Pat) -> bool {
    matches!(pattern, syn::Pat::Ident(ident)
        if ident.by_ref.is_none() && ident.mutability.is_none() && ident.subpat.is_none())
}

/// The kind of literal the compiler's lexer records: an integer with a float suffix,
/// such as `1f64`, is an integer literal.
enum Literal {
    Bool,
    Integer,
    Float,
    Char,
    Byte,
    Text,
    Other,
}

fn literal(lit: &syn::Lit) -> (Literal, String) {
    match lit {
        syn::Lit::Bool(_) => (Literal::Bool, String::new()),
        syn::Lit::Int(int) => (Literal::Integer, int.suffix().to_owned()),
        syn::Lit::Float(float) => {
            let digits = float.base10_digits();
            let integer = !digits.contains(['.', 'e', 'E']);
            let kind = if integer {
                Literal::Integer
            } else {
                Literal::Float
            };
            (kind, float.suffix().to_owned())
        }
        syn::Lit::Char(char) => (Literal::Char, char.suffix().to_owned()),
        syn::Lit::Byte(byte) => (Literal::Byte, byte.suffix().to_owned()),
        syn::Lit::Str(text) => (Literal::Text, text.suffix().to_owned()),
        _ => (Literal::Other, String::new()),
    }
}

fn supported_literal(lit: &syn::Lit) -> bool {
    let (kind, suffix) = literal(lit);
    match kind {
        Literal::Bool => suffix.is_empty(),
        Literal::Integer => matches!(
            suffix.as_str(),
            "" | "i8"
                | "i16"
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
        ),
        _ => false,
    }
}

fn deferred(expression: &syn::Expr) -> Option<(&'static str, &'static str)> {
    use syn::Expr::*;
    match expression {
        Macro(_) => Some(("rust.macros", "macro_invocation")),
        Call(_) | MethodCall(_) => Some(("rust.call-summaries", "call")),
        Closure(_) => Some(("rust.closures", "closure")),
        Async(_) | Await(_) | Yield(_) => Some(("rust.async", "async_construct")),
        ForLoop(_) => Some(("rust.for-iteration", "for_loop")),
        Loop(_) | While(_) | Break(_) | Continue(_) => Some(("rust.loops", "loop_transfer")),
        Match(_) => Some(("rust.match", "match")),
        Reference(_) | RawAddr(_) | Field(_) | Index(_) => {
            Some(("rust.abstract-memory", "memory_operation"))
        }
        Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            Some(("rust.abstract-memory", "deref"))
        }
        Try(_) | TryBlock(_) => Some(("rust.unwinding-cleanup", "try_construct")),
        Cast(_) | Const(_) | Array(_) | Repeat(_) | Struct(_) | Range(_) => {
            Some(("rust.additional-types", "deferred_expression"))
        }
        Lit(lit) if !supported_literal(&lit.lit) => {
            Some(("rust.additional-types", "unsupported_literal"))
        }
        Lit(_) | Path(_) | Paren(_) | Binary(_) | Unary(_) | Block(_) | Unsafe(_) | If(_)
        | Assign(_) | Return(_) => None,
        Tuple(tuple) if tuple.elems.is_empty() => None,
        _ => Some(("rust.additional-types", "unlisted_source_expression")),
    }
}

fn selected_expression(expression: &syn::Expr, profile: ExtractionProfile) -> bool {
    use syn::Expr::*;
    let supports = |capability: &str| profile.supports(capability);
    match expression {
        Macro(mac) => selected_assertion(&mac.mac, profile),
        Lit(lit) => match literal(&lit.lit) {
            (Literal::Char, suffix) => {
                supports(crate::character_scalars::CAPABILITY) && suffix.is_empty()
            }
            (Literal::Float, suffix) => {
                supports("rust.float-kernel-scalars")
                    && matches!(suffix.as_str(), "" | "f32" | "f64")
            }
            (Literal::Byte, suffix) => {
                supports(crate::byte_literals::CAPABILITY) && suffix.is_empty()
            }
            _ => false,
        },
        Struct(_) => supports("rust.named-struct-values"),
        Repeat(_) => supports("rust.repeat-array-values"),
        Tuple(_) | Array(_) => supports("rust.aggregate-literal-values"),
        Field(_) | Reference(_) | RawAddr(_) => supports("rust.concrete-read-only-data"),
        Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            supports("rust.shared-dereference-reads")
        }
        Range(_) => supports("rust.shared-subslice-views"),
        Index(_) => supports("rust.scalar-index-reads"),
        Call(_) => supports("rust.verified-helper-calls"),
        MethodCall(_) => supports("rust.verified-primitive-summaries"),
        Closure(_) => supports("rust.immediate-option-callbacks"),
        Let(_) => supports("rust.scalar-option-if-let"),
        Try(_) => {
            supports("rust.same-error-result-propagation")
                || supports("rust.concrete-option-propagation")
        }
        While(_) | Loop(_) | Break(_) | Continue(_) => supports("rust.scalar-loops"),
        Cast(_) => supports("rust.integer-casts"),
        Match(_) => supports("rust.scalar-match"),
        _ => false,
    }
}

fn selected_assertion(mac: &syn::Macro, profile: ExtractionProfile) -> bool {
    let path: Vec<String> = mac
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    profile.supports("rust.scalar-assertions")
        && crate::assertions::macro_name(&path.join("::")).is_some()
}

/// A string literal argument the frontend keeps as an error message candidate.
fn text_literal(expression: &syn::Expr) -> bool {
    matches!(expression, syn::Expr::Lit(lit)
        if matches!(&lit.lit, syn::Lit::Str(text) if text.suffix().is_empty()))
}

fn validator_message(expression: &syn::Expr, profile: ExtractionProfile) -> Option<Span> {
    if !profile.supports("rust.named-error-validator-flow") {
        return None;
    }
    let syn::Expr::Try(attempt) = expression else {
        return None;
    };
    let syn::Expr::Call(call) = &*attempt.expr else {
        return None;
    };
    let [_, message] = call.args.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    // The compiler's string literal kinds, raw or not, regardless of a suffix.
    matches!(message, syn::Expr::Lit(lit) if matches!(lit.lit, syn::Lit::Str(_)))
        .then(|| message.span())
}

fn lossless_message(expression: &syn::Expr, profile: ExtractionProfile) -> Option<Span> {
    if !profile.supports("rust.lossless-usize-u64-calls") {
        return None;
    }
    let syn::Expr::MethodCall(method) = expression else {
        return None;
    };
    if method.method != "expect" || method.turbofish.is_some() {
        return None;
    }
    let [message] = method.args.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    if !text_literal(message) {
        return None;
    }
    let syn::Expr::Call(call) = &*method.receiver else {
        return None;
    };
    let syn::Expr::Path(path) = &*call.func else {
        return None;
    };
    if path.qself.is_some() || call.args.len() != 1 {
        return None;
    }
    let segments = &path.path.segments;
    if segments.iter().any(|segment| !segment.arguments.is_none()) {
        return None;
    }
    let names: Vec<String> = segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    ["u64", "usize"]
        .into_iter()
        .any(|target| {
            (target == "u64" || profile.supports("rust.lossless-u32-usize-calls"))
                && (names == [target, "try_from"]
                    || names == ["core", "primitive", target, "try_from"]
                    || names == ["std", "primitive", target, "try_from"])
        })
        .then(|| message.span())
}

fn match_pattern(pattern: &syn::Pat, profile: ExtractionProfile) -> bool {
    let supports = |capability: &str| profile.supports(capability);
    if supports("rust.scalar-enum-values")
        && let syn::Pat::TupleStruct(tuple) = pattern
        && matches!(tuple.elems.iter().collect::<Vec<_>>()[..],
            [child] if matches!(child, syn::Pat::Wild(_)) || plain_binding(child))
    {
        return true;
    }
    if supports("rust.checked-slice-option")
        && let syn::Pat::TupleStruct(tuple) = pattern
    {
        let children: Vec<&syn::Pat> = tuple.elems.iter().collect();
        if supports("rust.scalar-option-values")
            && matches!(children[..], [child] if plain_binding(child))
        {
            return true;
        }
        return matches!(children[..], [syn::Pat::Reference(reference)]
            if reference.mutability.is_none() && plain_binding(&reference.pat));
    }
    match pattern {
        syn::Pat::Or(or) if supports("rust.scalar-or-patterns") => or.cases.iter().all(|case| {
            if supports("rust.nested-scalar-or-patterns") {
                match_pattern(case, profile)
            } else {
                match_leaf(case, profile)
            }
        }),
        syn::Pat::Paren(paren) if supports("rust.nested-scalar-or-patterns") => {
            match_pattern(&paren.pat, profile)
        }
        _ => match_leaf(pattern, profile),
    }
}

fn match_leaf(pattern: &syn::Pat, profile: ExtractionProfile) -> bool {
    let supports = |capability: &str| profile.supports(capability);
    match pattern {
        syn::Pat::Wild(_) => true,
        syn::Pat::Path(_) => supports("rust.scalar-constant-patterns"),
        syn::Pat::Range(range) if supports("rust.scalar-range-patterns") => {
            range.start.iter().chain(&range.end).all(|bound| {
                pattern_literal(bound, profile)
                    || (supports("rust.scalar-constant-range-bounds")
                        && matches!(**bound, syn::Expr::Path(_)))
            })
        }
        syn::Pat::Lit(lit) => pattern_literal(&syn::Expr::Lit(lit.clone()), profile),
        syn::Pat::Ident(_) if plain_binding(pattern) => supports("rust.scalar-constant-patterns"),
        _ => false,
    }
}

fn pattern_literal(expression: &syn::Expr, profile: ExtractionProfile) -> bool {
    match expression {
        syn::Expr::Lit(lit) => {
            supported_literal(&lit.lit) || selected_expression(expression, profile)
        }
        syn::Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            matches!(&*unary.expr, syn::Expr::Lit(lit) if supported_literal(&lit.lit))
        }
        _ => false,
    }
}
