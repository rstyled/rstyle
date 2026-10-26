//! Whole original reader/product composition; no supported-prefix certification.
use super::super::wire_validators::source as shared;
use super::*;
use crate::{data_types::TypeToken, named_error_result_propagation as propagation};

pub(super) struct Checked<'a> {
    pub binding: crate::scalar_tuple_bindings::Checked<'a>,
    pub reader: shared::Reader<'a>,
    pub local: &'a HirNode,
    pub pattern: &'a HirNode,
    pub product: &'a HirNode,
    pub success: &'a HirNode,
    pub product_read: &'a HirNode,
    pub count_read: &'a HirNode,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }

    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;

    shared::syntax_interface(&syntax)?;
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    shared::statement_order(facts, &syntax)?;

    interface(facts, &syntax)?;

    let [declaration, local, success] = facts.body.children.as_slice() else {
        return Err(
            "Reader product requires exactly reader binding, product binding and success".into(),
        );
    };
    let binding = crate::scalar_tuple_bindings::verify(declaration, facts)?;
    shared::binding_shape(&binding)?;
    let reader = shared::reader_source_for(
        binding.initializer,
        facts,
        propagation::Mode::ReaderToCopyProduct,
    )?;

    let (pattern, product, ok) = product_source(local, &syntax, &reader, facts)?;
    for (position, field) in product.children.iter().enumerate() {
        bit_field(field, binding.entries[0].pattern, position, facts)?;
    }

    let (product_read, count_read) = success_source(success, &ok, facts)?;
    read(product_read, pattern)?;
    read(count_read, binding.entries[1].pattern)?;

    Ok(Some(Checked {
        binding,
        reader,
        local,
        pattern,
        product,
        success,
        product_read,
        count_read,
    }))
}

fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::source_evidence::initial_statement(facts, CAPABILITY, |first| {
        crate::scalar_tuple_bindings::candidate(first, facts)
            && first.children.last().is_some_and(|node| {
                propagation::mode_candidate(node, facts, propagation::Mode::ReaderToCopyProduct)
            })
    })
}

fn product_source<'a>(
    local: &'a HirNode,
    syntax: &syn::ItemFn,
    reader: &shared::Reader<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(&'a HirNode, &'a HirNode, TypeToken), String> {
    let [pattern, product] = local.children.as_slice() else {
        return Err(
            "Reader product local must contain its complete pattern and initializer".into(),
        );
    };
    immutable(pattern)?;
    local_source(syntax, pattern)?;

    if local.kind != "stmt_let" || product.kind != "expr_struct" || product.children.len() != 3 {
        return Err("Reader product requires one fresh three-field struct".into());
    }
    let TypeToken::Result { ok, .. } = crate::data_types::parse(&reader.propagation.output_token)?
    else {
        return Err("Missing product Result".into());
    };
    if !crate::reader_products::success(&ok, facts)? {
        return Err("Reader product lacks exact Copy/no-drop output ownership".into());
    }

    let TypeToken::Tuple(parts) = ok.as_ref() else {
        return Err("Missing complete product tuple".into());
    };
    if crate::data_aggregates::parsed_type(product)? != parts[0]
        || crate::data_aggregates::parsed_type(pattern)? != parts[0]
        || crate::data_structs::slots(product, facts)? != [Some(0), Some(1), Some(2)]
    {
        return Err("Fresh product changes its complete nominal type or field slots".into());
    }
    Ok((pattern, product, *ok))
}

fn success_source<'a>(
    success: &'a HirNode,
    ok: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(&'a HirNode, &'a HirNode), String> {
    let constructed = crate::concrete_results::verify_constructor(success, facts)?;
    let tuple = constructed.payload;
    let [product_read, count_read] = tuple.children.as_slice() else {
        return Err("Reader product success requires both original tuple components".into());
    };

    if constructed.construct != "data.result-ok"
        || tuple.kind != "expr_tup"
        || crate::data_aggregates::parsed_type(tuple)? != *ok
    {
        return Err("Reader product must return Ok((saved product, saved count))".into());
    }
    Ok((product_read, count_read))
}

fn interface(facts: &dyn EvidenceFacts, syntax: &syn::ItemFn) -> Result<(), String> {
    let signature = &facts.signature;
    if signature.params.len() != 1
        || syntax.sig.inputs.len() != 1
        || facts.bindings.len() != 4
        || signature.has_self
        || signature.is_async
        || signature.is_const
        || signature.is_unsafe
        || signature.is_variadic
        || signature.receiver.is_some()
        || signature.abi.value.as_deref() != Some("Rust")
        || facts.body.kind != "expr_block"
        || facts.body.expansion.is_some()
        || facts.body.is_unsafe_block
        || !matches!(
            syntax.block.stmts.as_slice(),
            [
                syn::Stmt::Local(_),
                syn::Stmt::Local(_),
                syn::Stmt::Expr(syn::Expr::Call(_), None)
            ]
        )
    {
        return Err(
            "Reader product requires its whole ordinary single-input three-statement source".into(),
        );
    }

    immutable(&signature.params[0].pat)?;
    if crate::data_aggregates::parsed_type(&signature.params[0].pat)?
        != TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
            "u8".into(),
        )))))
    {
        return Err("Reader product requires original shared bytes".into());
    }
    Ok(())
}

fn immutable(pattern: &HirNode) -> Result<(), String> {
    if pattern.kind != "pat_binding"
        || pattern.binding_mut != Some(false)
        || pattern.binding.is_none()
        || !pattern.children.is_empty()
    {
        return Err("Reader product requires an immutable plain binding".into());
    }
    Ok(())
}
fn read(node: &HirNode, binding: &HirNode) -> Result<(), String> {
    crate::source_evidence::verify_binding_read(
        node,
        binding,
        "Reader product changes its saved binding",
    )
}
fn binary<'a>(
    node: &'a HirNode,
    op: &str,
    ty: &str,
    facts: &dyn EvidenceFacts,
) -> Result<[&'a HirNode; 2], String> {
    let [left, right] = node.children.as_slice() else {
        return Err("Reader product binary loses operands".into());
    };
    if node.kind != "expr_binary"
        || node.expansion.is_some()
        || node.is_unsafe_block
        || crate::data_aggregates::token(node)? != ty
        || !node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && dispatch.op == op
                && dispatch.operand_tys == ["u32", if op == "<<" { "i32" } else { "u32" }]
                && dispatch.result_ty == ty
        })
    {
        return Err("Reader product changes its builtin scalar bit computation".into());
    }

    let original = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::Binary(expression) = crate::data_aggregates::ungroup(&original) else {
        return Err("Reader product binary loses its original operator source".into());
    };

    let source_op = match expression.op {
        syn::BinOp::Ne(_) => "!=",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::Shl(_) => "<<",
        _ => return Err("Reader product source uses an unregistered scalar operator".into()),
    };
    if source_op != op {
        return Err("Reader product observed operator differs from original source".into());
    }
    Ok([left, right])
}
fn literal(
    node: &HirNode,
    value: usize,
    ty: &str,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if node.kind != "expr_lit"
        || !node.children.is_empty()
        || node.dispatch.is_some()
        || node.expansion.is_some()
        || crate::data_aggregates::token(node)? != ty
    {
        return Err("Reader product requires original u32 bit literals".into());
    }
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(integer),
        ..
    }) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Reader product lost original scalar literal syntax".into());
    };
    if integer.base10_parse::<usize>().map_err(|e| e.to_string())? != value
        || node.lit.as_deref()
            != facts
                .source
                .bytes
                .get(node.span.start as usize..node.span.end as usize)
    {
        return Err("Reader product changes a selected bit or zero comparison".into());
    }
    Ok(())
}
fn bit_field(
    node: &HirNode,
    binding: &HirNode,
    position: usize,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let [masked, zero] = binary(node, "!=", "bool", facts)?;
    let [bits, mask] = binary(masked, "&", "u32", facts)?;
    let [one, shift] = binary(mask, "<<", "u32", facts)?;

    read(bits, binding)?;
    literal(zero, 0, "u32", facts)?;
    literal(one, 1, "u32", facts)?;
    literal(shift, position, "i32", facts)
}

fn local_source(syntax: &syn::ItemFn, pattern: &HirNode) -> Result<(), String> {
    let syn::Stmt::Local(local) = &syntax.block.stmts[1] else {
        return Err("Missing original product local".into());
    };
    let syn::Pat::Ident(binding) = &local.pat else {
        return Err("Product local source requires one plain identifier".into());
    };
    if !local.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || binding.ident != pattern.snippet
        || local.init.as_ref().is_none_or(|init| {
            init.diverge.is_some() || !matches!(init.expr.as_ref(), syn::Expr::Struct(_))
        })
    {
        return Err(
            "Product local source changes immutability or includes hidden control work".into(),
        );
    }
    Ok(())
}
