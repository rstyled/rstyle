//! Independent bounded source/substitution flow, without lexical trait or owner authority.
use crate::{BindingFact, HirFnFacts, HirNode, HirRef, ResolvedTy};
use std::collections::{BTreeMap, BTreeSet};
use syn::spanned::Spanned;

#[cfg(test)]
mod tests;

/// Only source shape, preserved evidence, and concrete scalar flow are checked.
/// This witness grants neither source-owner nor bound-trait authority.
pub(super) struct Checked<'a>(&'a HirFnFacts);

impl<'a> Checked<'a> {
    pub(super) fn facts(&self) -> &'a HirFnFacts {
        self.0
    }
}

type Result<T> = std::result::Result<T, String>;
type Site = (u32, u32, u32);

pub(super) fn verify<'a>(
    original: &HirFnFacts,
    projected: &'a HirFnFacts,
    scalar: &str,
) -> Result<Checked<'a>> {
    if original.source.bytes.len() > crate::concrete_types::MAX_SOURCE_BYTES
        || original.bindings.len() > 256
        || original.signature.params.len() > 64
    {
        return Err("Scalar instance source exceeds its bounded proof budget".into());
    }
    if !matches!(scalar, "i32" | "u32") {
        return Err("Scalar instance source argument is outside i32/u32".into());
    }

    preserved(original, projected)?;
    let syntax: syn::ItemFn = syn::parse_str(&original.source.bytes).map_err(|e| e.to_string())?;
    let parameter = signature(&syntax)?;
    signature_metadata(&syntax, projected, &parameter)?;

    let mut flow = Flow::new(projected, scalar, parameter)?;
    flow.parameters(&syntax.sig)?;
    let expected = flow.annotation(&syntax.sig.output)?;
    flow.result = expected;

    let actual = flow.block(&syntax.block, &projected.body)?;
    if actual != expected && actual != Value::Never
        || flow.bindings.len() != projected.bindings.len()
    {
        return Err("Scalar instance result or complete binding inventory disagrees".into());
    }
    Ok(Checked(projected))
}

fn preserved(original: &HirFnFacts, projected: &HirFnFacts) -> Result<()> {
    if original.name != projected.name
        || original.fn_span != projected.fn_span
        || original.source != projected.source
        || original.build != projected.build
        || original.source_diagnostics != projected.source_diagnostics
        || original.source_scan_complete != projected.source_scan_complete
        || original.bindings.len() != projected.bindings.len()
    {
        return Err("Scalar instance changed original source/build evidence".into());
    }
    signature_shape(&original.signature, &projected.signature)?;
    for (before, after) in original.bindings.iter().zip(&projected.bindings) {
        if (
            &before.symbol,
            &before.name,
            before.decl_span,
            &before.scope,
            &before.hir,
            before.is_param,
            before.is_mut,
            &before.ty.spelling,
        ) != (
            &after.symbol,
            &after.name,
            after.decl_span,
            &after.scope,
            &after.hir,
            after.is_param,
            after.is_mut,
            &after.ty.spelling,
        ) {
            return Err("Scalar instance changed original binding identity".into());
        }
    }
    node_shape(&original.body, &projected.body)
}

fn signature_shape(before: &crate::SignatureFacts, after: &crate::SignatureFacts) -> Result<()> {
    if (
        &before.sig_span,
        &before.receiver,
        &before.return_ty_span,
        before.has_self,
        &before.abi,
        &before.generics,
        before.is_variadic,
        before.is_async,
        before.is_unsafe,
        before.is_const,
        &before.return_ty.spelling,
    ) != (
        &after.sig_span,
        &after.receiver,
        &after.return_ty_span,
        after.has_self,
        &after.abi,
        &after.generics,
        after.is_variadic,
        after.is_async,
        after.is_unsafe,
        after.is_const,
        &after.return_ty.spelling,
    ) || before.params.len() != after.params.len()
    {
        return Err("Scalar instance changed original signature shape".into());
    }
    for (original, projected) in before.params.iter().zip(&after.params) {
        if (
            &original.symbol,
            original.position,
            original.name_span,
            original.ty_span,
            &original.ty.spelling,
        ) != (
            &projected.symbol,
            projected.position,
            projected.name_span,
            projected.ty_span,
            &projected.ty.spelling,
        ) {
            return Err("Scalar instance changed original parameter identity".into());
        }
        node_shape(&original.pat, &projected.pat)?;
    }
    Ok(())
}

fn node_shape(before: &HirNode, after: &HirNode) -> Result<()> {
    node_shape_at(before, after, 0)
}

fn node_shape_at(before: &HirNode, after: &HirNode, depth: usize) -> Result<()> {
    if depth > 64 {
        return Err("Scalar instance HIR exceeds its bounded nesting depth".into());
    }
    if (
        &before.hir,
        &before.kind,
        before.span,
        &before.snippet,
        &before.res,
        &before.expansion,
        &before.binding,
        before.binding_mut,
        &before.lit,
        before.param_index,
        before.is_unsafe_block,
    ) != (
        &after.hir,
        &after.kind,
        after.span,
        &after.snippet,
        &after.res,
        &after.expansion,
        &after.binding,
        after.binding_mut,
        &after.lit,
        after.param_index,
        after.is_unsafe_block,
    ) || before.ty.is_some() != after.ty.is_some()
        || before.children.len() != after.children.len()
        || before.ty.as_ref().map(|ty| &ty.spelling) != after.ty.as_ref().map(|ty| &ty.spelling)
        || before
            .dispatch
            .as_ref()
            .map(|dispatch| (&dispatch.kind, &dispatch.op))
            != after
                .dispatch
                .as_ref()
                .map(|dispatch| (&dispatch.kind, &dispatch.op))
    {
        return Err("Scalar instance changed original complete HIR shape".into());
    }
    for (original, projected) in before.children.iter().zip(&after.children) {
        node_shape_at(original, projected, depth + 1)?;
    }
    Ok(())
}

fn signature(function: &syn::ItemFn) -> Result<String> {
    let signature = &function.sig;
    attributes(&function.attrs)?;
    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.generics.where_clause.is_some()
        || signature.generics.params.len() != 1
    {
        return Err("Scalar instance requires one ordinary bare type parameter".into());
    }

    let Some(syn::GenericParam::Type(parameter)) = signature.generics.params.first() else {
        return Err("Scalar instance requires one original type parameter".into());
    };
    if !parameter.attrs.is_empty() || parameter.default.is_some() || parameter.bounds.len() != 1 {
        return Err("Scalar instance requires one exact comparison bound".into());
    }

    let Some(syn::TypeParamBound::Trait(bound)) = parameter.bounds.first() else {
        return Err("Scalar instance comparison bound is not an original trait path".into());
    };
    if bound.paren_token.is_some()
        || bound.lifetimes.is_some()
        || !matches!(bound.modifier, syn::TraitBoundModifier::None)
        || !comparison_bound(&bound.path)
    {
        return Err(
            "Scalar instance requires the bounded PartialOrd/PartialEq source shape".into(),
        );
    }
    Ok(parameter.ident.to_string())
}

fn signature_metadata(syntax: &syn::ItemFn, facts: &HirFnFacts, parameter: &str) -> Result<()> {
    let signature = &facts.signature;
    let generic = signature
        .generics
        .first()
        .ok_or("Missing scalar instance generic declaration")?;
    let original = syntax
        .sig
        .generics
        .params
        .first()
        .ok_or("Missing source type parameter")?;

    let syn::GenericParam::Type(original) = original else {
        unreachable!()
    };
    if signature.generics.len() != 1
        || generic.name != parameter
        || generic.span
            != crate::source_evidence::original_span(&facts.source.bytes, original.ident.span())
        || signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_unsafe
        || signature.is_const
        || signature.is_variadic
        || signature.abi.value.as_deref() != Some("Rust")
    {
        return Err("Scalar instance original signature metadata disagrees".into());
    }
    Ok(())
}

fn comparison_bound(path: &syn::Path) -> bool {
    let Some(names) = crate::source_evidence::plain_relative_path(path) else {
        return false;
    };
    names
        .last()
        .is_some_and(|name| matches!(name.as_str(), "PartialOrd" | "PartialEq"))
        && (names.len() == 1 || names.len() == 3 && names[0] == "core" && names[1] == "cmp")
}

fn attributes(attributes: &[syn::Attribute]) -> Result<()> {
    for attribute in attributes {
        if matches!(&attribute.meta, syn::Meta::Path(path) if path.is_ident("inline")) {
            continue;
        }
        let syn::Meta::List(list) = &attribute.meta else {
            return Err("Scalar instance has unsupported original attributes".into());
        };
        let path = list.parse_args::<syn::Path>().map_err(|e| e.to_string())?;
        if !list.path.is_ident("allow")
            || path.segments.len() != 2
            || path.leading_colon.is_some()
            || path.segments.iter().any(|part| !part.arguments.is_empty())
            || path
                .segments
                .first()
                .is_none_or(|part| part.ident != "clippy")
            || path
                .segments
                .last()
                .is_none_or(|part| part.ident != "eq_op")
        {
            return Err(
                "Scalar instance only permits exact inline/allow(clippy::eq_op) attributes".into(),
            );
        }
    }
    Ok(())
}

fn node_metadata(node: &HirNode) -> Result<()> {
    let binding = node.kind == "pat_binding";
    if !binding
        && (node.binding.is_some() || node.binding_mut.is_some() || node.param_index.is_some())
        || node.kind != "expr_binary" && node.dispatch.is_some()
        || !matches!(node.kind.as_str(), "expr_path" | "expr_assert") && node.res.is_some()
        || node.kind != "expr_assert" && node.expansion.is_some()
        || !matches!(node.kind.as_str(), "expr_lit" | "expr_assert") && node.lit.is_some()
        || node.ty.as_ref().is_some_and(|ty| !ty.spelling.is_empty())
    {
        return Err("Scalar instance node contains contradictory extra metadata".into());
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Value {
    Scalar,
    Bool,
    Unit,
    Never,
}

#[derive(Clone, Copy)]
struct ExpectedBinding {
    value: Value,
    is_param: bool,
}

#[derive(Clone)]
struct Binding {
    hir: HirRef,
    value: Value,
}

/// Parser columns count Unicode scalar values, not UTF-8 bytes. Index every
/// column boundary once; source locations then need no repeated character scan.
struct SpanIndex {
    lines: Vec<Vec<u32>>,
}

impl SpanIndex {
    fn new(source: &str) -> Self {
        let text = source.strip_prefix('\u{feff}').unwrap_or(source);
        let prefix = source.len() - text.len();
        let mut lines = vec![vec![prefix as u32]];

        for (offset, character) in text.char_indices() {
            let next = (prefix + offset + character.len_utf8()) as u32;
            if character == '\n' {
                lines.push(vec![next]);
            } else {
                let line = lines.len() - 1;
                lines[line].push(next);
            }
        }
        Self { lines }
    }

    fn offset(&self, location: proc_macro2::LineColumn) -> Result<u32> {
        location
            .line
            .checked_sub(1)
            .and_then(|line| self.lines.get(line))
            .and_then(|columns| columns.get(location.column))
            .copied()
            .ok_or_else(|| "Scalar parser location is outside original UTF-8 source".into())
    }

    fn span(&self, span: proc_macro2::Span) -> Result<crate::ByteSpan> {
        Ok(crate::ByteSpan {
            start: self.offset(span.start())?,
            end: self.offset(span.end())?,
        })
    }
}

struct Branches<'a> {
    alternative: &'a syn::Expr,
    yes: &'a HirNode,
    no: &'a HirNode,
}

struct Flow<'a> {
    facts: &'a HirFnFacts,
    offsets: SpanIndex,
    scalar: &'a str,
    parameter: String,
    result: Value,
    scope: BTreeMap<String, Vec<Binding>>,
    frames: Vec<Vec<String>>,
    inventory: BTreeMap<&'a str, &'a BindingFact>,
    bindings: BTreeSet<String>,
    seen: BTreeSet<Site>,
}

impl<'a> Flow<'a> {
    fn new(facts: &'a HirFnFacts, scalar: &'a str, parameter: String) -> Result<Self> {
        let mut inventory = BTreeMap::new();
        for binding in &facts.bindings {
            if inventory.insert(binding.symbol.as_str(), binding).is_some() {
                return Err("Scalar instance binding inventory repeats a symbol".into());
            }
        }
        Ok(Self {
            facts,
            offsets: SpanIndex::new(&facts.source.bytes),
            scalar,
            parameter,
            result: Value::Scalar,
            scope: BTreeMap::new(),
            frames: vec![Vec::new()],
            inventory,
            bindings: BTreeSet::new(),
            seen: BTreeSet::new(),
        })
    }

    fn parameters(&mut self, signature: &syn::Signature) -> Result<()> {
        if signature.inputs.len() != self.facts.signature.params.len() {
            return Err("Scalar instance source parameter inventory disagrees".into());
        }
        for (index, input) in signature.inputs.iter().enumerate() {
            let syn::FnArg::Typed(input) = input else {
                return Err("Scalar instance receiver is unsupported".into());
            };
            let parameter = &self.facts.signature.params[index];
            let value = self.ty(&input.ty)?;
            self.resolved(&parameter.ty, value)?;
            self.annotation_spelling(&parameter.ty, input.ty.span())?;
            if parameter.position != index as u32
                || parameter.ty_span != self.span(input.ty.span())?
                || parameter.pat.param_index != Some(index as u32)
            {
                return Err("Scalar instance source parameter position disagrees".into());
            }
            self.binding(
                &input.pat,
                &parameter.pat,
                ExpectedBinding {
                    value,
                    is_param: true,
                },
            )?;
            if parameter.pat.binding.as_ref() != Some(&parameter.symbol)
                || parameter.name_span != parameter.pat.span
            {
                return Err("Scalar instance parameter binding disagrees".into());
            }
        }
        Ok(())
    }

    fn ty(&self, ty: &syn::Type) -> Result<Value> {
        let syn::Type::Path(path) = ty else {
            return Err("Scalar instance annotation is outside the bounded scalar grammar".into());
        };
        if path.qself.is_none() && path.path.is_ident(&self.parameter) {
            return Ok(Value::Scalar);
        }
        if path.qself.is_none() && path.path.is_ident("bool") {
            return Ok(Value::Bool);
        }
        Err("Scalar instance annotation is not its bare parameter or bool".into())
    }

    fn annotation(&self, output: &syn::ReturnType) -> Result<Value> {
        let value = match output {
            syn::ReturnType::Type(_, ty) => {
                self.annotation_spelling(&self.facts.signature.return_ty, ty.span())?;
                if self.facts.signature.return_ty_span != self.span(ty.span())? {
                    return Err("Scalar instance result annotation span disagrees".into());
                }
                self.ty(ty)?
            }
            syn::ReturnType::Default => Value::Unit,
        };
        self.resolved(&self.facts.signature.return_ty, value)?;
        Ok(value)
    }

    fn annotation_spelling(&self, ty: &ResolvedTy, span: proc_macro2::Span) -> Result<()> {
        let span = self.span(span)?;
        if self
            .facts
            .source
            .bytes
            .get(span.start as usize..span.end as usize)
            != Some(ty.spelling.as_str())
        {
            return Err(
                "Scalar instance annotation does not preserve exact original spelling".into(),
            );
        }
        Ok(())
    }

    fn resolved(&self, ty: &ResolvedTy, value: Value) -> Result<()> {
        let expected = match value {
            Value::Scalar => Some(self.scalar),
            Value::Bool => Some("bool"),
            Value::Unit => Some("unit"),
            Value::Never => None,
        };
        if ty.normalized.as_deref() != expected
            || matches!(value, Value::Never) && ty.observed != "!"
        {
            return Err(
                "Scalar instance resolved type disagrees with independent source flow".into(),
            );
        }
        Ok(())
    }

    fn span(&self, span: proc_macro2::Span) -> Result<crate::ByteSpan> {
        self.offsets.span(span)
    }

    fn node(&mut self, node: &HirNode, kind: &str, span: proc_macro2::Span) -> Result<()> {
        node_metadata(node)?;
        let site = (node.hir.krate, node.hir.owner, node.hir.local);
        if node.kind != kind
            || node.span != self.span(span)?
            || node.is_unsafe_block
            || node.hir.krate != 0
            || node.hir.owner != self.facts.body.hir.owner
            || !self.seen.insert(site)
            || self
                .facts
                .source
                .bytes
                .get(node.span.start as usize..node.span.end as usize)
                != Some(node.snippet.as_str())
        {
            return Err("Scalar instance HIR node lacks exact original source/owner shape".into());
        }
        Ok(())
    }

    fn typed(&self, node: &HirNode, value: Value) -> Result<Value> {
        self.resolved(
            node.ty
                .as_ref()
                .ok_or("Missing scalar instance node type")?,
            value,
        )?;
        Ok(value)
    }

    fn binding(&mut self, pat: &syn::Pat, node: &HirNode, expected: ExpectedBinding) -> Result<()> {
        let syn::Pat::Ident(pat) = pat else {
            return Err("Scalar instance requires an ordinary binding pattern".into());
        };
        if !pat.attrs.is_empty()
            || pat.by_ref.is_some()
            || pat.mutability.is_some()
            || pat.subpat.is_some()
        {
            return Err("Scalar instance binding modifiers are unsupported".into());
        }

        self.node(node, "pat_binding", pat.span())?;
        self.typed(node, expected.value)?;
        let symbol = node
            .binding
            .as_ref()
            .ok_or("Missing scalar instance binding symbol")?;
        let binding = self
            .inventory
            .get(symbol.as_str())
            .copied()
            .ok_or("Missing scalar instance binding record")?;

        self.binding_record(binding, node, expected)?;
        self.install_binding(binding, expected.value)
    }

    // rstyle-allow(side_effect_density): Lexical binding installation keeps symbol membership, shadow stacks and the current frame aligned.
    fn install_binding(&mut self, binding: &BindingFact, value: Value) -> Result<()> {
        self.bindings.insert(binding.symbol.clone());
        self.scope
            .entry(binding.name.clone())
            .or_default()
            .push(Binding {
                hir: binding.hir.clone(),
                value,
            });
        self.frames
            .last_mut()
            .ok_or("Missing scalar lexical frame")?
            .push(binding.name.clone());
        Ok(())
    }

    fn binding_record(
        &self,
        binding: &BindingFact,
        node: &HirNode,
        expected: ExpectedBinding,
    ) -> Result<()> {
        self.resolved(&binding.ty, expected.value)?;
        if !binding.ty.spelling.is_empty()
            || node.param_index.is_some() != expected.is_param
            || self.bindings.contains(&binding.symbol)
            || binding.hir != node.hir
            || binding.decl_span != node.span
            || binding.name != node.snippet
            || binding.is_param != expected.is_param
            || binding.is_mut
            || node.binding_mut != Some(false)
            || !node.children.is_empty()
            || node.dispatch.is_some()
            || node.res.is_some()
            || node.expansion.is_some()
            || node.lit.is_some()
        {
            return Err("Scalar instance binding does not match its exact declaration".into());
        }
        Ok(())
    }

    fn block(&mut self, syntax: &syn::Block, node: &HirNode) -> Result<Value> {
        self.enter_block(syntax, node)?;
        let mut value = Value::Unit;
        for (index, statement) in syntax.stmts.iter().enumerate() {
            if value == Value::Never {
                return Err("Unreachable scalar instance source suffix is unsupported".into());
            }
            value = self.statement(statement, &node.children[index])?;
        }

        self.exit_block()?;
        self.typed(node, value)
    }

    fn enter_block(&mut self, syntax: &syn::Block, node: &HirNode) -> Result<()> {
        self.node(node, "expr_block", syntax.span())?;
        if syntax.stmts.len() != node.children.len() {
            return Err("Scalar instance block inventory disagrees".into());
        }

        self.frames.push(Vec::new());
        Ok(())
    }

    fn exit_block(&mut self) -> Result<()> {
        for name in self.frames.pop().ok_or("Missing scalar block frame")? {
            self.scope
                .get_mut(&name)
                .ok_or("Missing scalar local binding")?
                .pop();
        }
        Ok(())
    }

    fn statement(&mut self, syntax: &syn::Stmt, node: &HirNode) -> Result<Value> {
        let transition: fn(&mut Self, &syn::Stmt, &HirNode) -> Result<Value> = match syntax {
            syn::Stmt::Local(_) => Self::local,
            syn::Stmt::Expr(_, None) => Self::tail,
            syn::Stmt::Expr(_, Some(_)) => Self::discarded,
            syn::Stmt::Macro(statement) if statement.attrs.is_empty() => Self::macro_statement,
            _ => {
                return Err(
                    "Scalar instance source contains an unsupported or nested owner statement"
                        .into(),
                );
            }
        };
        transition(self, syntax, node)
    }

    fn tail(&mut self, syntax: &syn::Stmt, node: &HirNode) -> Result<Value> {
        let syn::Stmt::Expr(expression, None) = syntax else {
            unreachable!()
        };
        self.expression(expression, node)
    }

    fn discarded(&mut self, syntax: &syn::Stmt, node: &HirNode) -> Result<Value> {
        let syn::Stmt::Expr(expression, Some(_)) = syntax else {
            unreachable!()
        };
        let child = self.discarded_statement_child(node, syntax.span())?;
        let value = self.expression(expression, child)?;
        Ok(if value == Value::Never {
            value
        } else {
            Value::Unit
        })
    }

    fn macro_statement(&mut self, syntax: &syn::Stmt, node: &HirNode) -> Result<Value> {
        let syn::Stmt::Macro(statement) = syntax else {
            unreachable!()
        };
        let child = self.discarded_statement_child(node, statement.mac.span())?;
        self.assertion(&statement.mac, child)
    }

    fn discarded_statement_child<'n>(
        &mut self,
        node: &'n HirNode,
        span: proc_macro2::Span,
    ) -> Result<&'n HirNode> {
        self.node(node, "stmt_semi", span)?;
        let [child] = node.children.as_slice() else {
            return Err("Scalar discarded statement arity disagrees".into());
        };
        if node.ty.is_some() {
            return Err("Scalar discarded statement unexpectedly has a value type".into());
        }
        Ok(child)
    }

    fn local(&mut self, syntax: &syn::Stmt, node: &HirNode) -> Result<Value> {
        let syn::Stmt::Local(syntax) = syntax else {
            unreachable!()
        };
        self.node(node, "stmt_let", syntax.span())?;
        let [pattern, initializer] = node.children.as_slice() else {
            return Err("Scalar local arity disagrees".into());
        };
        let init = syntax
            .init
            .as_ref()
            .ok_or("Scalar local requires its complete initializer")?;
        if !syntax.attrs.is_empty() || init.diverge.is_some() || node.ty.is_some() {
            return Err("Scalar local attributes/diverging initializer are unsupported".into());
        }

        let value = self.expression(&init.expr, initializer)?;
        let pat = if let syn::Pat::Type(annotation) = &syntax.pat {
            if !annotation.attrs.is_empty() || self.ty(&annotation.ty)? != value {
                return Err("Scalar local annotation disagrees with initializer".into());
            }
            annotation.pat.as_ref()
        } else {
            &syntax.pat
        };

        self.binding(
            pat,
            pattern,
            ExpectedBinding {
                value,
                is_param: false,
            },
        )?;
        Ok(Value::Unit)
    }

    fn expression(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let transition: fn(&mut Self, &syn::Expr, &HirNode) -> Result<Value> = match syntax {
            syn::Expr::Path(path) if path.attrs.is_empty() => Self::path,
            syn::Expr::Block(block) if block.attrs.is_empty() && block.label.is_none() => {
                Self::block_expression
            }
            syn::Expr::If(branch) if branch.attrs.is_empty() => Self::branch,
            syn::Expr::Binary(binary) if binary.attrs.is_empty() => Self::comparison,
            syn::Expr::Lit(literal) if literal.attrs.is_empty() => Self::boolean,
            syn::Expr::Return(ret) if ret.attrs.is_empty() => Self::returned,
            syn::Expr::Macro(mac) if mac.attrs.is_empty() => Self::macro_expression,
            _ => {
                return Err(
                    "Scalar instance expression exceeds its complete bounded grammar".into(),
                );
            }
        };
        transition(self, syntax, node)
    }

    fn block_expression(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Block(block) = syntax else {
            unreachable!()
        };
        self.block(&block.block, node)
    }

    fn macro_expression(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Macro(mac) = syntax else {
            unreachable!()
        };
        self.assertion(&mac.mac, node)
    }

    fn path(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Path(syntax) = syntax else {
            unreachable!()
        };
        self.node(node, "expr_path", syntax.span())?;
        let name = syntax
            .path
            .get_ident()
            .ok_or("Scalar instance path is not a bare local")?;
        if syntax.qself.is_some() || !node.children.is_empty() {
            return Err("Scalar instance local path shape disagrees".into());
        }

        let binding = self
            .scope
            .get(&name.to_string())
            .and_then(|values| values.last())
            .ok_or("Scalar instance path lacks a lexical binding")?;
        let resolution = node
            .res
            .as_ref()
            .ok_or("Scalar instance local path lacks resolution")?;
        if resolution.kind != "local" || resolution.target.as_ref() != Some(&binding.hir) {
            return Err("Scalar instance path resolves to another binding".into());
        }
        self.typed(node, binding.value)
    }

    fn branch(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::If(syntax) = syntax else {
            unreachable!()
        };
        let branches = self.branch_inputs(syntax, node)?;
        let yes = self.block(&syntax.then_branch, branches.yes)?;
        let no = self.expression(branches.alternative, branches.no)?;

        let value = if yes == Value::Never {
            no
        } else if no == Value::Never || yes == no {
            yes
        } else {
            return Err("Scalar if branches have different types".into());
        };
        self.typed(node, value)
    }

    fn branch_inputs<'s>(
        &mut self,
        syntax: &'s syn::ExprIf,
        node: &'s HirNode,
    ) -> Result<Branches<'s>> {
        self.node(node, "expr_if", syntax.span())?;
        let Some((_, alternative)) = &syntax.else_branch else {
            return Err("Scalar instance if requires complete alternative".into());
        };
        let [condition, yes, no] = node.children.as_slice() else {
            return Err("Scalar if branch inventory disagrees".into());
        };
        if self.expression(&syntax.cond, condition)? != Value::Bool {
            return Err("Scalar if condition is not bool".into());
        }

        Ok(Branches {
            alternative,
            yes,
            no,
        })
    }

    fn comparison(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Binary(syntax) = syntax else {
            unreachable!()
        };
        self.node(node, "expr_binary", syntax.span())?;
        let op = match syntax.op {
            syn::BinOp::Lt(_) => "<",
            syn::BinOp::Le(_) => "<=",
            syn::BinOp::Gt(_) => ">",
            syn::BinOp::Ge(_) => ">=",
            syn::BinOp::Eq(_) => "==",
            syn::BinOp::Ne(_) => "!=",
            _ => return Err("Scalar instance only supports comparison operators".into()),
        };
        let [left, right] = node.children.as_slice() else {
            return Err("Scalar comparison arity disagrees".into());
        };
        if self.expression(&syntax.left, left)? != Value::Scalar
            || self.expression(&syntax.right, right)? != Value::Scalar
        {
            return Err("Scalar comparison operands disagree with substitution".into());
        }

        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or("Scalar comparison lacks dispatch")?;
        if dispatch.kind != "overloaded"
            || dispatch.op != op
            || dispatch.operand_tys != [self.scalar, self.scalar]
            || dispatch.result_ty != "bool"
        {
            return Err("Scalar comparison dispatch disagrees with exact projected flow".into());
        }
        self.typed(node, Value::Bool)
    }

    fn boolean(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Lit(syntax) = syntax else {
            unreachable!()
        };
        self.node(node, "expr_lit", syntax.span())?;
        let syn::Lit::Bool(value) = &syntax.lit else {
            return Err("Scalar instance only supports bool literals".into());
        };

        if !node.children.is_empty()
            || node.lit.as_deref() != Some(value.value.to_string().as_str())
        {
            return Err("Scalar bool literal evidence disagrees".into());
        }
        self.typed(node, Value::Bool)
    }

    fn returned(&mut self, syntax: &syn::Expr, node: &HirNode) -> Result<Value> {
        let syn::Expr::Return(syntax) = syntax else {
            unreachable!()
        };
        self.node(node, "expr_ret", syntax.span())?;
        let (Some(expression), [child]) = (&syntax.expr, node.children.as_slice()) else {
            return Err("Scalar return requires one original value".into());
        };
        if self.expression(expression, child)? != self.result {
            return Err("Scalar return type disagrees".into());
        }

        // Diagnostic capture represents return nodes using the function result.
        self.typed(node, self.result)?;
        Ok(Value::Never)
    }

    fn assertion(&mut self, syntax: &syn::Macro, node: &HirNode) -> Result<Value> {
        self.node(node, "expr_assert", syntax.span())?;
        let assertion = crate::assertions::Assertion::from_macro(syntax)?;
        if assertion.right.is_some() {
            return Err("Scalar instance only supports condition assertions".into());
        }
        let [condition] = node.children.as_slice() else {
            return Err("Scalar assertion condition inventory disagrees".into());
        };
        if self.expression(&assertion.condition, condition)? != Value::Bool {
            return Err("Scalar assertion condition must be bool".into());
        }

        crate::assertions::verify(
            node,
            &node.snippet,
            self.facts.build.debug_assertions.value,
            crate::extraction_profile::ExtractionProfile::from_facts(self.facts)?,
        )?;
        self.typed(node, Value::Unit)
    }
}
