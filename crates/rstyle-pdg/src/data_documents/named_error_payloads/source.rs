//! Whole original arithmetic roots; summaries do not substitute for source proof.
use super::*;
use crate::source_evidence::verify_binding_read as formal_read;

use crate::data_types::TypeToken;
use syn::spanned::Spanned;

const BINDING_READ: &str = "Fresh arithmetic changes an exact original binding read";

pub(super) enum Checked<'a> {
    Constructor(Box<crate::named_error_payloads::Function<'a>>),
    Root(Root<'a>),
}

pub(super) struct Root<'a> {
    pub expression: &'a HirNode,
    pub option: crate::checked_options::Match<'a>,
    pub kind: crate::checked_lengths::Kind,
    pub ok: &'a HirNode,
    pub err: &'a HirNode,
    pub call: &'a HirNode,
    pub actual: &'a HirNode,
    pub contract: crate::copy_tagged_values::origins::WireContract,
}

pub(super) fn verify(facts: &dyn EvidenceFacts) -> Result<Option<Checked<'_>>, String> {
    if crate::named_error_payloads::candidate(facts) {
        return crate::named_error_payloads::verify_function(facts)
            .map(Box::new)
            .map(Checked::Constructor)
            .map(Some);
    }
    if !crate::extraction_profile::facts_support(facts, crate::named_error_payloads::CAPABILITY)
        || facts.signature.params.len() != 3
        || !facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"mixed_enum\""))
        || !matches!(facts.body.children.as_slice(), [tail] if tail.kind == "expr_match")
    {
        return Ok(None);
    }

    let expression = &facts.body.children[0];
    let Some(scrutinee) = expression.children.first() else {
        return Ok(None);
    };
    let Some(kind) = crate::checked_lengths::kind(scrutinee, facts) else {
        return Ok(None);
    };

    if !matches!(
        kind,
        crate::checked_lengths::Kind::Add | crate::checked_lengths::Kind::Mul
    ) {
        return Ok(None);
    }
    if crate::named_error_payloads::error_tree(facts)?.is_none() {
        return Ok(None);
    }

    interface(facts, expression)?;
    crate::checked_lengths::verify(scrutinee, kind, facts)?;
    let option = crate::checked_options::verify_match(expression, facts)?;
    if !option.scalar_value {
        return Err("Fresh arithmetic requires owned checked usize payload".into());
    }

    let [left, right] = scrutinee.children.as_slice() else {
        return Err("Fresh arithmetic must evaluate exactly two original scalar formals".into());
    };
    formal_read(left, &facts.signature.params[0].pat, BINDING_READ)?;
    formal_read(right, &facts.signature.params[1].pat, BINDING_READ)?;

    root(facts, expression, option, kind)
        .map(Checked::Root)
        .map(Some)
}

fn root<'a>(
    facts: &'a dyn EvidenceFacts,
    expression: &'a HirNode,
    option: crate::checked_options::Match<'a>,
    kind: crate::checked_lengths::Kind,
) -> Result<Root<'a>, String> {
    let ok = arm(expression, option.some_arm)?;
    let err = arm(expression, option.none_arm)?;
    let success = crate::concrete_results::verify_constructor(ok, facts)?;
    let failure = crate::concrete_results::verify_constructor(err, facts)?;

    if success.construct != "data.result-ok" || failure.construct != "data.result-err" {
        return Err("Fresh arithmetic must preserve Some/Ok and None/Err branches".into());
    }
    if [ok, err, expression].iter().any(|node| {
        node.ty.as_ref().and_then(|ty| ty.normalized.as_ref())
            != facts.signature.return_ty.normalized.as_ref()
    }) {
        return Err(
            "Fresh arithmetic branches change their complete original Result interface".into(),
        );
    }

    formal_read(success.payload, option.binding, BINDING_READ)?;
    let call = failure.payload;
    let [actual] = call.children.as_slice() else {
        return Err("Fresh arithmetic error helper requires exactly one static argument".into());
    };
    if call.kind != "expr_call" || call.expansion.is_some() || call.is_unsafe_block {
        return Err("Fresh arithmetic error helper must be an original direct call".into());
    }

    let formal = crate::named_error_payloads::static_formal(facts, 2)?;
    formal_read(actual, formal.pattern, BINDING_READ)?;
    let helper = crate::helper_calls::verify_use(call, facts)?;
    let contract = crate::helper_calls::named_errors::validate_shape(helper.summary())?.clone();

    if helper.summary().argument_types != [formal.token.text()?] {
        return Err("Fresh arithmetic helper changes its static argument interface".into());
    }

    Ok(Root {
        expression,
        option,
        kind,
        ok,
        err,
        call,
        actual,
        contract,
    })
}

fn arm(expression: &HirNode, index: usize) -> Result<&HirNode, String> {
    let arm = &expression.children[index + 1];
    let [_, body] = arm.children.as_slice() else {
        return Err("Fresh arithmetic arm contains omitted guard or operations".into());
    };
    Ok(body)
}

fn interface(facts: &dyn EvidenceFacts, expression: &HirNode) -> Result<(), String> {
    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    attributes(&syntax)?;

    if facts.body.kind != "expr_block"
        || facts.body.is_unsafe_block
        || facts.signature.has_self
        || facts.signature.is_async
        || facts.signature.is_unsafe
        || facts.signature.is_const
        || facts.signature.is_variadic
        || !facts.signature.generics.is_empty()
        || syntax.sig.constness.is_some()
        || syntax.sig.asyncness.is_some()
        || syntax.sig.unsafety.is_some()
        || syntax.sig.abi.is_some()
        || syntax.sig.variadic.is_some()
        || !syntax.sig.generics.params.is_empty()
        || syntax.sig.generics.where_clause.is_some()
        || syntax.sig.inputs.len() != 3
        || syntax.block.stmts.len() != 1
        || facts.body.expansion.is_some()
        || facts.body.dispatch.is_some()
        || facts.body.res.is_some()
    {
        return Err("Fresh arithmetic requires a complete ordinary three-formal root".into());
    }

    scalar_formals(facts, &syntax)?;
    let [syn::Stmt::Expr(syn::Expr::Match(tail), None)] = syntax.block.stmts.as_slice() else {
        return Err("Fresh arithmetic requires its entire original tail match".into());
    };
    let expected = crate::source_evidence::original_span(&facts.source.bytes, tail.span());
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, expression.span) {
        return Err("Fresh arithmetic whole-function tail correspondence disagrees".into());
    }
    Ok(())
}

fn attributes(syntax: &syn::ItemFn) -> Result<(), String> {
    use syn::visit::Visit;
    struct Attributes(bool);
    impl<'ast> Visit<'ast> for Attributes {
        fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
            self.0 = true;
        }
    }

    let mut nested = Attributes(false);
    nested.visit_block(&syntax.block);
    if nested.0 || !crate::source_evidence::literal_metadata(&syntax.attrs, "inline") {
        return Err(
            "Fresh arithmetic requires original unattributed operations and literal metadata"
                .into(),
        );
    }
    Ok(())
}

fn scalar_formals(facts: &dyn EvidenceFacts, syntax: &syn::ItemFn) -> Result<(), String> {
    for (position, argument) in syntax.sig.inputs.iter().take(2).enumerate() {
        let syn::FnArg::Typed(argument) = argument else {
            return Err("Scalar receiver unsupported".into());
        };
        let syn::Type::Path(ty) = argument.ty.as_ref() else {
            return Err("Scalar formal must be usize".into());
        };
        let syn::Pat::Ident(pattern) = argument.pat.as_ref() else {
            return Err("Scalar formal must be a plain binding".into());
        };
        let parameter = &facts.signature.params[position];
        if !argument.attrs.is_empty()
            || ty.qself.is_some()
            || !ty.path.is_ident("usize")
            || pattern.by_ref.is_some()
            || pattern.mutability.is_some()
            || pattern.subpat.is_some()
            || parameter.position != position as u32
            || parameter.pat.kind != "pat_binding"
            || parameter.pat.binding_mut != Some(false)
            || !parameter.pat.children.is_empty()
            || crate::data_aggregates::parsed_type(&parameter.pat)?
                != TypeToken::Scalar("usize".into())
        {
            return Err("Fresh arithmetic changes its exact scalar formals".into());
        }
    }
    Ok(())
}
