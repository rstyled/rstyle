//! Original fieldless variant values and explicit discriminant-to-integer casts.
use crate::concrete_types::source_proof::PathResolution;
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.fieldless-enum-values";
pub(crate) const VALUE: &str = "data.fieldless-value";
pub(crate) const CAST: &str = "data.fieldless-cast";

fn selected(node: &HirNode, facts: &dyn EvidenceFacts, kind: &str) -> bool {
    node.kind == kind && crate::extraction_profile::facts_support(facts, CAPABILITY)
}

pub(crate) fn value_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    selected(node, facts, "expr_path")
        && node
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_fieldless_variant")
}

fn qualified_value(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(syn::Path, String), String> {
    let expression: syn::ExprPath =
        syn::parse_str(source(node, facts)?).map_err(|error| error.to_string())?;
    if expression.qself.is_some()
        || !expression.attrs.is_empty()
        || expression.path.segments.len() < 2
        || expression
            .path
            .segments
            .iter()
            .any(|part| !matches!(part.arguments, syn::PathArguments::None))
    {
        return Err("Fieldless value requires an enum-qualified path".into());
    }

    let mut owner = expression.path;
    let name = owner
        .segments
        .pop()
        .ok_or("Missing variant path")?
        .into_value()
        .ident
        .to_string();
    owner.segments.pop_punct();
    Ok((owner, name))
}

fn verify_identity(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    nominal: &crate::concrete_types::Nominal,
    name: &str,
) -> Result<(), String> {
    let resolution = node.res.as_ref().ok_or("Missing variant identity")?;
    let identity: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if resolution.target.is_some()
        || identity.path != format!("{}::{name}", nominal.definition_path)
        || identity.crate_hash != "local-unencoded"
        || identity.definition_hash.is_empty()
        || identity.implementation.is_some()
        || Some(identity.crate_name.as_str())
            != facts
                .build
                .frontend_options
                .get("rstyle.nominal_selected_unit")
                .and_then(|unit| unit.get("crate_name"))
                .and_then(serde_json::Value::as_str)
    {
        return Err("Fieldless variant identity disagrees with source".into());
    }

    Ok(())
}

pub(crate) fn cast_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    selected(node, facts, "expr_cast")
        && node.children.first().is_some_and(|child| {
            crate::data_aggregates::token(child)
                .is_ok_and(|token| token.starts_with("{\"kind\":\"fieldless_enum\","))
        })
}

pub(crate) fn value(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(String, i64), String> {
    if !value_candidate(node, facts) || node.expansion.is_some() || !node.children.is_empty() {
        return Err("Fieldless value requires original resolved unit-variant syntax".into());
    }

    let TypeToken::FieldlessEnum { nominal, variants } = checked_type(node, facts)? else {
        return Err("Missing complete fieldless value type".into());
    };

    let (owner, name) = qualified_value(node, facts)?;
    let tree = facts.nominal_resolution()?.resolve(&syn::TypePath {
        qself: None,
        path: owner,
    })?;
    if TypeToken::from_tree(&tree) != checked_type(node, facts)? {
        return Err("Fieldless variant path disagrees with nominal source".into());
    }
    verify_identity(node, facts, &nominal, &name)?;

    variants
        .into_iter()
        .find(|(tag, _)| tag == &name)
        .ok_or("Unknown fieldless variant".into())
}

pub(crate) fn cast(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !cast_candidate(node, facts) || node.expansion.is_some() {
        return Err("Fieldless cast requires original resolved syntax".into());
    }

    let [operand] = node.children.as_slice() else {
        return Err("Fieldless cast requires one saved value".into());
    };

    let input = crate::data_aggregates::token(operand)?;
    checked_type(operand, facts)?;
    let result = crate::data_aggregates::token(node)?;
    if !crate::concrete_types::scalar(result) || matches!(result, "bool" | "unit") {
        return Err("Fieldless cast requires an integer result".into());
    }

    let expected = cast_operand(node, facts, result)?;
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expected, operand.span)
        || !node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin"
                && dispatch.op == "as"
                && dispatch.operand_tys == [input]
                && dispatch.result_ty == result
        })
    {
        return Err(
            "Fieldless cast loses operand, destination or builtin dispatch evidence".into(),
        );
    }

    Ok(())
}

fn cast_operand(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    result: &str,
) -> Result<crate::ByteSpan, String> {
    let syntax: syn::ExprCast =
        syn::parse_str(source(node, facts)?).map_err(|error| error.to_string())?;
    if !syntax.attrs.is_empty()
        || !matches!(syntax.ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(result))
    {
        return Err("Fieldless cast requires its exact primitive destination".into());
    }
    let expected = crate::ByteSpan {
        start: node.span.start
            + crate::source_evidence::byte_offset(source(node, facts)?, syntax.expr.span().start())
                as u32,
        end: node.span.start
            + crate::source_evidence::byte_offset(source(node, facts)?, syntax.expr.span().end())
                as u32,
    };

    Ok(expected)
}

fn checked_type(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<TypeToken, String> {
    let token = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    if !matches!(token, TypeToken::FieldlessEnum { .. })
        || facts.build.pointer_width.value != Some(64)
        || !facts.nominal_resolution()?.checked_copy_token(&token)
    {
        return Err(
            "Fieldless enum requires complete checked Copy source in a 64-bit context".into(),
        );
    }

    Ok(token)
}

fn source<'a>(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Result<&'a str, String> {
    facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing fieldless source".into())
}
