//! Source-derived values for complete default-representation fieldless enums.
use super::*;

pub fn fieldless_variants(source: &str) -> Result<Vec<(String, i64)>, String> {
    let item: syn::ItemEnum = syn::parse_str(source).map_err(|error| error.to_string())?;
    if source.len() > MAX_SOURCE_BYTES
        || item.variants.is_empty()
        || item.variants.len() > MAX_FIELDS
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.attrs.iter().any(|attr| !allowed(attr))
    {
        return Err("Fieldless enum requires a bounded complete default-repr declaration".into());
    }

    let mut result = Vec::new();
    for variant in item.variants {
        if !matches!(variant.fields, syn::Fields::Unit)
            || variant
                .attrs
                .iter()
                .any(|attr| !attr.path().is_ident("doc"))
        {
            return Err(
                "Fieldless enum requires original unit variants without cfg removal".into(),
            );
        }

        let value = match variant.discriminant {
            Some((_, expression)) => discriminant(&expression, 0)?,
            None => result.last().map_or(Ok(0), |(_, value): &(String, i64)| {
                value
                    .checked_add(1)
                    .ok_or_else(|| "Fieldless discriminant overflow".to_owned())
            })?,
        };

        let name = variant.ident.to_string();
        if result
            .iter()
            .any(|(prior, prior_value)| prior == &name || prior_value == &value)
        {
            return Err("Fieldless enum repeats a name or discriminant".into());
        }
        result.push((name, value));
    }

    Ok(result)
}

fn allowed(attribute: &syn::Attribute) -> bool {
    attribute.path().is_ident("doc") || attribute.path().is_ident("derive")
}

fn discriminant(expression: &syn::Expr, depth: usize) -> Result<i64, String> {
    if depth > MAX_DEPTH {
        return Err("Fieldless discriminant exceeds expression depth budget".into());
    }

    match expression {
        syn::Expr::Paren(value) if value.attrs.is_empty() => discriminant(&value.expr, depth + 1),
        syn::Expr::Group(value) if value.attrs.is_empty() => discriminant(&value.expr, depth + 1),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(value),
            attrs,
        }) if attrs.is_empty() && value.suffix().is_empty() => value
            .base10_parse()
            .map_err(|error| format!("Invalid fieldless discriminant: {error}")),
        syn::Expr::Unary(value)
            if value.attrs.is_empty() && matches!(value.op, syn::UnOp::Neg(_)) =>
        {
            discriminant(&value.expr, depth + 1)?
                .checked_neg()
                .ok_or("Fieldless discriminant overflow".into())
        }
        syn::Expr::Binary(value)
            if value.attrs.is_empty() && matches!(value.op, syn::BinOp::Shl(_)) =>
        {
            let left = i128::from(discriminant(&value.left, depth + 1)?);
            let right = u32::try_from(discriminant(&value.right, depth + 1)?)
                .map_err(|_| "Invalid discriminant shift")?;
            if right >= 64 {
                return Err("Invalid discriminant shift".into());
            }
            i64::try_from(left << right).map_err(|_| "Fieldless discriminant overflow".into())
        }
        _ => {
            Err("Fieldless discriminant requires independently checked literal/shift syntax".into())
        }
    }
}

pub(super) fn check(
    nominal: &Nominal,
    source: &crate::SourceText,
    variants: &[(String, i64)],
) -> Result<(), String> {
    check_identity(nominal, source)?;
    let item: syn::ItemEnum = syn::parse_str(source).map_err(|error| error.to_string())?;
    if nominal.definition_path.rsplit("::").next() != Some(item.ident.to_string().as_str())
        || fieldless_variants(source)? != variants
    {
        return Err("Fieldless enum source, identity or discriminants disagree".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_discriminants_preserve_shift_values_and_implicit_successors() {
        assert_eq!(
            fieldless_variants("enum Flag { A = 1 << 0, B = 1 << 17, C, D = -7, E }").unwrap(),
            vec![
                ("A".into(), 1),
                ("B".into(), 131072),
                ("C".into(), 131073),
                ("D".into(), -7),
                ("E".into(), -6)
            ]
        );
    }

    #[test]
    fn unsupported_or_incomplete_declarations_do_not_invent_discriminants() {
        for source in [
            "enum E {}",
            "enum E<T> { A }",
            "#[repr(u8)] enum E { A }",
            "#[repr(C)] enum E { A }",
            "enum E { A(u8) }",
            "enum E { A { x: u8 } }",
            "enum E { A = CONST }",
            "enum E { A = 1 << 64 }",
            "enum E { A = 1 << 63 }",
            "enum E { A = 9223372036854775807, B }",
            "enum E { A = 1, B = 1 }",
            "enum E { #[cfg(any())] A, B }",
            "enum E { A = other() }",
        ] {
            assert!(fieldless_variants(source).is_err(), "{source}");
        }
    }

    #[test]
    fn observed_tables_require_exact_source_and_nominal_identity() {
        let source: crate::SourceText = "enum Flag { A = 1 << 2, B = 1 << 7 }".into();
        let nominal = Nominal {
            source_identity: Some("test-source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: digest(b"manifest"),
            definition_path: "sample::Flag".into(),
            declaration_digest: source.identity().into(),
        };

        let variants = fieldless_variants(&source).unwrap();
        assert!(check(&nominal, &source, &variants).is_ok());
        assert!(check(&nominal, &source, &variants[..1]).is_err());
        assert!(check(&nominal, &source, &[("A".into(), 2), ("B".into(), 128)]).is_err());
        let mut changed = nominal;
        changed.definition_path = "sample::Other".into();
        assert!(check(&changed, &source, &variants).is_err());
    }
}
