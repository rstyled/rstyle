//! Complete original-source roles joined to direct captured HIR children.
use crate::{HirNode, ParamFact, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

#[cfg(test)]
mod tests;

pub(crate) struct Roles<'a> {
    pub source: &'a HirNode,
    pub reader: &'a HirNode,
    pub continuation: &'a HirNode,
    pub payload: &'a HirNode,
    pub decoded: &'a HirNode,
    pub count: &'a HirNode,
    pub map: &'a HirNode,
    pub success: &'a HirNode,
    pub converted: &'a HirNode,
    pub conversion: &'a HirNode,
    pub decoded_read: &'a HirNode,
    pub tuple: &'a HirNode,
    pub value_read: &'a HirNode,
    pub count_read: &'a HirNode,
    pub map_err: &'a HirNode,
    pub failure: &'a HirNode,
    pub wildcard: &'a HirNode,
    pub constructor: &'a HirNode,
    pub message_read: &'a HirNode,
    pub slice: &'a ParamFact,
    pub message: &'a ParamFact,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Roles<'_>, String> {
    let function =
        syn::parse_str(&facts.source.bytes).map_err(|error: syn::Error| error.to_string())?;

    let syntax = super::source::syntax(&function)?;
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    facts.nominal_resolution()?;

    let [source] = children(&facts.body, "expr_block")?;

    let [reader, continuation] = children(source, "expr_method_call")?;

    let [payload, body] = children(continuation, "expr_closure")?;

    let [decoded, count] = children(payload, "pat_scalar_tuple")?;

    let [map_err] = children(body, "expr_block")?;

    let [map, failure] = children(map_err, "expr_method_call")?;

    let [conversion, success] = children(map, "expr_method_call")?;

    let [converted, tuple] = children(success, "expr_closure")?;

    let [value_read, count_read] = children(tuple, "expr_tup")?;

    let [decoded_read] = children(conversion, "expr_call")?;

    let [wildcard, constructor] = children(failure, "expr_closure")?;

    let [message_read] = children(constructor, "expr_call")?;

    let [slice, message] = interface(facts)?;

    let roles = Roles {
        source,
        reader,
        continuation,
        payload,
        decoded,
        count,
        map,
        success,
        converted,
        conversion,
        decoded_read,
        tuple,
        value_read,
        count_read,
        map_err,
        failure,
        wildcard,
        constructor,
        message_read,
        slice,
        message,
    };
    spans(&roles, &syntax, facts)?;
    bindings(&roles, facts)?;
    reads(&roles)?;
    types(&roles, body, facts)?;

    Ok(roles)
}

fn children<'a, const N: usize>(node: &'a HirNode, kind: &str) -> Result<&'a [HirNode; N], String> {
    if node.kind != kind || node.expansion.is_some() || node.is_unsafe_block {
        return Err("Widening reader changes an original complete HIR role".into());
    }
    node.children
        .as_slice()
        .try_into()
        .map_err(|_| "Widening reader has additional or missing HIR work".into())
}

fn interface(facts: &dyn EvidenceFacts) -> Result<[&ParamFact; 2], String> {
    let signature = &facts.signature;

    let [slice, message] = signature.params.as_slice() else {
        return Err("Widening reader requires exactly two complete formals".into());
    };

    if signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_const
        || signature.is_unsafe
        || signature.is_variadic
        || signature.abi.value.as_deref() != Some("Rust")
        || !crate::source_evidence::implicit_input_lifetime(facts)
    {
        return Err("Widening reader changes its ordinary concrete interface".into());
    }

    let bytes = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(scalar("u8")))));
    typed(&slice.pat, &bytes)?;
    crate::named_error_payloads::static_formal(facts, 1)?;

    Ok([slice, message])
}

fn spans(
    roles: &Roles<'_>,
    syntax: &super::source::Syntax<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let [slice_read, message_read] = children(roles.reader, "expr_call")?;

    let pairs = [
        (syntax.and_then.span(), roles.source),
        (syntax.reader.span(), roles.reader),
        (syntax.reader.args[0].span(), slice_read),
        (syntax.reader.args[1].span(), message_read),
        (syntax.continuation.span(), roles.continuation),
        (syntax.continuation.inputs[0].span(), roles.payload),
        (
            syntax.continuation.body.span(),
            &roles.continuation.children[1],
        ),
        (syntax.decoded.span(), roles.decoded),
        (syntax.count.span(), roles.count),
        (syntax.map.span(), roles.map),
        (syntax.success.span(), roles.success),
        (syntax.converted.span(), roles.converted),
        (syntax.conversion.span(), roles.conversion),
        (syntax.conversion.args[0].span(), roles.decoded_read),
        (syntax.tuple.span(), roles.tuple),
        (syntax.tuple.elems[0].span(), roles.value_read),
        (syntax.tuple.elems[1].span(), roles.count_read),
        (syntax.map_err.span(), roles.map_err),
        (syntax.failure.span(), roles.failure),
        (syntax.failure.inputs[0].span(), roles.wildcard),
        (syntax.constructor.span(), roles.constructor),
        (syntax.constructor.args[0].span(), roles.message_read),
        (syntax.formals[0].span(), &roles.slice.pat),
        (syntax.formals[1].span(), &roles.message.pat),
    ];

    for (span, node) in pairs {
        let span = crate::source_evidence::original_span(&facts.source.bytes, span);
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, span, node.span) {
            return Err("Widening reader source/HIR role span differs".into());
        }
    }

    Ok(())
}

fn bindings(roles: &Roles<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let roster = [
        &roles.slice.pat,
        &roles.message.pat,
        roles.decoded,
        roles.count,
        roles.converted,
    ];

    if facts.bindings.len() != roster.len() {
        return Err("Widening reader binding inventory has additional or missing work".into());
    }

    for (index, node) in roster.into_iter().enumerate() {
        crate::source_evidence::plain_immutable_binding(node, "Invalid widening binding")?;
        let record = facts
            .bindings
            .iter()
            .find(|record| record.hir == node.hir)
            .ok_or("Missing widening reader binding record")?;
        if record.is_mut
            || record.is_param != (index < 2)
            || record.decl_span != node.span
            || Some(&record.symbol) != node.binding.as_ref()
            || node
                .ty
                .as_ref()
                .is_none_or(|ty| ty.normalized != record.ty.normalized)
            || node.expansion.is_some()
            || node.is_unsafe_block
            || node.dispatch.is_some()
        {
            return Err("Widening reader binding record differs from its original role".into());
        }
        binding_scope(roles, index, &record.scope, node)?;
    }

    formal_records(roles, facts)?;

    if roles.wildcard.kind != "pat_wild"
        || !roles.wildcard.children.is_empty()
        || roles.wildcard.binding.is_some()
        || roles.wildcard.param_index.is_some()
        || roles.wildcard.expansion.is_some()
        || roles.wildcard.is_unsafe_block
        || roles.wildcard.dispatch.is_some()
    {
        return Err("Widening conversion error callback changes its whole wildcard".into());
    }

    Ok(())
}

fn formal_records(roles: &Roles<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let expected = format!("scope:source:fn:0:{}", facts.source.bytes.len());

    for (index, formal) in [roles.slice, roles.message].into_iter().enumerate() {
        let record = facts
            .bindings
            .iter()
            .find(|record| record.hir == formal.pat.hir)
            .ok_or("Missing widening original formal record")?;
        if record.scope != expected
            || formal.position != index as u32
            || formal.symbol != record.symbol
            || formal.name_span != formal.pat.span
            || formal.ty.normalized != record.ty.normalized
        {
            return Err("Widening reader formal changes its source position, type or scope".into());
        }
    }

    Ok(())
}

fn binding_scope(
    roles: &Roles<'_>,
    index: usize,
    scope: &str,
    node: &HirNode,
) -> Result<(), String> {
    if index < 2 {
        if node.param_index != Some(index as u32) {
            return Err("Widening reader formal position differs".into());
        }
        return Ok(());
    }

    let closure = if index == 4 {
        roles.success
    } else {
        roles.continuation
    };

    let expected = format!(
        "scope:source:callback:{}:{}",
        closure.span.start, closure.span.end
    );

    if scope != expected || node.param_index.is_some() {
        return Err("Widening callback binding changes its original lexical scope".into());
    }

    Ok(())
}

fn reads(roles: &Roles<'_>) -> Result<(), String> {
    let [slice, message] = children(roles.reader, "expr_call")?;

    for (read, binding) in [
        (slice, &roles.slice.pat),
        (message, &roles.message.pat),
        (roles.decoded_read, roles.decoded),
        (roles.value_read, roles.converted),
        (roles.count_read, roles.count),
        (roles.message_read, &roles.message.pat),
    ] {
        crate::source_evidence::verify_binding_read(
            read,
            binding,
            "Widening reader changes a saved binding read",
        )?;
    }

    Ok(())
}

fn scalar(name: &str) -> TypeToken {
    TypeToken::Scalar(name.into())
}

fn typed(node: &HirNode, expected: &TypeToken) -> Result<(), String> {
    if crate::data_aggregates::parsed_type(node)? != *expected {
        return Err("Widening reader role has a different concrete type".into());
    }

    Ok(())
}

fn types(roles: &Roles<'_>, body: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let output = crate::data_aggregates::parsed_type(roles.source)?;

    let TypeToken::Result { ok, err } = &output else {
        return Err("Widening reader lacks its complete final Result".into());
    };

    let tuple = TypeToken::Tuple(vec![scalar("usize"); 2]);

    if **ok != tuple {
        return Err("Widening reader changes the complete usize/count result".into());
    }
    typed(&facts.body, &output)?;
    typed(body, &output)?;
    typed(roles.map_err, &output)?;
    typed(roles.constructor, err)?;
    typed(roles.tuple, &tuple)?;
    continuation_types(roles, err)?;
    typed(
        roles.map,
        &TypeToken::MappedIntConversionResult {
            ok: Box::new(tuple),
        },
    )?;

    if facts
        .signature
        .return_ty
        .normalized
        .as_deref()
        .map(crate::data_types::parse)
        .transpose()?
        != Some(output)
    {
        return Err("Widening reader signature differs from its complete returned Result".into());
    }

    Ok(())
}

fn continuation_types(roles: &Roles<'_>, err: &TypeToken) -> Result<(), String> {
    typed(roles.decoded, &scalar("u32"))?;
    typed(roles.count, &scalar("usize"))?;
    typed(roles.converted, &scalar("usize"))?;
    typed(
        roles.payload,
        &TypeToken::Tuple(vec![scalar("u32"), scalar("usize")]),
    )?;
    typed(
        roles.reader,
        &TypeToken::Result {
            ok: Box::new(TypeToken::Tuple(vec![scalar("u32"), scalar("usize")])),
            err: Box::new(err.clone()),
        },
    )?;
    typed(
        roles.conversion,
        &TypeToken::IntConversionResult("usize".into()),
    )?;

    Ok(())
}
