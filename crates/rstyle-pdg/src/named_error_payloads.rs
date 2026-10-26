//! Fresh named-error source/value receipts. No registration or graph authority.
mod boundary;
mod constructors;
mod formals;
mod source;
pub(crate) use boundary::{error_tree, permits_type};
pub(crate) use constructors::constructor;
pub(crate) use formals::static_formal;
#[cfg(test)]
mod tests;

use crate::{
    HirNode,
    concrete_types::{
        InterfaceObservations, Nominal, Observation, TypeTree, VariantShape,
        source_proof::PathResolution,
    },
    data_types::TypeToken,
    unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.named-error-payload-flow";

pub(crate) struct StaticFormal<'a> {
    pub position: u32,
    pub pattern: &'a HirNode,
    pub token: TypeToken,
}

pub(crate) struct FieldOperand<'a> {
    /// Declaration slot; this vector itself remains in source evaluation order.
    pub position: usize,
    pub name: String,
    pub operand: &'a HirNode,
    pub formal: StaticFormal<'a>,
}

pub(crate) enum Constructor<'a> {
    Named {
        token: TypeToken,
        nominal: Nominal,
        variant: String,
        fields: Vec<FieldOperand<'a>>,
    },
    Wrapper {
        token: TypeToken,
        payload: &'a HirNode,
    },
}

pub(crate) struct Function<'a> {
    pub formal: StaticFormal<'a>,
    pub wrapper: Constructor<'a>,
    pub variant: Constructor<'a>,
}

/// Cheap routing only; source, nominal ownership and field identities are
/// checked by `verify_function` before any value or summary can be published.
pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(|text| text.contains("\"kind\":\"mixed_enum\""))
        && !facts.signature.has_self
        && facts.signature.params.len() == 1
        && facts.body.kind == "expr_block"
        && matches!(facts.body.children.as_slice(), [tail]

            if tail.kind == "expr_struct"
                && matches!(tail.children.as_slice(), [payload]

                    if payload.kind == "expr_struct"))
}

/// A whole fresh constructor function, never stored/borrowed owned error state.
pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<Function<'_>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Named error payload requires its registered capability".into());
    }

    facts.verify_source_inputs()?;
    crate::source_evidence::verify(facts)?;

    let tail = source_tail(facts)?;

    let formal = static_formal(facts, 0)?;

    let wrapper = constructor(tail, facts)?.ok_or("Missing fresh error wrapper")?;

    let Constructor::Wrapper { token, payload } = &wrapper else {
        return Err("Fresh named error result must retain its unary wrapper".into());
    };
    verify_return(facts, token)?;

    let variant = constructor(payload, facts)?.ok_or("Missing fresh named error payload")?;

    let Constructor::Named {
        token: payload_token,
        fields,
        ..
    } = &variant
    else {
        return Err("Fresh error wrapper must contain its named variant".into());
    };

    if !matches!(token, TypeToken::Struct { fields, .. } if matches!(fields.as_slice(), [(name, inner)] if name == "0" && inner == payload_token))
    {
        return Err("Fresh named error payload differs from its exact wrapper field".into());
    }

    if fields.len() != 1 || fields[0].formal.position != formal.position {
        return Err("Fresh named error constructor changes its sole formal payload".into());
    }

    Ok(Function {
        formal,
        wrapper,
        variant,
    })
}

fn source_tail(facts: &dyn EvidenceFacts) -> Result<&HirNode, String> {
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;

    source::signature(&syntax)?;

    if syntax.sig.inputs.len() != 1
        || syntax.block.stmts.len() != 1
        || facts.body.kind != "expr_block"
    {
        return Err(
            "Fresh named error function requires one formal and one whole tail constructor".into(),
        );
    }

    let [tail] = facts.body.children.as_slice() else {
        return Err("Fresh named error function cannot omit body operations".into());
    };

    let [syn::Stmt::Expr(expression, None)] = syntax.block.stmts.as_slice() else {
        return Err("Fresh named error requires a tail expression".into());
    };

    source::span(expression, tail.span, facts)?;

    Ok(tail)
}

fn verify_return(facts: &dyn EvidenceFacts, token: &TypeToken) -> Result<(), String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;

    if !matches!(&interface.result, Observation::Captured { ty } if TypeToken::from_tree(ty) == *token)
        || facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .map(crate::data_types::parse)
            .transpose()?
            != Some(token.clone())
    {
        return Err("Fresh named error return differs from complete captured result".into());
    }

    Ok(())
}
