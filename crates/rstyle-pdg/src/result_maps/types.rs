//! Structural vocabulary only; nominal, cleanup and consuming-use proof is separate.
use crate::{concrete_types::TypeTree, data_types::TypeToken};

pub(crate) fn tree_result(ok: &TypeTree, err: &TypeTree) -> bool {
    matches!(err, TypeTree::GeneratedStruct { fields, .. }
        if matches!(fields.as_slice(), [field] if field.name == "0"
            && super::shape::error_product(&field.ty)))
        && super::shape::scalar_product(ok)
}

pub(crate) fn token_result(ok: &TypeToken, err: &TypeToken) -> bool {
    scalar_product(ok)
        && (wrapped_error(err)
            || wrapped_success(ok) && crate::concrete_results::scalar_errors::error(err).is_some())
}

pub(crate) fn observed_result(ok: &TypeTree, err: &TypeTree) -> bool {
    tree_result(ok, err) || token_result(&TypeToken::from_tree(ok), &TypeToken::from_tree(err))
}

fn wrapped_success(token: &TypeToken) -> bool {
    matches!(token, TypeToken::Struct { fields, .. }
        if matches!(fields.as_slice(), [(name, payload @ TypeToken::Struct { .. })]
            if name == "0" && scalar_product(payload)))
}

pub(super) fn scalar_product(token: &TypeToken) -> bool {
    match token {
        TypeToken::Scalar(value) => crate::concrete_types::scalar(value) && value != "unit",
        TypeToken::Tuple(values) => !values.is_empty() && values.iter().all(scalar_product),
        TypeToken::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|(_, field)| scalar_product(field))
        }
        _ => false,
    }
}

pub(crate) fn wrapped_error(token: &TypeToken) -> bool {
    let TypeToken::Struct { fields, .. } = token else {
        return false;
    };
    let [(name, TypeToken::Struct { fields, .. })] = fields.as_slice() else {
        return false;
    };
    name == "0"
        && !fields.is_empty()
        && fields.iter().all(|(name, field)| {
            name.parse::<usize>().is_err()
                && matches!(field, TypeToken::Scalar(value)
                if crate::concrete_types::scalar(value) && value != "unit")
        })
}

pub(crate) fn contains_result(token: &TypeToken) -> bool {
    match token {
        TypeToken::Result { ok, err } => {
            token_result(ok, err) || contains_result(ok) || contains_result(err)
        }
        TypeToken::Tuple(values) => values.iter().any(contains_result),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, ty)| contains_result(ty)),
        TypeToken::Option(ty)
        | TypeToken::Shared(ty)
        | TypeToken::Slice(ty)
        | TypeToken::Array { element: ty, .. } => contains_result(ty),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(name: &str, field: &str, payload: TypeToken) -> TypeToken {
        TypeToken::Struct {
            nominal: crate::concrete_types::Nominal {
                source_identity: Some("shape-only".into()),
                package_name: "shape-only".into(),
                package_version: "1".into(),
                manifest_digest: "xxh3-64:0000000000000000".into(),
                definition_path: name.into(),
                declaration_digest: "xxh3-64:0000000000000000".into(),
            },
            fields: vec![(field.into(), payload)],
        }
    }

    #[test]
    fn intermediate_result_preserves_ordinary_error_before_error_mapping() {
        let base = product("Index", "0", TypeToken::Scalar("u32".into()));
        let wrapped = product("PublicIndex", "0", base);
        let error = product("IndexError", "attempted", TypeToken::Scalar("u64".into()));
        assert!(token_result(&wrapped, &error));
        let intermediate = TypeToken::Result {
            ok: Box::new(wrapped.clone()),
            err: Box::new(error.clone()),
        };
        let encoded = serde_json::to_string(&intermediate).unwrap();
        assert_eq!(crate::data_types::parse(&encoded).unwrap(), intermediate);
        assert!(!crate::concrete_results::scalar_errors::GRAMMAR.permits(&wrapped, &error));
    }

    #[test]
    fn intermediate_shape_does_not_admit_borrowed_or_opaque_payloads() {
        let error = product("IndexError", "attempted", TypeToken::Scalar("u64".into()));
        for payload in [
            TypeToken::Shared(Box::new(TypeToken::Scalar("u32".into()))),
            TypeToken::Scalar("String".into()),
            TypeToken::Scalar("unit".into()),
        ] {
            let base = product("Index", "0", payload);
            let wrapped = product("PublicIndex", "0", base);
            assert!(!token_result(&wrapped, &error));
        }
    }

    #[test]
    fn shared_scalar_grammar_retains_generated_origin_rejection_at_every_product_level() {
        let TypeToken::Struct { nominal, .. } =
            product("Wrapper", "0", TypeToken::Scalar("u32".into()))
        else {
            unreachable!()
        };
        let generated = TypeTree::GeneratedStruct {
            nominal,
            template: "struct $name(u32);".into(),
            definition: Box::new(crate::generated_sources::capture::Definition {
                path: "Wrapper".into(),
                definition_identity: "shape-only".into(),
                crate_name: "fixture".into(),
                stable_crate_id: "shape-only".into(),
                local: true,
            }),
            fields: vec![crate::concrete_types::Field {
                position: 0,
                name: "0".into(),
                declaration: "u32".into(),
                ty: TypeTree::Scalar("u32".into()),
            }],
        };
        assert!(scalar_product(&TypeToken::from_tree(&generated)));
        assert!(!super::super::shape::scalar_product(&generated));
        assert!(!super::super::shape::scalar_product(&TypeTree::Tuple(
            vec![generated]
        )));
        let ordinary = TypeTree::Tuple(vec![TypeTree::Scalar("u32".into())]);
        assert!(super::super::shape::scalar_product(&ordinary));
    }
}
