//! Source-boundary kernels; these do not certify synthetic facts as Cargo capture.
use super::*;
use crate::declaration_origins::{BuiltinCopy, Origin};

const PLAIN: &str = "pub enum Month { January = 1, February = 2 }";
const EXPECT: &str = "pub enum Month { #[expect(missing_docs)] January = 1, February = 2 }";

struct Case {
    facts: HirFnFacts,
    environment: Rc<Environment>,
    tree: TypeTree,
    origins: Origins,
}

impl Case {
    fn new(representation: &str, snippet: &str) -> Self {
        let source = format!("{representation} #[derive(Clone, Copy)] {snippet}");
        let mut census =
            (*crate::nominal_boundary::resolution::environment::tests::census()).clone();
        census.files[0].bytes = source.into();
        census.files[0].digest = concrete_types::digest(census.files[0].bytes.as_bytes());
        let environment =
            Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false).unwrap();
        let facts =
            serde_json::from_str(include_str!("function_cleanup_tests/facts.json")).unwrap();

        let nominal = nominal(&environment, snippet);
        let origins = Origins {
            identity: crate::declaration_origins::IDENTITY.into(),
            declarations: vec![origin(&facts, &environment, nominal.clone())],
        };
        let tree = TypeTree::FieldlessEnum {
            nominal,
            source: snippet.into(),
            variants: vec![("January".into(), 1), ("February".into(), 2)],
        };
        Self {
            facts,
            environment: Rc::new(environment),
            tree,
            origins,
        }
    }

    fn verify(&self) -> Result<(), String> {
        let resolution = Resolution {
            namespace: vec![],
            receiver: None,
            owner_path: None,
            range_value: None,
            range_owner: None,
            trees: BTreeMap::new(),
            environment: Rc::clone(&self.environment),
            copy_trees: Default::default(),
            no_drop_trees: Default::default(),
            array_views: false,
            scalar_aliases: false,
            local_import_chains: false,
            passive_copy_dependencies: false,
        };
        DeclarationChecker {
            resolution: &resolution,
            facts: &self.facts,
            origins: &self.origins,
            environment: &self.environment,
            owner: None,
        }
        .verify(&vec!["Month".into()], &self.tree)
        .map(|_| ())
    }
}

fn nominal(environment: &Environment, snippet: &str) -> concrete_types::Nominal {
    let package = &environment.census().context.package;
    concrete_types::Nominal {
        source_identity: Some(package.package_id.clone()),
        package_name: package.name.clone(),
        package_version: package.version.clone(),
        manifest_digest: package.manifest_digest.clone(),
        definition_path: "Month".into(),
        declaration_digest: concrete_types::digest(snippet.as_bytes()),
    }
}

fn origin(
    facts: &HirFnFacts,
    environment: &Environment,
    nominal: concrete_types::Nominal,
) -> Origin {
    let declaration = &environment.index.declarations[&vec!["Month".into()]];
    let file = &environment.census().files[0];
    let start = file.bytes.find("Copy").unwrap() as u32;
    let registry = crate::data_library::registry_for(facts).unwrap();

    Origin {
        nominal,
        source_file: declaration.path.to_string_lossy().into(),
        file_bytes: file.bytes.clone(),
        file_digest: file.digest.clone(),
        declaration_span: declaration.span,
        builtin_copy: Some(BuiltinCopy {
            trait_definition: authority(registry, "trait"),
            derive_definition: authority(registry, "derive"),
            derive_span: crate::ByteSpan {
                start,
                end: start + 4,
            },
        }),
    }
}

fn authority(registry: &crate::data_library::Registry, kind: &str) -> crate::data_library::Callee {
    crate::data_library::Callee {
        path: "core::marker::Copy".into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions[&format!("core::marker::Copy#{kind}")].clone(),
        implementation: None,
    }
}

#[test]
fn complete_repr_is_required_for_variant_expectations_but_default_repr_is_supported() {
    for snippet in [PLAIN, EXPECT] {
        assert!(Case::new("#[repr(u8)]", snippet).verify().is_ok());
        for representation in [
            "#[repr(C)]",
            "#[repr(u16)]",
            "#[repr(u8, C)]",
            "#[repr(u8)] #[repr(u8)]",
        ] {
            assert!(
                Case::new(representation, snippet).verify().is_err(),
                "{representation}"
            );
        }
    }

    assert!(Case::new("", PLAIN).verify().is_ok());
    assert!(Case::new("", EXPECT).verify().is_err());
}

#[test]
fn unchanged_observation_rejects_changed_complete_discriminants_and_historical_profile() {
    let mut case = Case::new("#[repr(u8)]", EXPECT);
    assert!(case.verify().is_ok());
    let changed: syn::ItemEnum = syn::parse_str(
        "#[repr(u8)] pub enum Month { #[expect(missing_docs)] January=1, February=3 }",
    )
    .unwrap();
    assert!(explicit_u8_owner(&case.facts, &changed, &case.tree).is_err());

    case.facts
        .build
        .frontend_options
        .insert("extraction_profile".into(), "hir-data-v2".into());
    assert!(case.verify().is_err());
}

#[test]
fn positive_copy_authority_and_exact_original_derive_token_remain_required() {
    let mut case = Case::new("#[repr(u8)]", EXPECT);
    assert!(case.verify().is_ok());
    let original = case.origins.declarations[0].builtin_copy.clone().unwrap();
    for kind in ["trait", "derive", "span", "absent"] {
        let mut changed = original.clone();
        match kind {
            "trait" => changed.trait_definition.definition_hash.push('x'),
            "derive" => changed.derive_definition.crate_name = "custom".into(),
            "span" => changed.derive_span.start += 1,
            _ => {}
        }
        case.origins.declarations[0].builtin_copy = (kind != "absent").then_some(changed);
        assert!(case.verify().is_err(), "{kind}");
    }
}

#[test]
fn contradictory_drop_is_invalid_rust_and_not_a_new_source_certificate() {
    let case = Case::new("#[repr(u8)]", EXPECT);
    let source = format!(
        "{} impl Drop for Month {{ fn drop(&mut self) {{}} }}",
        case.environment.census().files[0].bytes,
    );
    let mut census = case.environment.census().clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = concrete_types::digest(census.files[0].bytes.as_bytes());
    let environment =
        Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false).unwrap();
    let case = Case {
        environment: Rc::new(environment),
        ..case
    };

    // This kernel consumes positive rustc Copy authority; it cannot establish
    // compilation of contradictory source. Real rustc rejects Copy + Drop.
    // Origin snapshots intentionally remain stale, so full binding rejects it.
    assert!(case.verify().is_ok());
    let (_, snippet) = case.tree.nominal_parts().unwrap();
    assert!(
        case.environment
            .check_origin(&case.origins.declarations[0], snippet)
            .is_err()
    );
}
