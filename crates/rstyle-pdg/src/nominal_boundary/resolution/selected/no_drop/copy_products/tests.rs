use super::*;
use crate::concrete_types::{Field, Nominal};

fn product(source: &str, ty: TypeTree) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: None,
            package_name: "shape-only".into(),
            package_version: "0".into(),
            manifest_digest: "unverified".into(),
            definition_path: "Product".into(),
            declaration_digest: "unverified".into(),
        },
        source: source.into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: "usize".into(),
            ty,
        }],
    }
}

#[test]
fn grammar_accepts_nested_scalar_products_without_granting_proof() {
    let inner = product("struct Inner(usize);", TypeTree::Scalar("usize".into()));
    let outer = product("struct Outer(Inner);", inner);
    assert!(supported(&outer));
    assert!(!supported(&TypeTree::Scalar("usize".into())));
}

#[test]
fn grammar_rejects_generic_lifetime_and_predicate_owners() {
    for source in [
        "struct Product<T>(T);",
        "struct Product<'a>(&'a usize);",
        "struct Product(usize) where usize: Copy;",
        "struct Product<const N: usize>(usize);",
    ] {
        let tree = product(source, TypeTree::Scalar("usize".into()));
        assert!(!supported(&tree), "{source}");
    }
}

#[test]
fn grammar_rejects_reference_option_unit_and_unregistered_fields() {
    for ty in [
        TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Option(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Scalar("unit".into()),
        TypeTree::Scalar("Unknown".into()),
    ] {
        assert!(!supported(&product("struct Product(usize);", ty)));
    }
}

#[test]
fn grammar_bounds_empty_and_recursive_products() {
    let mut tree = product("struct Product(usize);", TypeTree::Scalar("usize".into()));
    for _ in 0..=concrete_types::MAX_DEPTH {
        tree = product("struct Product(Inner);", tree);
    }
    assert!(!supported(&tree));

    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    fields.clear();
    assert!(!supported(&tree));
}

use super::super::product_fields::test_support::{facts, origins, resolution, sources};
use std::collections::BTreeMap;

fn complete(name: &str, source: &str, fields: Vec<(&str, &str, TypeTree)>) -> TypeTree {
    let mut tree = product(source, TypeTree::Scalar("usize".into()));
    let TypeTree::Struct {
        nominal,
        fields: output,
        ..
    } = &mut tree
    else {
        unreachable!()
    };
    nominal.definition_path = format!("sample::{name}");
    *output = fields
        .into_iter()
        .enumerate()
        .map(|(position, (name, declaration, ty))| Field {
            position,
            name: name.into(),
            declaration: declaration.into(),
            ty,
        })
        .collect();
    tree
}

fn pattern() -> TypeTree {
    let small = complete(
        "SmallIndex",
        "#[derive(Clone, Copy)] #[repr(transparent)] struct SmallIndex(u32);",
        vec![("0", "u32", TypeTree::Scalar("u32".into()))],
    );
    complete(
        "PatternID",
        "#[derive(Clone, Copy)] #[repr(transparent)] struct PatternID(SmallIndex);",
        vec![("0", "SmallIndex", small)],
    )
}

fn span() -> TypeTree {
    complete(
        "Span",
        "#[derive(Clone, Copy)] struct Span { start: usize, end: usize }",
        vec![
            ("start", "usize", TypeTree::Scalar("usize".into())),
            ("end", "usize", TypeTree::Scalar("usize".into())),
        ],
    )
}

fn fixture() -> TypeTree {
    complete(
        "Match",
        "#[derive(Clone, Copy)] struct Match { pattern: PatternID, span: Span }",
        vec![
            ("pattern", "PatternID", pattern()),
            ("span", "Span", span()),
        ],
    )
}

fn check(extra: &str, change: impl FnOnce(&mut Origins)) -> Result<CheckedNoDrop, String> {
    let mut tree = fixture();
    let mut declarations = BTreeMap::new();
    sources(&tree, &mut declarations);
    let source = format!(
        "{}\n{extra}",
        declarations.into_values().collect::<Vec<_>>().join("\n")
    );
    let resolution = resolution(&source, &mut tree);
    let facts = facts(&tree);
    let mut origins = origins(&resolution, &facts);
    change(&mut origins);
    verify(&facts, &tree, &resolution, &origins)
}

#[test]
fn complete_nested_copy_source_constructs_only_the_checked_cleanup_witness() {
    let checked = check("", |_| {}).unwrap();
    assert_eq!(checked.nominal().definition_path, "Match");
}

#[test]
fn every_active_nested_nominal_requires_independent_builtin_copy() {
    for name in ["Match", "PatternID", "SmallIndex", "Span"] {
        let missing = check("", |origins| {
            origins
                .declarations
                .iter_mut()
                .find(|origin| origin.nominal.definition_path == name)
                .unwrap()
                .builtin_copy = None;
        })
        .unwrap_err();
        assert!(missing.contains("Copy"), "{name}: {missing}");

        let forged = check("", |origins| {
            origins
                .declarations
                .iter_mut()
                .find(|origin| origin.nominal.definition_path == name)
                .unwrap()
                .builtin_copy
                .as_mut()
                .unwrap()
                .trait_definition
                .crate_hash = "forged".into();
        })
        .unwrap_err();
        assert!(
            forged.contains("Copy observation disagrees"),
            "{name}: {forged}"
        );
    }
}

#[test]
fn source_closure_rejects_original_nested_generated_and_opaque_cleanup() {
    for extra in [
        "impl Drop for Match { fn drop(&mut self) {} }",
        "impl Drop for PatternID { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for SmallIndex { fn drop(&mut self) {} } }",
        "macro_rules! hidden { ($owner:ident) => { impl Drop for $owner { fn drop(&mut self) {} } }; } hidden!(Span);",
        "unknown_sibling!(PatternID);",
        "use external::Cleanup; impl Cleanup for Match {}",
    ] {
        assert!(check(extra, |_| {}).is_err(), "{extra}");
    }
}

#[test]
fn source_copy_shadowing_requires_actual_builtin_derive_authority() {
    for extra in [
        "trait Copy {}",
        "use external::Copy;",
        "macro_rules! shadow { () => { trait Copy {} }; } shadow!();",
    ] {
        let error = check(extra, |origins| {
            let copy = origins
                .declarations
                .iter_mut()
                .find(|origin| origin.nominal.definition_path == "SmallIndex")
                .unwrap()
                .builtin_copy
                .as_mut()
                .unwrap();
            copy.derive_definition.crate_name = "external".into();
        })
        .unwrap_err();
        assert!(
            error.contains("Copy observation disagrees"),
            "{extra}: {error}"
        );
    }
}

#[test]
fn resolved_complete_tree_rejects_swapped_or_truncated_value_components() {
    let mut tree = fixture();
    let mut declarations = BTreeMap::new();
    sources(&tree, &mut declarations);
    let source = declarations.into_values().collect::<Vec<_>>().join("\n");
    let resolution = resolution(&source, &mut tree);
    let facts = facts(&tree);
    let origins = origins(&resolution, &facts);

    let mut swapped = tree.clone();
    let TypeTree::Struct { fields, .. } = &mut swapped else {
        unreachable!()
    };
    fields.swap(0, 1);
    assert!(verify(&facts, &swapped, &resolution, &origins).is_err());

    let TypeTree::Struct { fields, .. } = &mut tree else {
        unreachable!()
    };
    fields.pop();
    assert!(verify(&facts, &tree, &resolution, &origins).is_err());
}

fn owned_fixture() -> (HirFnFacts, Resolution) {
    let mut tree = fixture();
    let mut declarations = BTreeMap::new();
    sources(&tree, &mut declarations);
    let source = format!(
        "{}\nfn f(){{}}",
        declarations.into_values().collect::<Vec<_>>().join("\n")
    );
    let resolution = resolution(&source, &mut tree);
    let mut facts = facts(&tree);
    let offset = source.rfind("fn f()").unwrap() as u32;
    facts.fn_span = crate::ByteSpan {
        start: offset,
        end: offset + facts.source.bytes.len() as u32,
    };
    facts.name = "f".into();
    let census = resolution.environment.census();
    facts.build.frontend_options.insert(
        "source_file".into(),
        serde_json::json!(census.files[0].path),
    );
    facts.build.frontend_options.insert(
        crate::nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(&census.context).unwrap(),
    );
    facts.build.frontend_options.insert(
        "definition_identity".into(),
        serde_json::json!("captured-test-owner"),
    );
    let origins = origins(&resolution, &facts);
    facts.build.frontend_options.insert(
        crate::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    (facts, resolution)
}

#[test]
fn retained_receipts_keep_each_actual_component_certificate_distinct() {
    let (facts, resolution) = owned_fixture();
    let checked = capture(&facts, &resolution).unwrap();

    assert_eq!(checked.declarations.len(), 4);
    assert_eq!(checked.root().nominal().definition_path, "Match");
    for (key, tree) in &resolution.trees {
        let certificate = checked.declaration(tree).unwrap();
        assert_eq!(certificate.nominal().definition_path, key.join("::"));
    }
    let tree = resolution.trees.values().next().unwrap();
    let mut changed = tree.clone();
    let TypeTree::Struct { fields, .. } = &mut changed else {
        unreachable!()
    };
    fields[0].ty = TypeTree::Scalar("u64".into());
    assert!(checked.declaration(&changed).is_none());
    assert!(
        checked
            .declaration(&TypeTree::Scalar("usize".into()))
            .is_none()
    );
}

#[test]
fn retained_root_source_receipt_is_bound_to_exact_function_namespace() {
    let (facts, resolution) = owned_fixture();
    let checked = capture(&facts, &resolution).unwrap();

    assert!(
        checked
            .function_namespace(&facts, &resolution.environment, &[])
            .unwrap()
            .is_some()
    );
    assert!(
        checked
            .function_namespace(&facts, &resolution.environment, &["other".into()])
            .unwrap()
            .is_none()
    );
    let mut changed = facts.clone();
    changed.name = "neighbor".into();
    assert!(
        checked
            .verify_owner(&changed, &resolution.environment)
            .is_err()
    );
    changed = facts.clone();
    changed.fn_span.end -= 1;
    assert!(
        checked
            .verify_owner(&changed, &resolution.environment)
            .is_err()
    );
    changed = facts.clone();
    changed.build.cfgs.push("different_build".into());
    assert!(
        checked
            .verify_owner(&changed, &resolution.environment)
            .is_err()
    );
    changed = facts.clone();
    changed.build.frontend_options.insert(
        "definition_identity".into(),
        serde_json::json!("another-owner"),
    );
    assert!(
        checked
            .verify_owner(&changed, &resolution.environment)
            .is_err()
    );
}

#[test]
fn equivalent_immutable_environment_is_valid_but_changed_source_is_not() {
    let (facts, resolution) = owned_fixture();
    let checked = capture(&facts, &resolution).unwrap();
    let second = Environment::with_options(
        std::sync::Arc::new(resolution.environment.census().clone()),
        true,
        Some(&[]),
        false,
    )
    .unwrap();

    checked.verify_owner(&facts, &second).unwrap();
    let mut census = resolution.environment.census().clone();
    census.files[0].bytes = format!("{}\nconst NEIGHBOR: usize = 1;", census.files[0].bytes).into();
    census.files[0].digest = census.files[0].bytes.identity().into();
    let changed =
        Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false).unwrap();
    assert!(checked.verify_owner(&facts, &changed).is_err());
}
