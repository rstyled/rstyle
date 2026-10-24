//! Primitive conversion syntax only. Callers prove resolution, concrete types,
//! source correspondence, operand evaluation and operation semantics separately.
pub(crate) fn primitive_conversion<'a>(
    source: &'a syn::Expr,
    target: &str,
) -> Result<&'a syn::Expr, String> {
    let syn::Expr::Call(call) = source else {
        return Err("Integer conversion requires direct syntax".into());
    };
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Integer conversion requires a builtin primitive path".into());
    };

    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    if call.args.len() != 1
        || path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .any(|part| !part.arguments.is_empty())
        || !(names == [target, "try_from"]
            || names == ["core", "primitive", target, "try_from"]
            || names == ["std", "primitive", target, "try_from"])
    {
        return Err("Integer conversion source is outside its primitive contract".into());
    }
    Ok(&call.args[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_target_syntax_preserves_the_complete_evaluated_argument() {
        for target in ["u16", "usize", "u64"] {
            for root in [
                "",
                "core::primitive::",
                "std::primitive::",
                "::core::primitive::",
            ] {
                let syntax = format!("{root}{target}::try_from({{ effect(); value }})");
                let expression = syn::parse_str(&syntax).unwrap();
                let argument = primitive_conversion(&expression, target).unwrap();
                assert!(
                    matches!(argument, syn::Expr::Block(block) if block.block.stmts.len() == 2)
                );
                assert!(primitive_conversion(&expression, "u32").is_err());
            }
        }
    }

    #[test]
    fn alternate_callees_generic_paths_and_wrong_arity_are_not_primitive_syntax() {
        for syntax in [
            "Custom::try_from(value)",
            "convert(value)",
            "value.try_from()",
            "u64::try_from()",
            "u64::try_from(value, other)",
            "u64::try_from::<usize>(value)",
            "<u64 as TryFrom<usize>>::try_from(value)",
            "alias::primitive::u64::try_from(value)",
            "u64::from(value)",
        ] {
            let expression = syn::parse_str(syntax).unwrap();
            assert!(
                primitive_conversion(&expression, "u64").is_err(),
                "{syntax}"
            );
        }
    }
}
