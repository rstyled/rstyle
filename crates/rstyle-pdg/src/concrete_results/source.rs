//! Original constructor operands and exact lexical core variant spelling.
use super::*;
use syn::spanned::Spanned;

pub(crate) fn constructor(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    variant: &str,
) -> Result<(), String> {
    let expression = crate::source_evidence::original_expression(node, facts)?;
    let path = constructor_path(&expression, variant)?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&expression) else {
        unreachable!("checked original constructor call")
    };

    crate::source_evidence::verify_child_span(node, &node.children[0], call.args[0].span(), facts)?;
    facts
        .nominal_resolution()?
        .verify_result_constructor_source(facts, path, variant)
}

pub(crate) fn constructor_path<'a>(
    expression: &'a syn::Expr,
    variant: &str,
) -> Result<&'a syn::Path, String> {
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(expression) else {
        return Err("Result constructor requires an original one-payload call".into());
    };
    if call.args.len() != 1 || !call.attrs.is_empty() {
        return Err("Result constructor requires exactly one original payload".into());
    }

    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Result constructor requires a direct original variant path".into());
    };

    if path.qself.is_some()
        || !path.attrs.is_empty()
        || path
            .path
            .segments
            .last()
            .is_none_or(|part| part.ident != variant)
        || path
            .path
            .segments
            .iter()
            .any(|part| !part.arguments.is_empty())
    {
        return Err("Result constructor loses its exact unsubstituted variant path".into());
    }
    Ok(&path.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn result_source_does_not_accept_unrelated_tags_or_evaluation_shapes() {
        for source in [
            "Ok(())",
            "Err(Error::new())",
            "Result::Ok(())",
            "core::result::Result::Err(Error::new())",
        ] {
            let expression: syn::Expr = syn::parse_str(source).unwrap();
            let variant = if source.contains("Err") { "Err" } else { "Ok" };
            constructor_path(&expression, variant).unwrap();
        }
        for source in [
            "Some(())",
            "Ok()",
            "Ok((), ())",
            "factory()(())",
            "Ok::<(), E>(())",
            "<E as Other>::Ok(())",
        ] {
            let expression: syn::Expr = syn::parse_str(source).unwrap();
            assert!(constructor_path(&expression, "Ok").is_err(), "{source}");
        }
    }
}
