//! Source closure checks alone establish no Copy receipt or callable admission.
use super::super::product_fields::test_support as fixture;
use super::*;
use crate::concrete_types::{Field, Nominal};

fn product(name: &str, field: &str, scalar: &str) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("path+file:///sample#sample@0".into()),
            package_name: "sample".into(),
            package_version: "0".into(),
            manifest_digest: "unverified".into(),
            definition_path: name.into(),
            declaration_digest: "unverified".into(),
        },
        source: if field == "0" {
            format!("struct {name}({scalar});")
        } else {
            format!("struct {name} {{ {field}: {scalar} }}")
        }
        .into(),
        fields: vec![Field {
            position: 0,
            name: field.into(),
            declaration: scalar.into(),
            ty: TypeTree::Scalar(scalar.into()),
        }],
    }
}

fn inspect(source: &str) -> Result<(), String> {
    let mut success = product("Product", "0", "u32");
    let resolution = fixture::resolution(source, &mut success);
    let TypeTree::Struct { nominal, .. } = &mut success else {
        unreachable!()
    };
    nominal.definition_path = "owner::Product".into();
    let facts = fixture::facts(&TypeTree::Result {
        ok: Box::new(success),
        err: Box::new(product("Problem", "attempted", "u64")),
    });

    verify_copy_owner(
        &resolution.environment,
        &facts,
        &vec!["owner".into(), "Product".into()],
    )
}

fn in_owner(binding: &str) -> String {
    format!(
        "mod owner {{ struct Product(u32); struct Problem {{ attempted: u64 }} \
         macro_rules! binding {{ () => {{ {binding} }}; }} binding!(); }}"
    )
}

#[test]
fn generated_result_and_tryfrom_bindings_block_original_prelude_lookup() {
    for binding in [
        "type Result<T, E> = core::result::Result<T, E>;",
        "use core::result::Result;",
        "use core::result::{Result as Result};",
        "trait TryFrom<T> {}",
        "use core::convert::TryFrom;",
        "use core::convert::TryFrom as TryFrom;",
    ] {
        let error = inspect(&in_owner(binding)).unwrap_err();
        assert!(
            error.contains("required prelude binding"),
            "{binding}: {error}"
        );
    }
}

#[test]
fn generated_ancestor_and_nested_block_bindings_are_rejected() {
    for source in [
        "macro_rules! binding { () => { type Result<T,E> = core::result::Result<T,E>; }; } \
         binding!(); mod owner { struct Product(u32); struct Problem { attempted: u64 } }",
        "mod owner { struct Product(u32); struct Problem { attempted: u64 } \
         fn nested() { macro_rules! binding { () => { use core::result::Result; }; } binding!(); } }",
    ] {
        assert!(
            inspect(source)
                .unwrap_err()
                .contains("required prelude binding")
        );
    }
}

#[test]
fn generated_root_and_primitive_shadowing_still_fail_the_existing_closure() {
    for binding in [
        "type core = ();",
        "use other as core;",
        "type usize = u32;",
        "use other as usize;",
    ] {
        assert!(inspect(&in_owner(binding)).is_err(), "{binding}");
    }
}

#[test]
fn generated_bindings_in_unrelated_module_namespaces_do_not_shadow_owner() {
    let source = "mod owner { struct Product(u32); struct Problem { attempted: u64 } } \
        mod unrelated { macro_rules! binding { () => { \
            type Result<T,E> = core::result::Result<T,E>; trait TryFrom<T> {} \
        }; } binding!(); }";
    assert!(inspect(source).is_ok());
    let nested = "mod owner { struct Product(u32); struct Problem { attempted: u64 } \
        mod child { macro_rules! binding { () => { use core::result::Result; }; } binding!(); } }";
    assert!(inspect(nested).is_ok());
}

#[test]
fn original_bindings_remain_the_lexical_binders_responsibility() {
    let source = "mod owner { struct Product(u32); struct Problem { attempted: u64 } \
        type Result<T,E> = core::result::Result<T,E>; trait TryFrom<T> {} }";
    assert!(inspect(source).is_ok());
}

#[test]
fn protection_requires_the_copy_owner_mode_and_exact_bounded_success_target() {
    let result = TypeTree::Result {
        ok: Box::new(product("Product", "0", "u32")),
        err: Box::new(product("Problem", "attempted", "u64")),
    };
    let facts = fixture::facts(&result);
    let target = vec!["Product".into()];
    assert_eq!(
        result_prelude_namespace(&facts, &target, SourceMode::CopyOwner).unwrap(),
        Some(vec![])
    );
    for mode in [
        SourceMode::Cleanup,
        SourceMode::Product,
        SourceMode::ScalarError,
    ] {
        assert!(
            result_prelude_namespace(&facts, &target, mode)
                .unwrap()
                .is_none()
        );
    }
    assert!(
        result_prelude_namespace(&facts, &vec!["Other".into()], SourceMode::CopyOwner)
            .unwrap()
            .is_none()
    );
}
