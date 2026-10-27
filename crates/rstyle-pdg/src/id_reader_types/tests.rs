use super::*;
use crate::concrete_types::{Field, Nominal};

fn wrapper(name: &str, child: TypeTree) -> TypeTree {
    let declaration = match &child {
        TypeTree::Scalar(value) => value.as_str(),
        _ => "Inner",
    };
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("shape-only".into()),
            package_name: "shape-only".into(),
            package_version: "1".into(),
            manifest_digest: "xxh3-64:0000000000000000".into(),
            definition_path: name.into(),
            declaration_digest: "xxh3-64:0000000000000000".into(),
        },
        source: format!("struct {name}({declaration});").into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: declaration.into(),
            ty: child,
        }],
    }
}

fn tree() -> TypeTree {
    TypeTree::Tuple(vec![
        wrapper("ID", wrapper("Inner", TypeTree::Scalar("u32".into()))),
        TypeTree::Scalar("usize".into()),
    ])
}

#[test]
fn ordinary_success_product_retains_its_entire_nested_identity() {
    let success = tree();
    let root = product(&success).unwrap();
    assert!(shape(&TypeToken::from_tree(&success)));
    assert!(original_value(root).is_some());
    assert!(crate::concrete_types::payload_errors::id_value(
        &TypeToken::from_tree(root)
    ));
    assert_eq!(root.nominal_identity().unwrap().definition_path, "ID");
    assert_eq!(
        root.nominal_fields()
            .next()
            .unwrap()
            .ty
            .nominal_identity()
            .unwrap()
            .definition_path,
        "Inner"
    );
}

#[test]
fn shape_does_not_activate_an_older_profile_or_supply_source_ownership() {
    let tree = tree();
    let token = TypeToken::from_tree(&tree);
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts
        .build
        .frontend_options
        .insert("extraction_profile".into(), "hir-data-v2".into());
    assert!(!super::success(&token, &facts).unwrap());
    let value = TypeToken::from_tree(product(&tree).unwrap());
    assert!(!super::value(&value, &facts).unwrap());

    facts
        .build
        .frontend_options
        .insert("extraction_profile".into(), "hir-data-v3".into());
    assert!(super::success(&token, &facts).is_err());
    assert!(super::value(&value, &facts).is_err());
    assert!(!super::value(&token, &facts).unwrap());
}
