//! Independent original-source inventory checks; graph extraction remains HIR-only.

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
}

pub fn capture(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Correspondence, String> {
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes)
        .map_err(|error| format!("Original owned source inventory unavailable: {error}"))?;
    let mut inventory = Inventory {
        facts,
        entries: Vec::new(),
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
    if expected.entries.iter().any(|entry| entry.hir.is_none()) {
        return Err("Removed or unmatched owned-source operations lack independent resolved HIR evidence; semantics remain unknown".into());
    }

    Ok(())
}

impl Inventory<'_> {
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

    fn add(&mut self, kind: &str, span: proc_macro2::Span) {
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
        self.entries.push(Entry {
            kind: kind.into(),
            span,
            hir,
        });
    }
}

impl<'ast> Visit<'ast> for Inventory<'_> {
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
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

    fn visit_pat(&mut self, pattern: &'ast syn::Pat) {
        let constants = crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
            .is_ok_and(|profile| profile.supports("rust.scalar-constant-patterns"));
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
            syn::Pat::Or(pattern) if scalar_alternatives(pattern, constants) => Some("pat_or"),
            _ => None,
        };
        if let Some(kind) = kind {
            self.add(kind, pattern.span());
        }

        if let syn::Pat::Or(pattern) = pattern
            && scalar_alternatives(pattern, constants)
        {
            for case in &pattern.cases {
                self.visit_pat(case);
            }
        }
    }

    fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
        if matches!(statement, syn::Stmt::Macro(_)) {
            self.add("source_macro", statement.span());
            return;
        }
        visit::visit_stmt(self, statement);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        let span = local
            .let_token
            .span
            .join(local.semi_token.span)
            .unwrap_or(local.let_token.span);
        self.add("stmt_let", span);
        visit::visit_local(self, local);
    }

    fn visit_item(&mut self, item: &'ast syn::Item) {
        self.add("stmt_item", item.span());
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

    before.chars().all(|ch| ch == '(' || ch.is_whitespace())
        && after.chars().all(|ch| ch == ')' || ch.is_whitespace())
        && before.chars().filter(|ch| *ch == '(').count()
            == after.chars().filter(|ch| *ch == ')').count()
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

/// Deferred source constructs remain exclusions even when cfg removes their HIR.
pub fn diagnostics(facts: &dyn crate::unit_store::EvidenceFacts) -> Vec<crate::HirDiagnostic> {
    let Ok(inventory) = capture(facts) else {
        return Vec::new();
    };
    inventory.entries.into_iter().filter_map(|entry| {
        if entry.kind == "expr_call" && crate::extraction_profile::ExtractionProfile::from_facts(facts).is_ok_and(|profile| profile.supports("rust.verified-helper-calls")) { return None; }
        if entry.kind == "expr_method_call" && crate::extraction_profile::ExtractionProfile::from_facts(facts).is_ok_and(|profile| profile.supports("rust.verified-primitive-summaries"))
            && entry.hir.as_ref().is_some_and(|hir| resolved_summary(&facts.body, hir, facts)) { return None; }
        let capability = match entry.kind.as_str() {
            "expr_call" | "expr_method_call" => "rust.call-summaries",
            "source_macro" => "rust.macros",
            "stmt_item" => "rust.blocks",
            "source_deferred" => "rust.additional-types",
            _ => return None,
        };
        Some(crate::HirDiagnostic {
            capability: capability.into(), construct: entry.kind, span: entry.span,
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
}

struct Definitions<'a> {
    source: &'a str,
    entries: Vec<SourceDefinition>,
}

pub fn functions(source: &str) -> Result<Vec<SourceDefinition>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut definitions = Definitions {
        source,
        entries: Vec::new(),
    };
    definitions.visit_file(&file);
    Ok(definitions.entries)
}

impl Definitions<'_> {
    fn add(&mut self, name: &syn::Ident, span: proc_macro2::Span, owned: proc_macro2::Span) {
        let start = offset(self.source, span.start());
        let end = offset(self.source, span.end());
        self.entries.push(SourceDefinition {
            name: name.to_string(),
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
