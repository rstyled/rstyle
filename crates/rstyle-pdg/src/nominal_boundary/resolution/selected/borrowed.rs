//! Borrowed scalar owners do not acquire owned Copy permission.
use super::*;
use crate::{HirNode, data_types::TypeToken};

pub(super) fn verify(
    facts: &HirFnFacts,
    tree: &TypeTree,
    copies: &std::collections::BTreeSet<Namespace>,
) -> Result<(), String> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Borrowed owner requires a concrete struct".into());
    };
    if !supports(facts, "rust.borrowed-scalar-owners")?
        || fields
            .iter()
            .any(|field| !field_type(facts, &field.ty, copies))
    {
        return Err("Mixed-unit declaration lacks builtin Copy evidence".into());
    }
    for ty in facts
        .signature
        .params
        .iter()
        .map(|parameter| &parameter.ty)
        .chain([&facts.signature.return_ty])
    {
        check_type(ty, nominal)?;
    }
    check_body(&facts.body, nominal)
}

fn field_type(
    facts: &HirFnFacts,
    tree: &TypeTree,
    copies: &std::collections::BTreeSet<Namespace>,
) -> bool {
    match tree {
        TypeTree::Scalar(_) => true,
        TypeTree::Array { element, .. } => {
            supports(facts, "rust.borrowed-array-views").unwrap_or(false)
                && matches!(element.as_ref(), TypeTree::Scalar(_))
        }
        TypeTree::Shared(referent)
            if supports(facts, "rust.borrowed-mixed-owners").unwrap_or(false) =>
        {
            matches!(referent.as_ref(), TypeTree::Slice(element) if matches!(element.as_ref(), TypeTree::Scalar(value) if concrete_types::scalar(value) && value != "unit"))
        }
        TypeTree::MixedEnum { nominal, .. }
            if supports(facts, "rust.borrowed-mixed-owners").unwrap_or(false) =>
        {
            copies.contains(
                &nominal
                    .definition_path
                    .split("::")
                    .map(str::to_string)
                    .collect::<Namespace>(),
            )
        }
        TypeTree::Struct { nominal, .. } => {
            supports(facts, "rust.borrowed-copy-child-fields").unwrap_or(false)
                && copies.contains(
                    &nominal
                        .definition_path
                        .split("::")
                        .map(str::to_string)
                        .collect::<Namespace>(),
                )
        }
        _ => false,
    }
}

fn check_type(
    ty: &crate::ResolvedTy,
    nominal: &crate::concrete_types::Nominal,
) -> Result<(), String> {
    if let Some(token) = &ty.normalized
        && owns(&crate::data_types::parse(token)?, nominal)
    {
        return Err("Borrowed scalar owner cannot be consumed, constructed or dropped".into());
    }
    Ok(())
}

fn owns(token: &TypeToken, owner: &crate::concrete_types::Nominal) -> bool {
    match token {
        TypeToken::Shared(_) => false,
        TypeToken::Result { ok, err } => owns(ok, owner) || owns(err, owner),
        TypeToken::Enum { variants, .. } => variants.iter().any(|(_, value)| owns(value, owner)),
        TypeToken::MixedEnum { variants, .. } => variants
            .iter()
            .filter_map(|(_, shape)| shape.field())
            .any(|value| owns(value, owner)),
        TypeToken::Struct { nominal, fields } => {
            nominal == owner || fields.iter().any(|(_, ty)| owns(ty, owner))
        }
        TypeToken::Tuple(values) => values.iter().any(|ty| owns(ty, owner)),
        TypeToken::Array { element, .. }
        | TypeToken::Slice(element)
        | TypeToken::Option(element) => owns(element, owner),
        _ => false,
    }
}

fn check_body(node: &HirNode, nominal: &crate::concrete_types::Nominal) -> Result<(), String> {
    if let Some(ty) = &node.ty {
        check_type(ty, nominal)?;
    }
    for child in &node.children {
        check_body(child, nominal)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> crate::concrete_types::Nominal {
        crate::concrete_types::Nominal {
            source_identity: Some("original-source".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: concrete_types::digest(b"manifest"),
            definition_path: "sample::Bytes".into(),
            declaration_digest: concrete_types::digest(b"struct Bytes(u8);"),
        }
    }

    #[test]
    fn borrowed_policy_finds_owned_values_in_both_result_arms_and_nested_containers() {
        let nominal = owner();
        let owned = TypeToken::Struct {
            nominal: nominal.clone(),
            fields: vec![("0".into(), TypeToken::Scalar("u8".into()))],
        };
        let scalar = TypeToken::Scalar("unit".into());
        for token in [
            TypeToken::Result {
                ok: Box::new(owned.clone()),
                err: Box::new(scalar.clone()),
            },
            TypeToken::Result {
                ok: Box::new(scalar.clone()),
                err: Box::new(owned.clone()),
            },
            TypeToken::Option(Box::new(TypeToken::Result {
                ok: Box::new(scalar.clone()),
                err: Box::new(owned.clone()),
            })),
            TypeToken::Tuple(vec![TypeToken::Result {
                ok: Box::new(owned.clone()),
                err: Box::new(scalar.clone()),
            }]),
        ] {
            assert!(owns(&token, &nominal), "{token:?}");
            let ty = crate::ResolvedTy {
                spelling: String::new(),
                observed: String::new(),
                normalized: Some(serde_json::to_string(&token).unwrap()),
            };
            assert!(check_type(&ty, &nominal).is_err());
        }
        assert!(!owns(
            &TypeToken::Result {
                ok: Box::new(TypeToken::Shared(Box::new(owned))),
                err: Box::new(scalar)
            },
            &nominal
        ));
    }
}
