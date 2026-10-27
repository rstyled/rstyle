//! Reference-free Option products require independently checked Copy declarations.
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.concrete-option-propagation";

pub(crate) fn payload(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<TypeToken, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Product Option requires its concrete propagation capability".into());
    }
    let token = crate::data_aggregates::token(node)?;
    let TypeToken::Option(payload) = crate::data_types::parse(token)? else {
        return Err("Expected concrete Option product".into());
    };
    if !matches!(
        payload.as_ref(),
        TypeToken::Tuple(_) | TypeToken::Struct { .. }
    ) {
        return Err("Expected an owned Option product payload".into());
    }

    let resolution = facts.nominal_resolution()?;
    if !checked_copy(&payload, &resolution) {
        return Err("Option product lacks independent reference-free Copy evidence".into());
    }
    Ok(*payload)
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    for ty in types(facts) {
        let Some(text) = ty.normalized.as_deref() else {
            continue;
        };
        if !text.contains("\"kind\":\"option\"") {
            continue;
        }
        let token = crate::data_types::parse(text)?;
        if crate::data_types::product_option(&token) {
            if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
                return Err("Product Options require their registered capability".into());
            }
            let TypeToken::Option(payload) = token else {
                return Err("Nested product Option storage is outside this boundary".into());
            };

            let resolution = facts.nominal_resolution()?;
            if !checked_copy(&payload, &resolution) {
                return Err("Product Option lacks complete independent Copy proof".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || {
            types(facts)
                .into_iter()
                .filter_map(|ty| ty.normalized.as_deref())
                .filter(|text| text.contains("\"kind\":\"option\""))
                .any(|text| {
                    crate::data_types::parse(text)
                        .is_ok_and(|ty| crate::data_types::product_option(&ty))
                })
        },
        "Product Option requires its registered document policy",
    )
}

fn types(facts: &dyn EvidenceFacts) -> Vec<&crate::ResolvedTy> {
    fn nodes<'a>(node: &'a HirNode, output: &mut Vec<&'a crate::ResolvedTy>) {
        if let Some(ty) = &node.ty {
            output.push(ty);
        }
        for child in &node.children {
            nodes(child, output);
        }
    }
    let mut output: Vec<_> = facts
        .signature
        .params
        .iter()
        .map(|param| &param.ty)
        .chain(std::iter::once(&facts.signature.return_ty))
        .chain(facts.bindings.iter().map(|binding| &binding.ty))
        .collect();
    nodes(&facts.body, &mut output);
    output
}

pub(crate) fn checked_copy(
    token: &TypeToken,
    resolution: &crate::nominal_boundary::resolution::Resolution,
) -> bool {
    crate::data_types::option_product_with(token, &|nominal| resolution.checked_copy_token(nominal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(payload: TypeToken) -> String {
        serde_json::to_string(&TypeToken::Option(Box::new(payload))).unwrap()
    }

    #[test]
    fn product_vocabulary_accepts_only_reference_free_components() {
        let scalar = TypeToken::Scalar("usize".into());
        let positive =
            TypeToken::Tuple(vec![scalar.clone(), TypeToken::Tuple(vec![scalar.clone()])]);
        assert!(crate::data_types::parse(&option(positive)).is_ok());
        let negatives = [
            TypeToken::Shared(Box::new(scalar.clone())),
            TypeToken::Option(Box::new(scalar.clone())),
            TypeToken::Array {
                element: Box::new(scalar.clone()),
                length: 2,
            },
            TypeToken::Scalar("unproved".into()),
        ];
        for negative in negatives {
            assert!(crate::data_types::parse(&option(TypeToken::Tuple(vec![negative]))).is_err());
        }
    }

    #[test]
    fn product_detector_finds_options_inside_containers_without_widening_scalars() {
        let scalar = TypeToken::Scalar("u8".into());
        assert!(!crate::data_types::product_option(&TypeToken::Option(
            Box::new(scalar)
        )));
        let nested = TypeToken::Shared(Box::new(TypeToken::Tuple(vec![TypeToken::Option(
            Box::new(TypeToken::Tuple(vec![TypeToken::Scalar("usize".into())])),
        )])));
        assert!(crate::data_types::product_option(&nested));
    }
}
