//! Independent original-source inventory checks; graph extraction remains HIR-only.

mod attributes;
mod candidates;
pub(crate) mod trait_members;
pub(crate) mod type_scopes;
pub(crate) use candidates::Candidates;
pub(crate) mod checked_activation;
mod positions;
pub(crate) use attributes::{has_body_attributes, literal_metadata, single_identifier_attribute};
mod exact_pairs;
pub(crate) use exact_pairs::exact_pairs;
mod definitions;
pub(crate) use definitions::require_definition;
mod conversions;
pub(crate) use conversions::primitive_conversion;
mod syntax;
pub(crate) use syntax::{plain_type_path, plain_value_path, unique_active};
mod variant_paths;
pub(crate) use variant_paths::qualified_variant;
mod assertion_scopes;
pub(crate) use assertion_scopes::closed_assertion_body;

#[cfg(test)]
mod tests;

use crate::{ByteSpan, HirNode, HirRef};
use serde::{Deserialize, Serialize};
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
};

pub const KEY: &str = "source_correspondence";

/// Borrow an exact UTF-8 byte range; out-of-bounds and split characters fail.
pub(crate) fn byte_range(bytes: &str, span: ByteSpan) -> Result<&str, String> {
    bytes
        .get(span.start as usize..span.end as usize)
        .ok_or_else(|| "Source range is out of bounds or splits UTF-8".into())
}

/// Original leaf path with a captured local binding; supplies no type or dispatch proof.
pub(crate) fn resolved_local_leaf(node: &HirNode) -> Option<&HirRef> {
    if node.kind != "expr_path" || node.expansion.is_some() || !node.children.is_empty() {
        return None;
    }
    node.res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
}

/// Check an ordinary original read of one exact binding, including its type.
pub(crate) fn verify_binding_read(
    node: &HirNode,
    binding: &HirNode,
    diagnostic: &str,
) -> Result<(), String> {
    if resolved_local_leaf(node) != Some(&binding.hir)
        || node.dispatch.is_some()
        || node.is_unsafe_block
        || crate::data_aggregates::parsed_type(node)?
            != crate::data_aggregates::parsed_type(binding)?
    {
        return Err(diagnostic.into());
    }
    Ok(())
}

/// Project only a successfully checked whole source; never a wire claim.
pub(crate) fn source_contract<'a, T, R>(
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    verify: impl FnOnce(&'a dyn crate::unit_store::EvidenceFacts) -> Result<T, String>,
    project: impl FnOnce(&T) -> Result<R, String>,
) -> Result<R, String> {
    project(&verify(facts)?)
}

/// Distinguish an inapplicable source family from a failed complete proof.
pub(crate) fn source_present<'a, T>(
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    verify: impl FnOnce(&'a dyn crate::unit_store::EvidenceFacts) -> Result<Option<T>, String>,
) -> Result<bool, String> {
    verify(facts).map(|checked| checked.is_some())
}

/// Relative identifier paths only; this proves syntax, never name resolution.
pub(crate) fn plain_relative_path(path: &syn::Path) -> Option<Vec<String>> {
    if path.leading_colon.is_some() || path.segments.iter().any(|part| !part.arguments.is_none()) {
        return None;
    }
    Some(
        path.segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect(),
    )
}

/// Prove that module lexical resolution suffices for every scope in this body.
pub(crate) fn closed_body(body: &syn::Block) -> Result<(), String> {
    struct LocalItems(bool);
    impl<'ast> Visit<'ast> for LocalItems {
        fn visit_macro(&mut self, _: &'ast syn::Macro) {
            self.0 = true;
        }

        fn visit_stmt(&mut self, statement: &'ast syn::Stmt) {
            if matches!(statement, syn::Stmt::Item(_) | syn::Stmt::Macro(_)) {
                self.0 = true;
            }
            visit::visit_stmt(self, statement);
        }
    }

    let mut local = LocalItems(false);
    local.visit_block(body);
    if local.0 {
        return Err("Source body has unresolved block-local item/macro scopes".into());
    }
    Ok(())
}

/// Locate original syntax without repeating parser span conversion at each caller.
pub(crate) fn syntax_span(bytes: &str, syntax: &impl syn::spanned::Spanned) -> ByteSpan {
    original_span(bytes, syntax.span())
}

/// Convert a parser span to original UTF-8 offsets, retaining a leading BOM.
pub(crate) fn original_span(bytes: &str, span: proc_macro2::Span) -> ByteSpan {
    let prefix = if bytes.starts_with('\u{feff}') { 3 } else { 0 };
    let range = span.byte_range();
    ByteSpan {
        start: prefix + range.start as u32,
        end: prefix + range.end as u32,
    }
}

/// Parse an original expression slice without manufacturing syntax for expansions.
pub(crate) fn original_expression(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<syn::Expr, String> {
    if node.expansion.is_some() {
        return Err("Expression requires original source outside expansions".into());
    }
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original expression source")?;
    syn::parse_str(source).map_err(|error| error.to_string())
}

/// Bind a parsed child syntax node to its original parent slice.
pub(crate) fn verify_child_source(
    syntax: &impl Spanned,
    child: &HirNode,
    parent: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    verify_child_span(parent, child, syntax.span(), facts)
}

/// Bind a parsed child span to the same original parent slice used by the HIR.
pub(crate) fn verify_child_span(
    parent: &HirNode,
    child: &HirNode,
    span: proc_macro2::Span,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    let source = facts
        .source
        .bytes
        .get(parent.span.start as usize..parent.span.end as usize)
        .ok_or("Missing original parent source")?;
    let expected = ByteSpan {
        start: parent.span.start + byte_offset(source, span.start()) as u32,
        end: parent.span.start + byte_offset(source, span.end()) as u32,
    };

    if !corresponding_span(&facts.source.bytes, expected, child.span) {
        return Err("Child source correspondence disagrees with its original parent".into());
    }
    Ok(())
}

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
    candidates: candidates::Candidates<'a>,
    positions: positions::Positions,
    checked_options: bool,
    configuration: Option<crate::conditional_source::Configuration<'a>>,
}

pub fn capture(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Correspondence, String> {
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes)
        .map_err(|error| format!("Original owned source inventory unavailable: {error}"))?;
    let mut inventory = Inventory {
        facts,
        entries: Vec::new(),
        candidates: candidates::Candidates::new(&facts.body),
        positions: positions::Positions::new(&facts.source.bytes),
        checked_options: crate::checked_options::enabled(facts) && has_option_patterns(&facts.body),
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

fn has_option_patterns(node: &HirNode) -> bool {
    node.kind == "pat_option_some" || node.children.iter().any(has_option_patterns)
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
            start: self.positions.offset(syntax.start()),
            end: self.positions.offset(syntax.end()),
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

        if assertion.diagnostic.is_some()
            && !crate::extraction_profile::facts_support(self.facts, "rust.end-of-input")
        {
            return false;
        }
        self.add("expr_assert", mac.span());
        self.assertion_children(&assertion);
        true
    }

    fn assertion_children(&mut self, assertion: &crate::assertions::Assertion) {
        self.visit_expr(&assertion.condition);
        if let Some(right) = &assertion.right {
            self.visit_expr(right);
        }
        if let Some(diagnostic) = &assertion.diagnostic {
            for capture in &diagnostic.captures {
                self.add("expr_path", capture.span);
            }
        }
    }

    fn expression(&mut self, expression: &syn::Expr) {
        let kind = self.expression_kind(expression);
        if let Some(kind) = kind {
            self.add(kind, expression.span());
        }

        let callback = matches!(expression, syn::Expr::Closure(_))
            && crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
                .is_ok_and(|profile| profile.supports("rust.immediate-option-callbacks"));
        if matches!(expression, syn::Expr::Async(_) | syn::Expr::Const(_))
            || matches!(expression, syn::Expr::Closure(_)) && !callback
        {
            return;
        }
        self.expression_children(expression);
    }

    fn expression_children(&mut self, expression: &syn::Expr) {
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

        if let syn::Expr::Repeat(repeat) = expression
            && crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
                .is_ok_and(|profile| profile.supports("rust.repeat-array-values"))
        {
            self.visit_expr(&repeat.expr);
            return;
        }

        visit::visit_expr(self, expression);
    }

    fn expression_kind(&self, expression: &syn::Expr) -> Option<&'static str> {
        let (capability, kind) = match expression {
            syn::Expr::Try(_) => ("rust.same-error-result-propagation", "expr_try_desugar"),
            syn::Expr::Let(_) => ("rust.scalar-option-if-let", "expr_let"),
            syn::Expr::Closure(_) => ("rust.immediate-option-callbacks", "expr_closure"),
            syn::Expr::Range(_) => ("rust.shared-subslice-views", "expr_struct"),
            syn::Expr::Struct(_) => ("rust.named-struct-values", "expr_struct"),
            syn::Expr::Repeat(_) => ("rust.repeat-array-values", "expr_repeat"),
            syn::Expr::Array(_) => ("rust.aggregate-literal-values", "expr_array"),
            syn::Expr::Index(_) => ("rust.scalar-index-reads", "expr_index"),
            syn::Expr::Field(_) => ("rust.concrete-read-only-data", "expr_field"),
            syn::Expr::Reference(_) => ("rust.concrete-read-only-data", "expr_addr_of"),
            _ => return expression_kind(expression),
        };
        if crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
            .is_ok_and(|profile| profile.supports(capability))
        {
            Some(kind)
        } else {
            expression_kind(expression)
        }
    }

    fn add(&mut self, kind: &str, span: proc_macro2::Span) -> bool {
        let span = ByteSpan {
            start: self.positions.offset(span.start()),
            end: self.positions.offset(span.end()),
        };
        let hir = self.candidates.unique(&self.facts.source.bytes, kind, span);
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
        if self.checked_options && self.option_pattern(pattern) {
            return;
        }
        self.scalar_pattern(pattern, profile);
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
            let table = crate::extraction_profile::facts_support(
                self.facts,
                crate::bool_constants::CAPABILITY,
            );
            let source = &self.facts.source.bytes;
            let bytes = source
                .get(
                    self.positions.offset(item.span().start()) as usize
                        ..self.positions.offset(item.span().end()) as usize,
                )
                .unwrap_or_default();
            let kind = if table
                && matches!(item, syn::Item::Fn(_) if crate::bool_constants::function_declaration(bytes).is_ok())
            {
                "stmt_const_array_fn"
            } else if table
                && matches!(item, syn::Item::Const(_))
                && crate::bool_constants::array_declaration(bytes).is_ok()
            {
                "stmt_bool_array"
            } else if crate::extraction_profile::ExtractionProfile::from_facts(self.facts)
                .is_ok_and(|profile| profile.supports("rust.native-borrowed-byte-readers"))
                && matches!(item, syn::Item::Const(_))
            {
                "stmt_size_const"
            } else {
                "stmt_item"
            };
            self.add(kind, item.span());
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

pub(crate) fn corresponding_span(source: &str, syntax: ByteSpan, hir: ByteSpan) -> bool {
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
        syn::Expr::Assign(assign) if matches!(&*assign.left, syn::Expr::Tuple(_)) => {
            "expr_tuple_assign"
        }
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
            || crate::local_value_trait_calls::resolution(node, facts).is_some()
            || crate::helper_option_callbacks::candidate(node, facts)
                && crate::helper_option_callbacks::verify_source(node, facts).is_ok()
            || crate::named_callbacks::candidate(node, facts)
                && crate::named_callbacks::verify_source(node, facts).is_ok()
            || crate::option_callbacks::candidate(node, facts)
                && crate::option_callbacks::verify_map_or(node, facts).is_ok()
            || crate::iterator_position::source_resolved(node, hir, facts)
            || crate::data_library::source_resolved(node, facts).unwrap_or(false)
            || crate::call_summaries::resolve_for(node, facts)
                .is_ok_and(|summary| summary.is_some())))
        || node
            .children
            .iter()
            .any(|child| resolved_summary(child, hir, facts))
}

impl<'ast> Inventory<'_> {
    fn scalar_pattern(
        &mut self,
        pattern: &'ast syn::Pat,
        profile: Option<crate::extraction_profile::ExtractionProfile>,
    ) {
        let constants =
            profile.is_some_and(|profile| profile.supports("rust.scalar-constant-patterns"));
        let nested =
            profile.is_some_and(|profile| profile.supports("rust.nested-scalar-or-patterns"));
        let kind = match pattern {
            syn::Pat::Reference(_)
                if profile.is_some_and(|profile| {
                    profile.supports(crate::iterator_position::CAPABILITY)
                }) =>
            {
                Some("pat_shared_copy")
            }
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
            syn::Pat::Range(range) if self.character_range(range, profile) => Some("pat_range"),
            syn::Pat::Range(range)
                if crate::scalar_patterns::scalar_range_for(
                    range,
                    constants,
                    profile.is_some_and(|p| p.supports(crate::byte_literals::CAPABILITY)),
                ) =>
            {
                Some("pat_range")
            }
            syn::Pat::Or(pattern) if (nested || scalar_alternatives(pattern, constants)) => {
                Some("pat_or")
            }
            _ => None,
        };

        let matched = kind.is_some_and(|kind| self.add(kind, pattern.span()));
        self.pattern_children(pattern, matched, profile);
    }

    fn character_range(
        &self,
        range: &syn::PatRange,
        profile: Option<crate::extraction_profile::ExtractionProfile>,
    ) -> bool {
        let source = &self.facts.source.bytes;
        profile.is_some_and(|p| p.supports(crate::character_ranges::CAPABILITY))
            && source
                .get(
                    self.positions.byte_offset(range.span().start())
                        ..self.positions.byte_offset(range.span().end()),
                )
                .is_some_and(|text| crate::character_ranges::from_source(text).is_ok())
    }

    fn pattern_children(
        &mut self,
        pattern: &'ast syn::Pat,
        matched: bool,
        profile: Option<crate::extraction_profile::ExtractionProfile>,
    ) {
        let constants = profile.is_some_and(|p| p.supports("rust.scalar-constant-patterns"));
        let nested = profile.is_some_and(|p| p.supports("rust.nested-scalar-or-patterns"));

        if matched && let syn::Pat::Reference(reference) = pattern {
            self.visit_pat(&reference.pat);
        }

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

    fn option_pattern(&mut self, pattern: &'ast syn::Pat) -> bool {
        let span = ByteSpan {
            start: self.positions.offset(pattern.span().start()),
            end: self.positions.offset(pattern.span().end()),
        };
        let kind = match pattern {
            syn::Pat::TupleStruct(_) => "pat_option_some",
            syn::Pat::Reference(_) => "pat_shared",
            syn::Pat::Ident(_) | syn::Pat::Path(_) => "pat_option_none",
            _ => return false,
        };
        if self
            .candidates
            .unique(&self.facts.source.bytes, kind, span)
            .is_none()
        {
            return false;
        }
        self.add(kind, pattern.span());

        match pattern {
            syn::Pat::TupleStruct(some) => {
                for child in &some.elems {
                    self.visit_pat(child);
                }
            }
            syn::Pat::Reference(shared) => self.visit_pat(&shared.pat),
            _ => {}
        }
        true
    }
}

/// Compare complete ordered inventories without accepting a matching prefix.
pub(crate) fn all_pairs<L, R>(
    left: &[L],
    right: &[R],
    mut equal: impl FnMut(&L, &R) -> bool,
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| equal(left, right))
}

/// Append one lexical source name while preserving its original module route.
pub(crate) fn qualified_name(namespace: &[String], name: &str) -> Vec<String> {
    namespace.iter().cloned().chain([name.to_owned()]).collect()
}

/// Gate a structural family at its complete first statement; no proof authority.
pub(crate) fn initial_statement(
    facts: &dyn crate::unit_store::EvidenceFacts,
    capability: &str,
    predicate: impl FnOnce(&HirNode) -> bool,
) -> bool {
    crate::extraction_profile::facts_support(facts, capability)
        && facts.body.children.first().is_some_and(predicate)
}

/// Require equal cardinality and exactly one match for each expected identity.
pub(crate) fn require_exact_coverage<T, U>(
    actual: &[T],
    expected: &[U],
    matches: impl Fn(&T, &U) -> bool,
    diagnostic: &str,
) -> Result<(), String> {
    if actual.len() != expected.len()
        || expected.iter().any(|expected| {
            actual
                .iter()
                .filter(|actual| matches(actual, expected))
                .count()
                != 1
        })
    {
        return Err(diagnostic.into());
    }
    Ok(())
}
