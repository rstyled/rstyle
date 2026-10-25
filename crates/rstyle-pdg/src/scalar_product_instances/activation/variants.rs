//! Exact pinned Some/None and product payload types; no external call authority.
use super::*;
use crate::data_types::TypeToken;
use syn::spanned::Spanned;

pub(super) struct CheckedVariant<'a> {
    pub some: &'a HirNode,
    pub product: &'a HirNode,
}

pub(super) fn verify<'a>(
    prefix: &semantics::Checked<'a>,
    instance: &Instance,
    input: &dyn EvidenceFacts,
) -> Result<CheckedVariant<'a>, String> {
    let success = success_body(prefix, input)?;
    let [product] = success.children.as_slice() else {
        return Err("Digit product Some must contain exactly one product value".into());
    };
    let payload =
        TypeToken::from_tree(&TypeTree::ScalarProductInstance(Box::new(instance.clone())));
    let option = TypeToken::Option(Box::new(payload.clone()));
    for node in [
        &input.body,
        prefix.matched(),
        prefix.success(),
        success,
        prefix.failure(),
    ] {
        if crate::data_aggregates::parsed_type(node)? != option {
            return Err(
                "Digit product body or variant disagrees with its exact Option instance".into(),
            );
        }
    }
    if crate::data_aggregates::parsed_type(product)? != payload {
        return Err("Digit product Some payload type disagrees with its exact instance".into());
    }

    some(success, product, input)?;
    Ok(CheckedVariant {
        some: success,
        product,
    })
}

/// Retain the entire success block; this selects its variant only after every
/// original scalar binding and its initializer has an exact source/HIR join.
pub(super) fn success_body<'a>(
    prefix: &semantics::Checked<'a>,
    input: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    let body = prefix.success();
    if body.kind != "expr_block" {
        return Ok(body);
    }
    let syntax = crate::source_evidence::original_expression(body, input)?;
    let syn::Expr::Block(block) = syntax else {
        return Err("Digit success block must retain its original block syntax".into());
    };
    let (scalar, count) = match prefix.consumed() {
        2 => ("u8", 2),
        3 => ("u16", 3),
        _ => return Err("Only original two/three-digit arithmetic blocks are supported".into()),
    };
    if body.expansion.is_some()
        || body.dispatch.is_some()
        || body.is_unsafe_block
        || !block.attrs.is_empty()
        || block.label.is_some()
        || block.block.stmts.len() != count + 1
        || body.children.len() != count + 1
        || crate::source_evidence::byte_range(&input.source.bytes, body.span)? != body.snippet
    {
        return Err("Digit success block cannot omit or append original statements".into());
    }
    let prelude = Prelude {
        input,
        scalar,
        body,
    };
    for (index, digit) in prefix.digits().iter().enumerate() {
        prelude.binding(&block.block.stmts[index], &body.children[index], digit)?;
    }
    prelude.inventory(body, count)?;
    let syn::Stmt::Expr(tail, None) = &block.block.stmts[count] else {
        return Err("Digit success block requires its original final value expression".into());
    };
    let success = &body.children[count];
    crate::source_evidence::verify_child_span(body, success, tail.span(), input)?;

    Ok(success)
}

struct Prelude<'a> {
    input: &'a dyn EvidenceFacts,
    scalar: &'static str,
    body: &'a HirNode,
}

impl Prelude<'_> {
    fn binding(
        &self,
        source: &syn::Stmt,
        node: &HirNode,
        digit: &semantics::Digit<'_>,
    ) -> Result<(), String> {
        let syn::Stmt::Local(local) = source else {
            return Err("Digit arithmetic prelude requires only original immutable lets".into());
        };
        let syn::Pat::Ident(pattern) = &local.pat else {
            return Err("Digit arithmetic prelude requires scalar identifier bindings".into());
        };
        let Some(init) = &local.init else {
            return Err("Digit arithmetic binding cannot be uninitialized".into());
        };
        let (binding, value) = self.binding_parts(local, node)?;

        crate::source_evidence::verify_child_span(self.body, node, local.span(), self.input)?;
        crate::source_evidence::verify_child_span(
            self.body,
            binding,
            local.pat.span(),
            self.input,
        )?;
        crate::source_evidence::verify_child_span(self.body, value, init.expr.span(), self.input)?;
        self.record(binding, &pattern.ident)?;
        self.initializer(
            &crate::source_evidence::original_expression(value, self.input)?,
            value,
            digit,
        )
    }

    fn binding_parts<'a>(
        &self,
        local: &syn::Local,
        node: &'a HirNode,
    ) -> Result<(&'a HirNode, &'a HirNode), String> {
        let (syn::Pat::Ident(pattern), Some(init)) = (&local.pat, &local.init) else {
            return Err("Digit arithmetic requires an initialized identifier".into());
        };
        let [binding, value] = node.children.as_slice() else {
            return Err("Digit arithmetic let must retain both binding and initializer".into());
        };
        if node.kind != "stmt_let"
            || node.expansion.is_some()
            || node.dispatch.is_some()
            || node.is_unsafe_block
            || !local.attrs.is_empty()
            || !pattern.attrs.is_empty()
            || pattern.by_ref.is_some()
            || pattern.mutability.is_some()
            || pattern.subpat.is_some()
            || init.diverge.is_some()
            || binding.kind != "pat_binding"
            || binding.binding_mut != Some(false)
            || binding.binding.is_none()
            || !binding.children.is_empty()
            || binding.expansion.is_some()
            || crate::source_evidence::byte_range(&self.input.source.bytes, node.span)?
                != node.snippet
            || crate::data_aggregates::token(binding)? != self.scalar
            || crate::data_aggregates::token(value)? != self.scalar
        {
            return Err(
                "Digit arithmetic prelude changes its original immutable scalar binding".into(),
            );
        }
        Ok((binding, value))
    }

    fn record(&self, binding: &HirNode, name: &syn::Ident) -> Result<(), String> {
        let records = self
            .input
            .bindings
            .iter()
            .filter(|record| record.hir == binding.hir)
            .collect::<Vec<_>>();
        let [record] = records.as_slice() else {
            return Err(
                "Digit arithmetic local requires exactly one original binding record".into(),
            );
        };
        if *name != record.name
            || record.is_mut
            || record.is_param
            || record.decl_span != binding.span
            || Some(&record.symbol) != binding.binding.as_ref()
            || record.ty.normalized.as_deref() != Some(self.scalar)
        {
            return Err("Digit arithmetic local binding inventory disagrees".into());
        }
        Ok(())
    }

    fn initializer(
        &self,
        source: &syn::Expr,
        node: &HirNode,
        digit: &semantics::Digit<'_>,
    ) -> Result<(), String> {
        let (source, node) = if self.scalar == "u16" {
            let syn::Expr::Cast(cast) = source else {
                return Err("Three-digit arithmetic must retain its explicit u16 cast".into());
            };
            let [child] = node.children.as_slice() else {
                return Err("Digit scalar cast cannot omit its original operand".into());
            };
            if node.kind != "expr_cast"
                || node.expansion.is_some()
                || node.is_unsafe_block
                || node.dispatch.as_ref().is_none_or(|dispatch| {
                    dispatch.kind != "builtin"
                        || dispatch.op != "as"
                        || dispatch.operand_tys != ["u8"]
                        || dispatch.result_ty != "u16"
                })
                || !cast.attrs.is_empty()
                || !matches!(cast.ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u16"))
            {
                return Err(
                    "Digit arithmetic cast differs from the original concrete widening".into(),
                );
            }
            let syn::Expr::Paren(paren) = cast.expr.as_ref() else {
                return Err(
                    "Original widening cast must retain its parenthesized byte difference".into(),
                );
            };
            (paren.expr.as_ref(), child)
        } else {
            (source, node)
        };
        self.difference(source, node, digit)
    }

    fn difference(
        &self,
        source: &syn::Expr,
        node: &HirNode,
        digit: &semantics::Digit<'_>,
    ) -> Result<(), String> {
        let syn::Expr::Binary(binary) = source else {
            return Err("Digit arithmetic must retain its byte-minus-zero expression".into());
        };
        let [read, zero] = node.children.as_slice() else {
            return Err("Digit arithmetic must retain both original difference operands".into());
        };
        if node.kind != "expr_binary"
            || node.expansion.is_some()
            || !binary.attrs.is_empty()
            || !matches!(binary.op, syn::BinOp::Sub(_))
            || *read != *digit.read()
            || crate::data_aggregates::token(node)? != "u8"
            || !matches!(binary.right.as_ref(), syn::Expr::Lit(literal) if literal.attrs.is_empty()
                && matches!(&literal.lit, syn::Lit::Byte(byte) if byte.value() == b'0'))
            || zero.kind != "expr_lit"
            || !zero.children.is_empty()
            || zero.expansion.is_some()
            || zero.dispatch.is_some()
            || zero.is_unsafe_block
            || zero.res.is_some()
            || zero.lit.as_deref() != Some("b'0'")
            || zero.snippet != "b'0'"
            || crate::data_aggregates::token(zero)? != "u8"
            || node.dispatch.as_ref().is_none_or(|dispatch| {
                dispatch.kind != "builtin"
                    || dispatch.op != "-"
                    || dispatch.operand_tys != ["u8", "u8"]
                    || dispatch.result_ty != "u8"
            })
        {
            return Err(
                "Digit arithmetic initializer changes its checked byte read or zero subtraction"
                    .into(),
            );
        }
        crate::source_evidence::verify_child_span(node, read, binary.left.span(), self.input)?;

        crate::source_evidence::verify_child_span(node, zero, binary.right.span(), self.input)
    }

    fn inventory(&self, body: &HirNode, count: usize) -> Result<(), String> {
        let records = self
            .input
            .bindings
            .iter()
            .filter(|record| {
                record.decl_span.start >= body.span.start && record.decl_span.end <= body.span.end
            })
            .collect::<Vec<_>>();
        if records.len() != count
            || records.iter().any(|record| {
                !body.children[..count].iter().any(|statement| {
                    statement
                        .children
                        .first()
                        .is_some_and(|binding| binding.hir == record.hir)
                })
            })
        {
            return Err("Digit success block has missing or surplus local binding records".into());
        }
        Ok(())
    }
}

fn some(node: &HirNode, product: &HirNode, input: &dyn EvidenceFacts) -> Result<(), String> {
    let resolution = node.res.as_ref().ok_or("Missing original Some identity")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let registry = crate::data_library::registry_for(input).ok_or("Unpinned digit product Some")?;
    if node.kind != "expr_call"
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || resolution.kind != "resolved_option_variant"
        || resolution.target.is_some()
        || callee.path.replace("core::", "std::") != "std::prelude::v1::Some"
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get("std::prelude::v1::Some") != Some(&callee.definition_hash)
        || callee.implementation.is_some()
    {
        return Err("Digit product success must retain its exact pinned original Some".into());
    }

    source(node, product, input)
}

fn source(node: &HirNode, product: &HirNode, input: &dyn EvidenceFacts) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, input)?;
    let syn::Expr::Call(call) = syntax else {
        return Err("Digit product Some requires original call syntax".into());
    };
    if !call.attrs.is_empty()
        || call.args.len() != 1
        || !matches!(call.func.as_ref(), syn::Expr::Path(path) if path.attrs.is_empty()
            && path.qself.is_none()
            && crate::source_evidence::plain_relative_path(&path.path) == Some(vec!["Some".into()]))
        || crate::source_evidence::byte_range(&input.source.bytes, node.span)? != node.snippet
    {
        return Err(
            "Digit product Some must preserve its exact original payload expression".into(),
        );
    }

    crate::source_evidence::verify_child_span(node, product, call.args[0].span(), input)
}
