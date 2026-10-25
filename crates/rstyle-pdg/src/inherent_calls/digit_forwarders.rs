//! Literal const owner spelling is checked against lexical and selected identities.
use crate::{HirNode, concrete_types::Nominal, unit_store::EvidenceFacts};

pub(super) fn verify(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    callee: Option<&dyn EvidenceFacts>,
    nominal: Option<&Nominal>,
) -> Result<Option<bool>, String> {
    if !crate::scalar_product_instances::forwarders::candidate(facts) {
        return Ok(None);
    }
    let syn::Expr::Call(call) = crate::source_evidence::original_expression(node, facts)? else {
        return Ok(Some(false));
    };

    let (owner, digits) = owner(&call)?;
    let selected =
        crate::helper_calls::direct_resolution(node).ok_or("Missing digit call selection")?;
    let selected = selected["path"]
        .as_str()
        .ok_or("Missing selected digit path")?;
    let resolution = facts.nominal_resolution()?;
    resolution.verify_digit_call_path(&owner, digits, selected)?;
    if let Some(nominal) = nominal {
        resolution.matches_nominal(&owner, nominal)?;
    }
    if let Some(callee) = callee {
        let checked = crate::scalar_product_instances::activation::verify(callee)?;
        if checked.prefix().consumed() != digits {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}

fn owner(call: &syn::ExprCall) -> Result<(syn::TypePath, u8), String> {
    let syn::Expr::Path(mut owner) = *call.func.clone() else {
        return Err("Digit callee requires its direct owner path".into());
    };
    if owner.qself.is_some() || owner.path.segments.len() < 2 {
        return Err("Digit callee requires its concrete inherent owner".into());
    }
    let method = owner
        .path
        .segments
        .pop()
        .ok_or("Missing digit method")?
        .into_value();
    owner.path.segments.pop_punct();
    if method.ident != "parse" || !matches!(method.arguments, syn::PathArguments::None) {
        return Err("Digit callee changes its original nongeneric method".into());
    }

    let segment = owner
        .path
        .segments
        .last_mut()
        .ok_or("Missing digit owner")?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Digit owner requires one exact literal const argument".into());
    };
    let digits = digits(arguments)?;
    segment.arguments = syn::PathArguments::None;
    Ok((
        syn::TypePath {
            qself: None,
            path: owner.path,
        },
        digits,
    ))
}

fn digits(arguments: &syn::AngleBracketedGenericArguments) -> Result<u8, String> {
    let arguments = arguments.args.iter().collect::<Vec<_>>();
    let [syn::GenericArgument::Const(syn::Expr::Lit(literal))] = arguments.as_slice() else {
        return Err("Digit call requires one independent literal".into());
    };
    let syn::Lit::Int(literal) = &literal.lit else {
        return Err("Digit call requires an integer literal".into());
    };

    let digits = literal
        .base10_parse::<u8>()
        .map_err(|error| error.to_string())?;
    if !literal.suffix().is_empty() || !matches!(digits, 1..=3) {
        return Err("Digit call requires its bounded unsuffixed width".into());
    }
    Ok(digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_const_selection_rejects_computed_generic_and_unrelated_methods() {
        for expression in [
            "ExactlyNDigits::parse(input)",
            "ExactlyNDigits::<0>::parse(input)",
            "ExactlyNDigits::<4>::parse(input)",
            "ExactlyNDigits::<2u8>::parse(input)",
            "ExactlyNDigits::<{1+1}>::parse(input)",
            "ExactlyNDigits::<N>::parse(input)",
            "ExactlyNDigits::<2>::other(input)",
            "ExactlyNDigits::<2>::parse::<u8>(input)",
            "ExactlyNDigits::<2,3>::parse(input)",
        ] {
            assert!(
                owner(&syn::parse_str(expression).unwrap()).is_err(),
                "{expression}"
            );
        }
        for width in 1..=3 {
            let source = format!("ExactlyNDigits::<{width}>::parse(input)");
            assert_eq!(owner(&syn::parse_str(&source).unwrap()).unwrap().1, width);
        }
    }
}
