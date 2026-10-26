//! Complete ordinary reader source, original binding reads and selected dependencies.
use super::*;
use crate::{copy_tagged_values::origins::WireContract, data_types::TypeToken};
use syn::spanned::Spanned;

pub(super) struct Checked<'a> {
    pub question: &'a HirNode,
    pub propagation: propagation::Checked<'a>,
    pub length_arguments: [&'a HirNode; 3],
    pub length_contract: WireContract,
    pub length_dependency: serde_json::Value,
    pub success: &'a HirNode,
    pub tuple: &'a HirNode,
    pub reader: &'a HirNode,
    pub reader_argument: &'a HirNode,
    pub consumed: &'a HirNode,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }

    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    let roles = syntax_roles(&syntax)?;
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    interface(facts)?;

    let [statement, success] = facts.body.children.as_slice() else {
        return Err("Reader requires one complete precheck statement and one success tail".into());
    };
    let [question] = statement.children.as_slice() else {
        return Err("Reader precheck has missing or extra original work".into());
    };
    if statement.kind != "stmt_semi" || facts.body.kind != "expr_block" {
        return Err("Reader changes its complete ordinary body shape".into());
    }

    bind(facts, roles[0], question)?;
    bind(facts, roles[1], success)?;
    let propagation = propagation::verify(question, facts)?;
    let [slice, minimum, message] = propagation.operand.children.as_slice() else {
        return Err("Reader precheck requires all three original arguments".into());
    };

    formal(slice, 0, facts)?;
    formal(message, 1, facts)?;
    size(minimum, facts)?;

    let helper = crate::helper_calls::verify_use(propagation.operand, facts)?;
    let length_contract =
        crate::helper_calls::slice_lengths::validate_shape(helper.summary())?.clone();
    let length_dependency = helper.wire()?;
    let success_roles = success_roles(success, facts)?;

    Ok(Some(Checked {
        question,
        propagation,
        length_arguments: [slice, minimum, message],
        length_contract,
        length_dependency,
        success,
        tuple: success_roles.tuple,
        reader: success_roles.reader,
        reader_argument: success_roles.reader_argument,
        consumed: success_roles.consumed,
    }))
}

struct Success<'a> {
    tuple: &'a HirNode,
    reader: &'a HirNode,
    reader_argument: &'a HirNode,
    consumed: &'a HirNode,
}

fn success_roles<'a>(
    success: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Success<'a>, String> {
    let constructed = crate::concrete_results::verify_constructor(success, facts)?;
    if constructed.construct != "data.result-ok" {
        return Err("Reader tail must construct exactly its successful Result".into());
    }

    let tuple = constructed.payload;
    let [reader, consumed] = tuple.children.as_slice() else {
        return Err("Reader success requires exact native value and consumed length".into());
    };
    if tuple.kind != "expr_tup" {
        return Err("Reader success cannot change product representation".into());
    }

    let [reader_argument] = reader.children.as_slice() else {
        return Err("Reader native helper requires exactly one saved slice".into());
    };
    formal(reader_argument, 0, facts)?;
    size(consumed, facts)?;
    native::verify(reader, facts)?;
    Ok(Success {
        tuple,
        reader,
        reader_argument,
        consumed,
    })
}

fn interface(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let signature = &facts.signature;
    if signature.params.len() != 2
        || facts.bindings.len() != 2
        || signature.has_self
        || signature.receiver.is_some()
        || signature.is_async
        || signature.is_unsafe
        || signature.is_const
        || signature.is_variadic
        || signature.abi.value.as_deref() != Some("Rust")
        || !implicit_slice_lifetime(facts)
        || facts.body.expansion.is_some()
        || facts.body.is_unsafe_block
        || facts
            .bindings
            .iter()
            .any(|binding| !binding.is_param || binding.is_mut)
    {
        return Err("Reader requires its whole ordinary two-formal interface".into());
    }

    let slice = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));
    if crate::data_aggregates::parsed_type(&signature.params[0].pat)? != slice {
        return Err("Reader slice must be exactly shared bytes".into());
    }
    crate::named_error_payloads::static_formal(facts, 1)?;
    Ok(())
}

pub(in crate::data_documents) fn implicit_slice_lifetime(facts: &dyn EvidenceFacts) -> bool {
    matches!(facts.signature.generics.as_slice(), [lifetime]
        if lifetime.name == "'_"
            && facts.signature.params.first().is_some_and(|parameter|
                lifetime.span.start == parameter.ty_span.start
                    && lifetime.span.end.checked_sub(lifetime.span.start) == Some(1)))
}

pub(in crate::data_documents) fn formal(
    node: &HirNode,
    position: usize,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let parameter = facts
        .signature
        .params
        .get(position)
        .ok_or("Missing original reader formal")?;
    crate::source_evidence::verify_binding_read(
        node,
        &parameter.pat,
        "Reader argument changes its exact original formal binding",
    )
}

pub(super) fn size(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if crate::size_calls::semantics(node, facts)?
        != serde_json::json!({
            "rule":"scalar-size-in-bytes", "argument":"u32", "value":4,
        })
    {
        return Err("Reader requires independently resolved size_of::<u32>()".into());
    }
    Ok(())
}

fn bind(facts: &dyn EvidenceFacts, syntax: &syn::Expr, node: &HirNode) -> Result<(), String> {
    let expected = crate::source_evidence::original_span(&facts.source.bytes, syntax.span());
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, node.span) {
        return Err("Reader source/HIR whole expression correspondence disagrees".into());
    }
    Ok(())
}

pub(super) fn syntax_roles(function: &syn::ItemFn) -> Result<[&syn::Expr; 2], String> {
    let signature = &function.sig;
    if !crate::source_evidence::literal_metadata(&function.attrs, "inline")
        || crate::source_evidence::has_body_attributes(&function.block)
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 2
    {
        return Err("Reader source contains unsupported interface or attributed work".into());
    }

    for argument in &signature.inputs {
        let syn::FnArg::Typed(argument) = argument else {
            return Err("Reader source cannot contain a receiver".into());
        };
        let syn::Pat::Ident(pattern) = argument.pat.as_ref() else {
            return Err("Reader source requires complete plain formal bindings".into());
        };
        if !argument.attrs.is_empty()
            || !pattern.attrs.is_empty()
            || pattern.by_ref.is_some()
            || pattern.mutability.is_some()
            || pattern.subpat.is_some()
        {
            return Err("Reader source cannot hide attributed or mutable formal work".into());
        }
    }

    let [
        syn::Stmt::Expr(question @ syn::Expr::Try(_), Some(_)),
        syn::Stmt::Expr(success @ syn::Expr::Call(_), None),
    ] = function.block.stmts.as_slice()
    else {
        return Err(
            "Reader source must contain exactly one question-mark statement and tail".into(),
        );
    };
    Ok([question, success])
}
