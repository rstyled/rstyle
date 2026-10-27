//! Whole original validator source, exact saved bindings and checked dependencies.
use super::*;
use crate::{data_types::TypeToken, named_error_result_propagation as propagation};
use syn::spanned::Spanned;

pub(super) struct Checked<'a> {
    pub binding: crate::scalar_tuple_bindings::Checked<'a>,
    pub reader: Reader<'a>,
    pub assertion: Assertion<'a>,
    pub branch: Branch<'a>,
    pub success: &'a HirNode,
    pub success_read: &'a HirNode,
}

pub(in crate::data_documents) struct Reader<'a> {
    pub question: &'a HirNode,
    pub propagation: propagation::Checked<'a>,
    pub slice: &'a HirNode,
    pub message: &'a HirNode,
    pub literal: crate::static_string_values::Descriptor,
    pub contract: crate::copy_tagged_values::origins::WireContract,
    pub dependency: serde_json::Value,
}

pub(super) struct Assertion<'a> {
    pub node: &'a HirNode,
    pub read: &'a HirNode,
    pub size: &'a HirNode,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if !candidate(facts) {
        return Ok(None);
    }
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    syntax_roles(&syntax)?;
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    interface(facts, &syntax)?;
    statement_order(facts, &syntax)?;

    let [declaration, assertion, conditional, success] = facts.body.children.as_slice() else {
        return Err("Wire validator requires its four complete original statements".into());
    };
    let binding = crate::scalar_tuple_bindings::verify(declaration, facts)?;
    binding_shape(&binding)?;
    let reader = reader_source(binding.initializer, facts)?;
    let assertion = assertion_source(
        statement(assertion, "stmt_semi", "expr_assert")?,
        &binding,
        facts,
    )?;
    let branch = branch(
        statement(conditional, "stmt_expr", "expr_if")?,
        &binding,
        facts,
    )?;
    let constructed = crate::concrete_results::verify_constructor(success, facts)?;
    if constructed.construct != "data.result-ok"
        || crate::data_aggregates::token(success)? != reader.propagation.output_token
    {
        return Err("Wire validator must return Ok(saved length)".into());
    }
    read(constructed.payload, binding.entries[1].pattern)?;

    Ok(Some(Checked {
        binding,
        reader,
        assertion,
        branch,
        success,
        success_read: constructed.payload,
    }))
}

fn reader_source<'a>(
    question: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Reader<'a>, String> {
    reader_source_for(question, facts, propagation::Mode::ReaderToScalar)
}

pub(in crate::data_documents) fn reader_source_for<'a>(
    question: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
    mode: propagation::Mode,
) -> Result<Reader<'a>, String> {
    let propagation = propagation::verify(question, facts)?;
    if propagation.mode != mode {
        return Err("Wire validator requires exact reader-to-scalar propagation".into());
    }
    let [slice, message] = propagation.operand.children.as_slice() else {
        return Err("Wire reader requires original slice and static message".into());
    };
    read(slice, &facts.signature.params[0].pat)?;
    let literal = crate::static_string_values::verify(message, facts)?;
    let (contract, dependency) = reader(propagation.operand, facts)?;

    Ok(Reader {
        question,
        propagation,
        slice,
        message,
        literal,
        contract,
        dependency,
    })
}

fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::source_evidence::initial_statement(facts, CAPABILITY, |first| {
        crate::scalar_tuple_bindings::candidate(first, facts)
            && first.children.last().is_some_and(|initializer| {
                propagation::mode_candidate(initializer, facts, propagation::Mode::ReaderToScalar)
            })
    })
}

pub(in crate::data_documents) fn binding_shape(
    binding: &crate::scalar_tuple_bindings::Checked<'_>,
) -> Result<(), String> {
    if !matches!(binding.entries.as_slice(), [first, second]
        if first.position == 0 && second.position == 1 && first.token == "u32" && second.token == "usize")
    {
        return Err("Wire validator requires saved (u32, usize) bindings in order".into());
    }
    Ok(())
}

fn statement<'a>(node: &'a HirNode, kind: &str, expression: &str) -> Result<&'a HirNode, String> {
    match node.children.as_slice() {
        [child] if node.kind == kind && child.kind == expression => Ok(child),
        _ => Err("Wire validator statement contains omitted or additional work".into()),
    }
}

fn interface(facts: &dyn EvidenceFacts, syntax: &syn::ItemFn) -> Result<(), String> {
    let signature = &facts.signature;
    if !matches!(signature.params.len(), 1 | 2)
        || syntax.sig.inputs.len() != signature.params.len()
        || facts.bindings.len() != signature.params.len() + 2
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
    {
        return Err("Wire validator requires its whole ordinary scalar/shared interface".into());
    }
    parameter_types(facts)
}

fn parameter_types(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let signature = &facts.signature;
    let expected = TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
        "u8".into(),
    )))));
    if crate::data_aggregates::parsed_type(&signature.params[0].pat)? != expected
        || signature.params.len() == 2
            && crate::data_aggregates::parsed_type(&signature.params[1].pat)?
                != TypeToken::Scalar("u32".into())
    {
        return Err(
            "Wire validator formals differ from shared bytes and optional expected u32".into(),
        );
    }

    for parameter in &signature.params {
        if parameter.pat.kind != "pat_binding"
            || parameter.pat.binding_mut != Some(false)
            || !parameter.pat.children.is_empty()
        {
            return Err("Wire validator requires complete immutable plain formals".into());
        }
    }
    Ok(())
}

pub(super) fn syntax_roles(function: &syn::ItemFn) -> Result<(), String> {
    syntax_interface(function)?;
    let [
        syn::Stmt::Local(local),
        syn::Stmt::Macro(assertion),
        syn::Stmt::Expr(syn::Expr::If(branch), None),
        syn::Stmt::Expr(syn::Expr::Call(_), None),
    ] = function.block.stmts.as_slice()
    else {
        return Err(
            "Wire validator requires declaration, assertion, mismatch branch and success tail"
                .into(),
        );
    };

    let Some(initializer) = &local.init else {
        return Err("Wire tuple must initialize once".into());
    };
    if initializer.diverge.is_some()
        || !matches!(initializer.expr.as_ref(), syn::Expr::Try(_))
        || assertion.semi_token.is_none()
        || crate::assertions::Assertion::from_macro(&assertion.mac)?.name != "assert_eq"
        || branch.else_branch.is_some()
        || !matches!(
            branch.then_branch.stmts.as_slice(),
            [syn::Stmt::Expr(syn::Expr::Return(_), Some(_))]
        )
    {
        return Err("Wire validator changes its complete source control topology".into());
    }
    Ok(())
}

pub(in crate::data_documents) fn syntax_interface(function: &syn::ItemFn) -> Result<(), String> {
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
    {
        return Err("Wire validator contains attributed or unsupported interface work".into());
    }
    Ok(())
}

fn assertion_source<'a>(
    node: &'a HirNode,
    binding: &crate::scalar_tuple_bindings::Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Assertion<'a>, String> {
    let [saved, size] = node.children.as_slice() else {
        return Err("Wire assertion requires both original operands".into());
    };
    crate::assertions::verify(
        node,
        &node.snippet,
        facts.build.debug_assertions.value,
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
    )?;
    if node.res.as_ref().map(|res| res.description.as_str()) != Some("core::assert_eq")
        || node.lit.as_deref() != Some("true")
    {
        return Err("Wire validator requires its active equality assertion".into());
    }
    read(saved, binding.entries[1].pattern)?;
    size_helper(size, facts)?;

    Ok(Assertion {
        node,
        read: saved,
        size,
    })
}

fn size_helper(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !node.children.is_empty() {
        return Err("Wire size helper cannot consume arguments".into());
    }

    let dependency = crate::helper_calls::verify_use(node, facts)?;
    let crate::helper_calls::VerifiedDependency::Shared(view) = dependency else {
        return Err("Wire size helper requires its selected original owner".into());
    };

    let callee = view.frontend();
    let [tail] = callee.body.children.as_slice() else {
        return Err("Wire size helper contains additional work".into());
    };
    if callee.body.kind != "expr_block"
        || !callee.signature.params.is_empty()
        || dependency_summary_size(view.summary())
        || crate::size_calls::semantics(tail, &callee)?
            != serde_json::json!({"rule":"scalar-size-in-bytes","argument":"u32","value":4})
    {
        return Err("Wire size helper must independently resolve size_of::<u32>()".into());
    }
    Ok(())
}

fn dependency_summary_size(summary: &crate::helper_calls::Summary) -> bool {
    summary.result_type != "usize"
        || !summary.argument_types.is_empty()
        || summary.mutates_arguments
        || summary.memory_effects
        || summary.may_panic
        || !summary.always_returns
}

fn reader(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<
    (
        crate::copy_tagged_values::origins::WireContract,
        serde_json::Value,
    ),
    String,
> {
    let dependency = crate::helper_calls::verify_use(node, facts)?;
    let summary = dependency.summary();
    if summary.identity != crate::helper_calls::guarded_effects::SUMMARY
        || summary.argument_types.len() != 2
        || summary.argument_types[0] != crate::data_aggregates::token(&node.children[0])?
        || summary.argument_types[1] != crate::data_aggregates::token(&node.children[1])?
        || summary.result_type != crate::data_aggregates::token(node)?
        || summary.mutates_arguments
        || !summary.memory_effects
        || summary
            .memory_reads
            .as_ref()
            .is_none_or(|reads| reads.is_empty() || reads.iter().any(|read| read.parameter != 0))
    {
        return Err("Wire reader lacks its verified guarded output and original byte reads".into());
    }

    let contract = crate::helper_calls::guarded_effects::validate_shape(summary)?.clone();
    error_route(&contract)?;
    Ok((contract, dependency.wire()?))
}

pub(super) fn error_route(
    contract: &crate::copy_tagged_values::origins::WireContract,
) -> Result<Vec<crate::copy_tagged_values::origins::Component>, String> {
    use crate::copy_tagged_values::origins::{CaseKind, Component};
    let errors: Vec<_> = contract
        .alternatives
        .iter()
        .filter(|alternative| alternative.output == CaseKind::ResultErr)
        .collect();
    let [alternative] = errors.as_slice() else {
        return Err("Wire reader requires one complete error alternative".into());
    };

    let [leaf] = alternative.leaves.as_slice() else {
        return Err("Wire reader requires its one carried static message".into());
    };
    if leaf.root.parameter != 1
        || leaf.input != [Component::SharedReferent]
        || leaf.output.first() != Some(&Component::ResultErr)
        || leaf.output.last() != Some(&Component::SharedReferent)
        || leaf.guard.is_some()
    {
        return Err("Wire reader changes its message component route".into());
    }
    Ok(leaf.output.clone())
}

pub(super) struct Branch<'a> {
    pub condition: &'a HirNode,
    pub compared: [&'a HirNode; 2],
    pub mismatch: &'a HirNode,
    pub arguments: [&'a HirNode; 2],
    pub failure: &'a HirNode,
    pub returned: &'a HirNode,
}

fn branch<'a>(
    node: &'a HirNode,
    binding: &crate::scalar_tuple_bindings::Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Branch<'a>, String> {
    let [condition, body] = node.children.as_slice() else {
        return Err("Wire mismatch requires a sole returning true branch".into());
    };
    let [found, expected] = condition.children.as_slice() else {
        return Err("Wire mismatch requires both scalar operands".into());
    };
    if condition.kind != "expr_binary"
        || !condition.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && dispatch.op == "!="
                && dispatch.operand_tys == ["u32", "u32"]
                && dispatch.result_ty == "bool"
        })
    {
        return Err("Wire mismatch must compare scalar inequality".into());
    }
    read(found, binding.entries[0].pattern)?;
    expected_value(expected, facts)?;
    let failure = failure(body, facts)?;
    let mismatch = mismatch(failure.value, expected, binding, facts)?;

    Ok(Branch {
        condition,
        compared: [found, expected],
        mismatch: mismatch.call,
        arguments: mismatch.arguments,
        failure: failure.value,
        returned: failure.returned,
    })
}

struct Failure<'a> {
    value: &'a HirNode,
    returned: &'a HirNode,
}

fn failure<'a>(body: &'a HirNode, facts: &dyn EvidenceFacts) -> Result<Failure<'a>, String> {
    if body.kind != "expr_block" || body.expansion.is_some() || body.is_unsafe_block {
        return Err("Wire mismatch requires its original ordinary block".into());
    }
    let [returned_statement] = body.children.as_slice() else {
        return Err("Wire mismatch branch contains extra work".into());
    };

    let returned = statement(returned_statement, "stmt_semi", "expr_ret")?;
    let [value] = returned.children.as_slice() else {
        return Err("Wire mismatch return loses its complete value".into());
    };
    if value.ty.as_ref().and_then(|ty| ty.normalized.as_ref())
        != facts.signature.return_ty.normalized.as_ref()
    {
        return Err("Wire mismatch changes its complete Result return type".into());
    }
    Ok(Failure { value, returned })
}

struct Mismatch<'a> {
    call: &'a HirNode,
    arguments: [&'a HirNode; 2],
}

fn mismatch<'a>(
    failure: &'a HirNode,
    expected: &HirNode,
    binding: &crate::scalar_tuple_bindings::Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Mismatch<'a>, String> {
    let constructed = crate::concrete_results::verify_constructor(failure, facts)?;
    if constructed.construct != "data.result-err" {
        return Err("Wire mismatch must return Err".into());
    }

    let mismatch = constructed.payload;
    let [actual_expected, actual_found] = mismatch.children.as_slice() else {
        return Err("Wire mismatch helper requires both scalar arguments".into());
    };
    same_expected(expected, actual_expected, facts)?;
    read(actual_found, binding.entries[0].pattern)?;

    let dependency = crate::helper_calls::verify_use(mismatch, facts)?;
    crate::helper_calls::named_errors::validate_shape(dependency.summary())?;
    if dependency.summary().argument_types != ["u32", "u32"] {
        return Err("Wire mismatch helper changes exact two-u32 constructor interface".into());
    }

    Ok(Mismatch {
        call: mismatch,
        arguments: [actual_expected, actual_found],
    })
}

fn expected_value(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if facts.signature.params.len() == 2 {
        return read(node, &facts.signature.params[1].pat);
    }
    scalar_literal(node, facts).map(|_| ())
}

fn same_expected(
    expected: &HirNode,
    actual: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    expected_value(actual, facts)?;
    if facts.signature.params.len() == 1
        && scalar_literal(expected, facts)? != scalar_literal(actual, facts)?
    {
        return Err("Wire mismatch constructor changes the compared expected literal".into());
    }
    Ok(())
}

fn scalar_literal(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<u32, String> {
    if node.kind != "expr_lit" || crate::data_aggregates::token(node)? != "u32" {
        return Err("Wire expected value must be an original builtin-u32 literal".into());
    }
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(value),
        ..
    }) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Wire expected value lost original literal syntax".into());
    };
    value.base10_parse().map_err(|error| error.to_string())
}

fn read(node: &HirNode, binding: &HirNode) -> Result<(), String> {
    crate::source_evidence::verify_binding_read(
        node,
        binding,
        "Wire validator changes its exact saved binding",
    )
}

pub(in crate::data_documents) fn statement_order(
    facts: &dyn EvidenceFacts,
    syntax: &syn::ItemFn,
) -> Result<(), String> {
    if syntax.block.stmts.len() != facts.body.children.len() {
        return Err("Wire validator cannot omit original statements".into());
    }
    for (original, observed) in syntax.block.stmts.iter().zip(&facts.body.children) {
        let expected = statement_span(original, observed, facts)?;
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, observed.span)
        {
            return Err("Wire validator changes original statement order or complete spans".into());
        }
    }
    Ok(())
}

fn statement_span(
    original: &syn::Stmt,
    observed: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<crate::ByteSpan, String> {
    if let syn::Stmt::Macro(assertion) = original {
        if assertion.semi_token.is_none()
            || observed.kind != "stmt_semi"
            || !matches!(observed.children.as_slice(), [child] if child.kind == "expr_assert" && child.span == observed.span)
            || crate::assertions::Assertion::from_macro(&assertion.mac)?.name != "assert_eq"
        {
            return Err(
                "Wire assertion statement loses its exact invocation and terminator".into(),
            );
        }
        return Ok(crate::source_evidence::original_span(
            &facts.source.bytes,
            assertion.mac.span(),
        ));
    }
    Ok(crate::source_evidence::original_span(
        &facts.source.bytes,
        original.span(),
    ))
}
