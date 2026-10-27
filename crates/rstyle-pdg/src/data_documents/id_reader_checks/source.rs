//! Original short-input check and complete tail call; no bounds proof erases reader panics.
use super::*;
use crate::{copy_tagged_values::origins::WireContract, data_types::TypeToken};

pub(super) struct Checked<'a> {
    pub condition: &'a HirNode,
    pub length: &'a HirNode,
    pub slice_read: &'a HirNode,
    pub width: &'a HirNode,
    pub error_return: &'a HirNode,
    pub error_result: &'a HirNode,
    pub constructor: &'a HirNode,
    pub message: &'a HirNode,
    pub error_contract: WireContract,
    pub error_dependency: serde_json::Value,
    pub reader: &'a HirNode,
    pub reader_actuals: [&'a HirNode; 2],
    pub reader_contract: WireContract,
    pub reader_dependency: serde_json::Value,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }

    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    syntax_shape(&syntax)?;
    facts.verify_source_inputs()?;

    crate::source_evidence::verify(facts)?;

    super::super::wire_validators::source::statement_order(facts, &syntax)?;
    interface(facts)?;

    let [statement, reader] = facts.body.children.as_slice() else {
        return Err("Checked ID reader requires exactly one precheck and one tail call".into());
    };

    let [branch] = statement.children.as_slice() else {
        return Err("Checked ID reader precheck cannot hide extra work".into());
    };

    let (condition, length, slice_read, width, body) = precheck(branch, facts)?;

    let (error_return, error_result, constructor, message) = error(body, facts)?;

    let (error_contract, error_dependency) = error_summary(constructor, facts)?;

    let (reader_actuals, reader_contract, reader_dependency) = reader_summary(reader, facts)?;

    Ok(Some(Checked {
        condition,
        length,
        slice_read,
        width,
        error_return,
        error_result,
        constructor,
        message,
        error_contract,
        error_dependency,
        reader,
        reader_actuals,
        reader_contract,
        reader_dependency,
    }))
}

fn syntax_shape(syntax: &syn::ItemFn) -> Result<(), String> {
    super::super::wire_validators::source::syntax_interface(syntax)?;

    let [
        syn::Stmt::Expr(syn::Expr::If(branch), None),
        syn::Stmt::Expr(syn::Expr::Call(_), None),
    ] = syntax.block.stmts.as_slice()
    else {
        return Err(
            "Checked ID reader requires its original precheck then direct tail call".into(),
        );
    };

    if branch.else_branch.is_some()
        || !matches!(
            branch.then_branch.stmts.as_slice(),
            [syn::Stmt::Expr(syn::Expr::Return(_), Some(_))]
        )
    {
        return Err(
            "Checked ID reader requires its sole early error without hidden branch work".into(),
        );
    }

    Ok(())
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
        || !super::super::fallible_readers::source::implicit_slice_lifetime(facts)
        || facts.body.kind != "expr_block"
        || facts.body.expansion.is_some()
        || facts.body.is_unsafe_block
        || facts
            .bindings
            .iter()
            .any(|binding| !binding.is_param || binding.is_mut)
    {
        return Err("Checked ID reader requires its whole immutable two-formal interface".into());
    }

    let slice = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));

    if crate::data_aggregates::parsed_type(&signature.params[0].pat)? != slice {
        return Err("Checked ID reader requires shared input bytes".into());
    }

    crate::named_error_payloads::static_formal(facts, 1)?;

    let TypeToken::Result { ok, .. } = crate::data_types::parse(
        signature
            .return_ty
            .normalized
            .as_deref()
            .ok_or("Missing checked ID reader result")?,
    )?
    else {
        return Err("Checked ID reader requires its complete Result".into());
    };

    if !crate::id_reader_types::success(&ok, facts)?
        || crate::named_error_payloads::error_tree(facts)?.is_none()
    {
        return Err("Checked ID reader lacks complete success/error cleanup authority".into());
    }

    Ok(())
}

fn precheck<'a>(
    branch: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<
    (
        &'a HirNode,
        &'a HirNode,
        &'a HirNode,
        &'a HirNode,
        &'a HirNode,
    ),
    String,
> {
    let [condition, body] = branch.children.as_slice() else {
        return Err("Checked ID reader requires one precheck without else".into());
    };

    let [length, width] = condition.children.as_slice() else {
        return Err("Checked ID reader requires length then exact width".into());
    };

    let [slice] = length.children.as_slice() else {
        return Err("Checked ID reader length requires its sole slice receiver".into());
    };

    if condition.kind != "expr_binary"
        || condition.expansion.is_some()
        || !condition.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && dispatch.op == "<"
                && dispatch.operand_tys == ["usize", "usize"]
                && dispatch.result_ty == "bool"
        })
        || length.kind != "expr_method_call"
        || crate::data_aggregates::token(length)? != "usize"
        || !crate::data_library::source_resolved(length, facts)?
        || facts
            .nominal_resolution()?
            .verify_id_reader_constant(width, facts)?
            != 4
    {
        return Err(
            "Checked ID reader requires exact original byte length < declared ID width".into(),
        );
    }
    formal(slice, 0, facts)?;

    Ok((condition, length, slice, width, body))
}

fn error<'a>(
    body: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode, &'a HirNode), String> {
    let [statement] = body.children.as_slice() else {
        return Err("Checked ID reader error block must contain its sole return".into());
    };

    let [returned] = statement.children.as_slice() else {
        return Err("Checked ID reader short branch contains extra work".into());
    };

    let [result] = returned.children.as_slice() else {
        return Err("Checked ID reader must return exactly its error Result".into());
    };

    if body.kind != "expr_block" || statement.kind != "stmt_semi" || returned.kind != "expr_ret" {
        return Err("Checked ID reader changes the short-input return topology".into());
    }

    let constructed = crate::concrete_results::verify_constructor(result, facts)?;

    let constructor = constructed.payload;

    let [message] = constructor.children.as_slice() else {
        return Err("Checked ID reader error requires its sole static-message actual".into());
    };

    if constructed.construct != "data.result-err" || constructor.kind != "expr_call" {
        return Err("Checked ID reader short branch must construct its exact error".into());
    }
    formal(message, 1, facts)?;

    Ok((returned, result, constructor, message))
}

fn error_summary(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(WireContract, serde_json::Value), String> {
    let dependency = crate::helper_calls::verify_use(node, facts)?;

    let contract = crate::helper_calls::named_errors::validate_shape(dependency.summary())?;

    if dependency.summary().identity != crate::helper_calls::named_errors::SUMMARY
        || dependency.summary().argument_types
            != [crate::data_aggregates::token(&node.children[0])?]
        || dependency.summary().result_type != crate::data_aggregates::token(node)?
    {
        return Err("Checked ID reader short branch changes its proved static constructor".into());
    }

    let crate::helper_calls::VerifiedDependency::Shared(view) = &dependency else {
        return Err("Checked ID reader error requires its complete selected original owner".into());
    };

    let callee = view.frontend();

    let constructor = crate::named_error_payloads::verify_function(&callee)?;

    if !matches!(constructor.variant, crate::named_error_payloads::Constructor::Named { variant, .. }
        if variant == "BufferTooSmall")
    {
        return Err(
            "Checked ID reader must preserve the actual BufferTooSmall error variant".into(),
        );
    }

    Ok((contract.clone(), dependency.wire()?))
}

fn reader_summary<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<([&'a HirNode; 2], WireContract, serde_json::Value), String> {
    let [slice, message] = node.children.as_slice() else {
        return Err("Checked ID reader tail requires its exact slice and message actuals".into());
    };
    formal(slice, 0, facts)?;
    formal(message, 1, facts)?;

    let dependency = crate::helper_calls::verify_use(node, facts)?;

    let contract = crate::helper_calls::id_readers::validate_shape(dependency.summary())?;

    if node.kind != "expr_call"
        || node.expansion.is_some()
        || dependency.summary().result_type
            != facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .ok_or("Missing checked ID reader return type")?
    {
        return Err("Checked ID reader tail changes its complete selected decoder reader".into());
    }

    Ok(([slice, message], contract.clone(), dependency.wire()?))
}

use super::super::fallible_readers::source::formal;

#[cfg(test)]
mod tests;
