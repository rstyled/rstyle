//! Independent original-source inventory checks; graph extraction remains HIR-only.

#[cfg(test)]
mod tests;

use crate::{ByteSpan, HirNode, HirRef};
use serde::{Deserialize, Serialize};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

pub const KEY: &str = "source_correspondence";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub kind: String,
    pub span: ByteSpan,
    pub hir: Option<HirRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Correspondence {
    pub source_digest: String,
    pub cfgs: Vec<String>,
    pub features: Vec<String>,
    pub entries: Vec<Entry>,
}

struct Inventory<'a> {
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    entries: Vec<Entry>,
    configuration: Option<crate::conditional_source::Configuration<'a>>,
}

pub fn capture(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Correspondence, String> {
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes)
        .map_err(|error| format!("Original owned source inventory unavailable: {error}"))?;
    let mut inventory = Inventory {
        facts,
        entries: Vec::new(),
        configuration: crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.cfg-source-removal")
            .then(|| crate::conditional_source::Configuration::new(&facts.build.cfgs)),
    };
    inventory.visit_block(&function.block);

    Ok(Correspondence {
        source_digest: facts.source.digest.clone(),
        cfgs: facts.build.cfgs.clone(),
        features: facts.build.features.clone(),
        entries: inventory.entries,
    })
}

pub fn verify(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let value = facts
        .build
        .frontend_options
        .get(KEY)
        .ok_or("Missing original-source correspondence evidence")?;
    let actual: Correspondence = serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid source correspondence: {error}"))?;
    let expected = capture(facts)?;
    if actual != expected {
        return Err(
            "Original source inventory, resolved HIR correspondence or cfg/features disagree"
                .into(),
        );
    }
    if expected.entries.iter().any(|entry| {
        if entry.kind == "cfg_inactive" {
            entry.hir.is_some()
        } else {
            entry.hir.is_none()
        }
    }) {
        return Err("Removed or unmatched owned-source operations lack independent resolved HIR evidence; semantics remain unknown".into());
    }

    Ok(())
}

impl Inventory<'_> {
    fn inactive(&mut self, attributes: &[syn::Attribute], syntax: proc_macro2::Span) -> bool {
        if attributes.is_empty()
            || !self
                .configuration
                .as_ref()
                .is_some_and(|configuration| configuration.active(attributes) == Ok(false))
        {
            return false;
        }

        let span = ByteSpan {
            start: offset(&self.facts.source.bytes, syntax.start()),
            end: offset(&self.facts.source.bytes, syntax.end()),
        };
        let hir = contained_node(&self.facts.body, span);
        self.entries.push(Entry {
            kind: "cfg_inactive".into(),
            span,
            hir,
        });
        true
    }

    fn assertion(&mut self, mac: &syn::Macro) -> bool {
        if !crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
            .is_ok_and(|profile| profile.supports("rust.scalar-assertions"))
        {
            return false;
        }
        let Ok(assertion) = crate::assertions::Assertion::from_macro(mac) else {
            return false;
        };

        self.add("expr_assert", mac.span());
        self.visit_expr(&assertion.condition);
        true
    }

    fn expression(&mut self, expression: &syn::Expr) {
        let kind = self.expression_kind(expression);
        if let Some(kind) = kind {
            self.add(kind, expression.span());
        }

        if matches!(
            expression,
            syn::Expr::Closure(_) | syn::Expr::Async(_) | syn::Expr::Const(_)
        ) {
            return;
        }
        if let syn::Expr::Call(call) = expression
            && crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
                .is_ok_and(|profile| profile.supports("rust.verified-helper-calls"))
            && matches!(&*call.func, syn::Expr::Path(_))
        {
            for argument in &call.args {
                self.visit_expr(argument);
            }
            return;
        }

        visit::visit_expr(self, expression);
    }

    fn expression_kind(&self, expression: &syn::Expr) -> Option<&'static str> {
        if matches!(expression, syn::Expr::Field(_) | syn::Expr::Reference(_))
            && crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
                .is_ok_and(|profile| profile.supports("rust.concrete-read-only-data"))
        {
            return Some(if matches!(expression, syn::Expr::Reference(_)) {
                "expr_addr_of"
            } else {
                "expr_field"
            });
        }
        expression_kind(expression)
    }

    fn add(&mut self, kind: &str, span: proc_macro2::Span) -> bool {
        let source = &self.facts.source.bytes;
        let span = ByteSpan {
            start: offset(source, span.start()),
            end: offset(source, span.end()),
        };
        let candidates = self.find(&self.facts.body, kind, span);

        let hir = if candidates.len() == 1 {
            Some(candidates[0].clone())
        } else {
            None
        };
        let matched = hir.is_some();
        self.entries.push(Entry {
            kind: kind.into(),
            span,
            hir,
        });
        matched
    }
}

impl<'ast> Visit<'ast> for Inventory<'_> {
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if self.inactive(
            crate::conditional_source::expression_attributes(expression),
            expression.span(),
        ) {
            return;
        }
        if let syn::Expr::Macro(expression) = expression
            && self.assertion(&expression.mac)
        {
            return;
        }
        self.expression(expression);
    }

    fn visit_pat(&mut self, pattern: &'ast syn::Pat) {
        let profile = crate::extraction_profile::ExtractionProfile::from_facts(self.facts).ok();
        let constants =
            profile.is_some_and(|profile| profile.supports("rust.scalar-constant-patterns"));
        let nested =
            profile.is_some_and(|profile| profile.supports("rust.nested-scalar-or-patterns"));
        let kind = match pattern {
            syn::Pat::Lit(_) => Some("pat_literal"),
            syn::Pat::Wild(_) => Some("pat_wild"),
            syn::Pat::Path(_) if constants => Some("pat_constant"),
            syn::Pat::Ident(ident)
                if constants
                    && ident.by_ref.is_none()
                    && ident.mutability.is_none()
                    && ident.subpat.is_none() =>
            {
                Some("pat_name")
            }
            syn::Pat::Range(range) if crate::scalar_patterns::scalar_range(range, constants) => {
                Some("pat_range")
            }
            syn::Pat::Or(pattern) if (nested || scalar_alternatives(pattern, constants)) => {
                Some("pat_or")
            }
            _ => None,
        };
        let matched = kind.is_some_and(|kind| self.add(kind, pattern.span()));

        if let syn::Pat::Or(pattern) = pattern
            && ((nested && matched) || (!nested && scalar_alternatives(pattern, constants)))
        {
            for case in &pattern.cases {
                self.visit_pat(case);
            }
        } else if nested && let syn::Pat::Paren(pattern) = pattern {
            self.visit_pat(&pattern.pat);
        }
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if let syn::Stmt::Macro(statement) = statement
            && self.inactive(&statement.attrs, statement.span())
        {
            return;
        }
        self.statement(statement);
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !self.inactive(&arm.attrs, arm.span()) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if self.inactive(&local.attrs, local.span()) {
            return;
        }
        self.local(local);
    }

    fn visit_item(&mut self, item: &'ast syn::Item) {
        if !self.inactive(
            crate::conditional_source::item_attributes(item),
            item.span(),
        ) {
            self.add("stmt_item", item.span());
        }
    }
}

impl Inventory<'_> {
    fn statement(&mut self, statement: &syn::Stmt) {
        if let syn::Stmt::Macro(statement) = statement
            && self.assertion(&statement.mac)
        {
            return;
        }
        if matches!(statement, syn::Stmt::Macro(_)) {
            self.add("source_macro", statement.span());
            return;
        }
        visit::visit_stmt(self, statement);
    }

    fn local(&mut self, local: &syn::Local) {
        let span = local
            .let_token
            .span
            .join(local.semi_token.span)
            .unwrap_or(local.let_token.span);
        self.add("stmt_let", span);
        visit::visit_local(self, local);
    }
}

fn contained_node(node: &HirNode, span: ByteSpan) -> Option<HirRef> {
    if span.start <= node.span.start && node.span.end <= span.end {
        return Some(node.hir.clone());
    }
    node.children
        .iter()
        .find_map(|node| contained_node(node, span))
}

impl Correspondence {
    pub fn inactive(&self, span: ByteSpan) -> bool {
        self.entries.iter().any(|entry| {
            entry.kind == "cfg_inactive"
                && entry.hir.is_none()
                && entry.span.start <= span.start
                && span.end <= entry.span.end
        })
    }
}

fn offset(source: &str, position: proc_macro2::LineColumn) -> u32 {
    u32::try_from(byte_offset(source, position)).unwrap_or(u32::MAX)
}

pub(crate) fn byte_offset(source: &str, position: proc_macro2::LineColumn) -> usize {
    let prefix: usize = source
        .split_inclusive('\n')
        .take(position.line.saturating_sub(1))
        .map(str::len)
        .sum();
    let line = source.get(prefix..).unwrap_or_default();
    let column = line
        .char_indices()
        .nth(position.column)
        .map_or(line.len(), |(offset, _)| offset);

    prefix + column
}

impl Inventory<'_> {
    fn find(&self, node: &HirNode, kind: &str, span: ByteSpan) -> Vec<HirRef> {
        let matches_kind = node.kind == kind
            || (kind == "pat_name" && matches!(node.kind.as_str(), "pat_binding" | "pat_constant"));
        let own = (matches_kind && corresponding_span(&self.facts.source.bytes, span, node.span))
            .then(|| node.hir.clone());

        own.into_iter()
            .chain(
                node.children
                    .iter()
                    .flat_map(|child| self.find(child, kind, span)),
            )
            .collect()
    }
}

fn corresponding_span(source: &str, syntax: ByteSpan, hir: ByteSpan) -> bool {
    if syntax == hir {
        return true;
    }
    if hir.start > syntax.start || hir.end < syntax.end {
        return false;
    }
    let Some(before) = source.get(hir.start as usize..syntax.start as usize) else {
        return false;
    };
    let Some(after) = source.get(syntax.end as usize..hir.end as usize) else {
        return false;
    };

    (before.chars().all(|ch| ch == '(' || ch.is_whitespace())
        && after.chars().all(|ch| ch == ')' || ch.is_whitespace())
        && before.chars().filter(|ch| *ch == '(').count()
            == after.chars().filter(|ch| *ch == ')').count())
        || commented_parentheses(before, after)
}

fn commented_parentheses(before: &str, after: &str) -> bool {
    if ![before, after]
        .iter()
        .any(|part| part.contains("//") || part.contains("/*"))
    {
        return false;
    }

    let Ok(mut expression) = syn::parse_str::<syn::Expr>(&format!("{before}__rstyle_group{after}"))
    else {
        return false;
    };
    while let syn::Expr::Paren(paren) = expression {
        expression = *paren.expr;
    }
    matches!(expression, syn::Expr::Path(path) if path.path.is_ident("__rstyle_group"))
}

fn scalar_alternatives(pattern: &syn::PatOr, constants: bool) -> bool {
    pattern.cases.iter().all(|case| match case {
        syn::Pat::Lit(literal) => matches!(literal.lit, syn::Lit::Int(_) | syn::Lit::Bool(_)),
        syn::Pat::Range(range) => crate::scalar_patterns::scalar_range(range, constants),
        syn::Pat::Path(_) => constants,
        syn::Pat::Ident(ident) => {
            constants
                && ident.by_ref.is_none()
                && ident.mutability.is_none()
                && ident.subpat.is_none()
        }
        _ => false,
    })
}

fn expression_kind(expression: &syn::Expr) -> Option<&'static str> {
    Some(match expression {
        syn::Expr::Paren(_) | syn::Expr::Group(_) | syn::Expr::Block(_) => return None,
        syn::Expr::Lit(_) => "expr_lit",
        syn::Expr::Path(_) => "expr_path",
        syn::Expr::Binary(binary)
            if matches!(
                binary.op,
                syn::BinOp::AddAssign(_)
                    | syn::BinOp::SubAssign(_)
                    | syn::BinOp::MulAssign(_)
                    | syn::BinOp::DivAssign(_)
                    | syn::BinOp::RemAssign(_)
                    | syn::BinOp::BitXorAssign(_)
                    | syn::BinOp::BitAndAssign(_)
                    | syn::BinOp::BitOrAssign(_)
                    | syn::BinOp::ShlAssign(_)
                    | syn::BinOp::ShrAssign(_)
            ) =>
        {
            "expr_assign_op"
        }
        syn::Expr::Binary(_) => "expr_binary",
        syn::Expr::Unary(_) => "expr_unary",
        syn::Expr::Assign(_) => "expr_assign",
        syn::Expr::Cast(_) => "expr_cast",
        syn::Expr::If(_) => "expr_if",
        syn::Expr::Match(_) => "expr_match",
        syn::Expr::Return(_) => "expr_ret",
        syn::Expr::Tuple(_) => "expr_tup",
        syn::Expr::While(_) => "expr_while",
        syn::Expr::Loop(_) => "expr_loop",
        syn::Expr::Break(_) => "expr_break",
        syn::Expr::Continue(_) => "expr_continue",
        syn::Expr::Call(_) => "expr_call",
        syn::Expr::MethodCall(_) => "expr_method_call",
        syn::Expr::Macro(_) => "source_macro",
        _ => "source_deferred",
    })
}

/// Deferred constructs remain exclusions unless independent cfg evaluation removes them.
pub fn diagnostics(
    facts: &dyn crate::unit_store::EvidenceFacts,
    inventory: &Correspondence,
) -> Vec<crate::HirDiagnostic> {
    let profile =
        crate::extraction_profile::ExtractionProfile::from_facts(facts).unwrap_or_default();

    inventory.entries.iter().filter_map(|entry| {
        if entry.kind == "expr_call" && profile.supports("rust.verified-helper-calls") { return None; }
        if entry.kind == "expr_method_call" && profile.supports("rust.verified-primitive-summaries")
            && entry.hir.as_ref().is_some_and(|hir| resolved_summary(&facts.body, hir, facts)) { return None; }
        let capability = match entry.kind.as_str() {
            "expr_call" | "expr_method_call" => "rust.call-summaries",
            "source_macro" => "rust.macros",
            "stmt_item" => "rust.blocks",
            "source_deferred" => "rust.additional-types",
            _ => return None,
        };
        Some(crate::HirDiagnostic {
            capability: capability.into(), construct: entry.kind.clone(), span: entry.span,
            snippet: facts.source.bytes.get(entry.span.start as usize..entry.span.end as usize).unwrap_or_default().into(),
            severity: "error".into(), reason: format!("Original owned source contains deferred {capability}; cfg removal cannot certify a supported remainder."),
        })
    }).collect()
}

#[derive(Serialize)]
pub struct SourceDefinition {
    pub name: String,
    pub span: ByteSpan,
    pub source: String,
    /// Exact signature/body ownership, excluding documentation and attributes.
    pub function_span: ByteSpan,
    pub function_source: String,
    /// Independently false cfg on this definition or a traversed enclosing scope.
    #[serde(skip)]
    pub inactive_by_cfg: bool,
}

struct Definitions<'a> {
    source: &'a str,
    entries: Vec<SourceDefinition>,
    configuration: Option<crate::conditional_source::Configuration<'a>>,
    inactive: bool,
}

pub fn functions(source: &str) -> Result<Vec<SourceDefinition>, String> {
    configured_functions(source, None)
}

pub fn configured_functions<'a>(
    source: &'a str,
    cfgs: Option<&'a [String]>,
) -> Result<Vec<SourceDefinition>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut definitions = Definitions {
        source,
        entries: Vec::new(),
        configuration: cfgs.map(crate::conditional_source::Configuration::new),
        inactive: false,
    };
    definitions.visit_file(&file);
    Ok(definitions.entries)
}

impl Definitions<'_> {
    fn scoped(&mut self, attributes: &[syn::Attribute], visit: impl FnOnce(&mut Self)) {
        let enclosing = self.inactive;
        self.inactive = enclosing
            || (!attributes.is_empty()
                && self
                    .configuration
                    .as_ref()
                    .is_some_and(|configuration| configuration.active(attributes) == Ok(false)));
        visit(self);
        self.inactive = enclosing;
    }

    fn add(&mut self, name: &syn::Ident, span: proc_macro2::Span, owned: proc_macro2::Span) {
        let start = offset(self.source, span.start());
        let end = offset(self.source, span.end());
        self.entries.push(SourceDefinition {
            name: name.to_string(),
            inactive_by_cfg: self.inactive,
            span: ByteSpan { start, end },
            source: self
                .source
                .get(start as usize..end as usize)
                .unwrap_or_default()
                .into(),
            function_span: ByteSpan {
                start: offset(self.source, owned.start()),
                end: offset(self.source, owned.end()),
            },
            function_source: self
                .source
                .get(
                    offset(self.source, owned.start()) as usize
                        ..offset(self.source, owned.end()) as usize,
                )
                .unwrap_or_default()
                .into(),
        });
    }
}

impl<'ast> Visit<'ast> for Definitions<'_> {
    fn visit_file(&mut self, node: &'ast syn::File) {
        self.scoped(&node.attrs, |this| visit::visit_file(this, node));
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.scoped(crate::conditional_source::item_attributes(node), |this| {
            visit::visit_item(this, node)
        });
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        self.scoped(
            crate::conditional_source::expression_attributes(node),
            |this| visit::visit_expr(this, node),
        );
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        self.scoped(&node.attrs, |this| visit::visit_local(this, node));
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.scoped(&node.attrs, |this| visit::visit_arm(this, node));
    }

    fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
        self.scoped(
            match node {
                syn::ImplItem::Fn(node) => &node.attrs,
                syn::ImplItem::Const(node) => &node.attrs,
                _ => &[],
            },
            |this| visit::visit_impl_item(this, node),
        );
    }

    fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
        self.scoped(
            match node {
                syn::TraitItem::Fn(node) => &node.attrs,
                syn::TraitItem::Const(node) => &node.attrs,
                _ => &[],
            },
            |this| visit::visit_trait_item(this, node),
        );
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let owned = owned_function_span(&function.vis, &function.sig, function.block.span());
        self.add(&function.sig.ident, function.span(), owned);
        self.visit_block(&function.block);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        let owned = owned_function_span(&function.vis, &function.sig, function.block.span());
        self.add(&function.sig.ident, function.span(), owned);
        self.visit_block(&function.block);
    }

    fn visit_trait_item_fn(&mut self, function: &'ast syn::TraitItemFn) {
        if let Some(block) = &function.default {
            let owned = function
                .sig
                .span()
                .join(block.span())
                .unwrap_or(function.sig.span());
            self.add(&function.sig.ident, function.span(), owned);
            self.visit_block(block);
        }
    }
}

fn owned_function_span(
    visibility: &syn::Visibility,
    signature: &syn::Signature,
    body: proc_macro2::Span,
) -> proc_macro2::Span {
    let start = if matches!(visibility, syn::Visibility::Inherited) {
        signature.span()
    } else {
        visibility.span()
    };

    start.join(body).unwrap_or(start)
}

fn resolved_summary(
    node: &HirNode,
    hir: &HirRef,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> bool {
    (node.hir == *hir
        && (crate::inherent_calls::source_resolved(node, facts).unwrap_or(false)
            || crate::data_library::source_resolved(node, facts).unwrap_or(false)
            || crate::call_summaries::resolve_for(node, facts)
                .is_ok_and(|summary| summary.is_some())))
        || node
            .children
            .iter()
            .any(|child| resolved_summary(child, hir, facts))
}
