//! Shared complete-source witness fixtures; supplied claims never install proof.
use super::*;
use crate::concrete_types::VariantShape;
use crate::nominal_boundary::resolution::{collect, environment};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};
use syn::spanned::Spanned;

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn sources(
    tree: &TypeTree,
    output: &mut BTreeMap<String, String>,
) {
    if let Some((nominal, source)) = tree.nominal_parts() {
        output.insert(nominal.definition_path.clone(), source.to_string());
    }
    for field in tree.nominal_fields() {
        sources(&field.ty, output);
    }
}

fn retag(tree: &mut TypeTree, package: &crate::nominal_binding::PackageContext) {
    match tree {
        TypeTree::Struct {
            nominal,
            source,
            fields,
            ..
        } => {
            strip_attributes(source);
            bind_nominal(nominal, package);
            nominal.declaration_digest = source.identity().into();
            for field in fields {
                retag(&mut field.ty, package);
            }
        }
        TypeTree::MixedEnum {
            nominal,
            source,
            variants,
            ..
        } => {
            strip_attributes(source);
            bind_nominal(nominal, package);
            nominal.declaration_digest = source.identity().into();
            for variant in variants {
                if let VariantShape::SingleTuple { field } = &mut variant.shape {
                    retag(&mut field.ty, package);
                }
            }
        }
        _ => {}
    }
}

fn strip_attributes(source: &mut crate::SourceText) {
    let item: syn::Item = syn::parse_str(source).unwrap();
    let span = crate::nominal_boundary::resolution::nominal_span(source, &item).unwrap();
    *source = source[span.start as usize..span.end as usize]
        .to_owned()
        .into();
}

fn bind_nominal(
    nominal: &mut concrete_types::Nominal,
    package: &crate::nominal_binding::PackageContext,
) {
    nominal.definition_path = nominal.definition_path.rsplit("::").next().unwrap().into();
    nominal.source_identity = Some(package.package_id.clone());
    nominal.package_name = package.name.clone();
    nominal.package_version = package.version.clone();
    nominal.manifest_digest = package.manifest_digest.clone();
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn resolution(
    source: &str,
    tree: &mut TypeTree,
) -> Resolution {
    let mut census = (*environment::tests::census()).clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = census.files[0].bytes.identity().into();
    retag(tree, &census.context.package);
    let environment =
        Rc::new(Environment::with_options(Arc::new(census), true, Some(&[]), false).unwrap());
    let mut trees = BTreeMap::new();
    collect(tree, &mut trees).unwrap();
    Resolution {
        namespace: vec![],
        receiver: None,
        owner_path: None,
        range_value: None,
        range_owner: None,
        trees,
        environment,
        copy_trees: Default::default(),
        no_drop_trees: Default::default(),
        array_views: false,
        scalar_aliases: false,
        local_import_chains: false,
        source_requirements: Default::default(),
        copy_owner_source: None,
        scalar_result_source: None,
        copy_conversion_source: None,
        scalar_product_source: None,
        nonzero_product_source: None,
        transport_sources: None,
        generated_declarations: Vec::new(),
        id_reader_constant_chains: Default::default(),
    }
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn facts(
    tree: &TypeTree,
) -> HirFnFacts {
    let mut facts: HirFnFacts =
        serde_json::from_str(include_str!("../../function_cleanup_tests/facts.json")).unwrap();
    let interface = InterfaceObservations {
        identity: concrete_types::IDENTITY.into(),
        parameters: vec![],
        result: Observation::Captured { ty: tree.clone() },
    };
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(interface).unwrap(),
    );
    facts
}

fn callee(facts: &HirFnFacts, key: &str) -> crate::data_library::Callee {
    let registry = crate::data_library::registry_for(facts).unwrap();
    crate::data_library::Callee {
        path: "core::marker::Copy".into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions[key].clone(),
        implementation: None,
    }
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn origins(
    resolution: &Resolution,
    facts: &HirFnFacts,
) -> Origins {
    let environment = &resolution.environment;
    let file = &environment.census().files[0];
    let declarations = resolution
        .trees
        .iter()
        .map(|(key, tree)| {
            let declaration = &environment.index.declarations[key];
            let item = &environment.modules[declaration.module_index].items[declaration.item_index];
            let attributes = super::super::super::declaration_attributes(item, tree).unwrap();
            let copy = copy_span(attributes, &file.bytes).map(|derive_span| {
                crate::declaration_origins::BuiltinCopy {
                    trait_definition: callee(facts, "core::marker::Copy#trait"),
                    derive_definition: callee(facts, "core::marker::Copy#derive"),
                    derive_span,
                }
            });
            crate::declaration_origins::Origin {
                nominal: tree.nominal_parts().unwrap().0.clone(),
                source_file: declaration.path.to_string_lossy().into(),
                file_bytes: file.bytes.clone(),
                file_digest: file.digest.clone(),
                declaration_span: declaration.span,
                builtin_copy: copy,
            }
        })
        .collect();
    Origins {
        generated: Vec::new(),
        identity: crate::declaration_origins::IDENTITY.into(),
        declarations,
    }
}

fn copy_span(attributes: &[syn::Attribute], source: &str) -> Option<crate::ByteSpan> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("derive"))
        .find_map(|attribute| {
            let paths = attribute
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                )
                .unwrap();
            paths
                .iter()
                .find(|path| path.is_ident("Copy"))
                .map(|path| crate::source_evidence::original_span(source, path.span()))
        })
}
