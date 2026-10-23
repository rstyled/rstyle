//! Source-derived cleanup proof, distinct from Copy and borrowed ownership.
use super::*;
mod closure;
pub(in crate::nominal_boundary::resolution) mod copy_components;
pub(in crate::nominal_boundary::resolution) mod copy_owners;
pub(in crate::nominal_boundary::resolution) mod copy_products;
pub(super) mod enum_errors;
mod enum_globs;
mod function_imports;
mod headers;
mod library_macros;
mod macros;
pub(super) mod module_globs;
mod prelude_traits;

pub(in crate::nominal_boundary::resolution) fn default_prelude(facts: &HirFnFacts) -> bool {
    crate::data_library::registry_for(facts).is_some_and(prelude_traits::pinned)
}
mod product_fields;
pub(in crate::nominal_boundary::resolution) mod reference_products;
mod source_imports;
#[cfg(test)]
mod source_imports_tests;
#[cfg(test)]
mod tests;

/// Select the separately bounded source/cleanup grammar; grants no value authority.
pub(in crate::nominal_boundary::resolution) enum ProductProof {
    Reference,
    Copy,
}

pub(in crate::nominal_boundary::resolution) fn probe_product(
    facts: &HirFnFacts,
    resolution: &Resolution,
    proof: ProductProof,
) -> Result<(), String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Err(match proof {
            ProductProof::Reference => "Product cleanup probe requires a captured complete result",
            ProductProof::Copy => "Copy product probe requires a captured complete result",
        }
        .into());
    };
    let origins = facts.build.frontend_options.data::<Origins>()?;
    match proof {
        ProductProof::Reference => reference_products::verify(facts, ty, resolution, &origins),
        ProductProof::Copy => copy_products::verify(facts, ty, resolution, &origins),
    }
    .map(|_| ())
}

/// Only this module constructs the witness; serialized negative flags cannot.
#[derive(Clone, Debug)]
pub(in crate::nominal_boundary::resolution) struct CheckedNoDrop {
    nominal: concrete_types::Nominal,
}

impl CheckedNoDrop {
    pub(in crate::nominal_boundary::resolution) fn nominal(&self) -> &concrete_types::Nominal {
        &self.nominal
    }
}

pub(super) fn verify(
    facts: &HirFnFacts,
    tree: &TypeTree,
    environment: &Environment,
) -> Result<CheckedNoDrop, String> {
    concrete_types::validate(tree)?;
    crate::data_library::verify_sysroot_roots(facts, &["alloc", "core", "std"], "Cleanup source")?;
    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Cleanup proof requires an original nominal type")?;
    match tree {
        TypeTree::Struct { fields, .. }
            if fields.len() == 1 && fields[0].ty == TypeTree::Scalar("unit".into()) => {}
        TypeTree::MixedEnum { .. }
            if supports(facts, enum_errors::CAPABILITY)? && enum_errors::supported(tree) => {}
        _ => {
            return Err(
                "Owned cleanup proof requires its complete registered source family".into(),
            );
        }
    }

    let registry = crate::data_library::registry_for(facts)
        .ok_or("Cleanup source requires its pinned compiler/target")?;
    if !library_macros::pinned_native(registry) {
        return Err("Cleanup source macro pin is incomplete".into());
    }
    let key = original_declaration(nominal, tree, environment)?;
    let profile = crate::extraction_profile::ExtractionProfile::from_facts(facts)?.token();
    if !environment.checked_cleanup_source(&facts.build.cfgs, &key, profile) {
        closure::verify(environment, facts, &key)?;
        environment.cache_cleanup_source(&facts.build.cfgs, &key, profile);
    }
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

fn original_declaration(
    nominal: &concrete_types::Nominal,
    tree: &TypeTree,
    environment: &Environment,
) -> Result<Namespace, String> {
    let key: Namespace = nominal
        .definition_path
        .split("::")
        .map(str::to_owned)
        .collect();
    let declaration = environment
        .index
        .declarations
        .get(&key)
        .ok_or("Missing exact cleanup declaration ownership")?;

    let item = &environment.modules[declaration.module_index].items[declaration.item_index];
    match (item, tree) {
        (syn::Item::Struct(item), TypeTree::Struct { .. }) => unit_declaration(item)?,
        (syn::Item::Enum(item), TypeTree::MixedEnum { .. }) => {
            enum_errors::declaration(item, tree)?
        }
        _ => return Err("Cleanup declaration has an unsupported nominal kind".into()),
    }
    if !environment.declaration_matches(declaration, &nominal.declaration_digest)? {
        return Err("Cleanup declaration source identity disagrees".into());
    }
    Ok(key)
}

fn unit_declaration(item: &syn::ItemStruct) -> Result<(), String> {
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(&item.fields, syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 && matches!(&fields.unnamed[0].ty, syn::Type::Tuple(unit) if unit.elems.is_empty()))
    {
        return Err("Cleanup declaration requires an original nongeneric unit tuple".into());
    }
    Ok(())
}
