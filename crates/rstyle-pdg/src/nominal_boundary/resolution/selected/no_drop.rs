//! Source-derived cleanup proof, distinct from Copy and borrowed ownership.
use super::*;
mod closure;
mod enum_globs;
mod function_imports;
mod headers;
mod library_macros;
mod macros;
#[cfg(test)]
mod tests;

/// Only this module constructs the witness; serialized negative flags cannot.
#[derive(Clone, Debug)]
pub(super) struct CheckedNoDrop {
    nominal: concrete_types::Nominal,
}

impl CheckedNoDrop {
    pub(super) fn nominal(&self) -> &concrete_types::Nominal {
        &self.nominal
    }
}

pub(super) fn verify(
    facts: &HirFnFacts,
    tree: &TypeTree,
    environment: &Environment,
) -> Result<CheckedNoDrop, String> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = tree
    else {
        return Err("Owned cleanup proof requires an exact local struct".into());
    };

    concrete_types::validate(tree)?;
    crate::data_library::verify_sysroot_roots(facts, &["alloc", "core", "std"], "Cleanup source")?;
    if fields.len() != 1 || fields[0].ty != TypeTree::Scalar("unit".into()) {
        return Err("Owned cleanup proof requires one complete unit tuple field".into());
    }
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Cleanup source requires its pinned compiler/target")?;
    if !library_macros::pinned_native(registry) {
        return Err("Cleanup source macro pin is incomplete".into());
    }
    let key = original_declaration(nominal, environment)?;
    if !environment.checked_cleanup_source(&facts.build.cfgs, &key) {
        closure::verify(environment, facts, &key)?;
        environment.cache_cleanup_source(&facts.build.cfgs, &key);
    }
    Ok(CheckedNoDrop {
        nominal: nominal.clone(),
    })
}

fn original_declaration(
    nominal: &concrete_types::Nominal,
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
    let syn::Item::Struct(item) = item else {
        return Err("Cleanup declaration does not identify a struct".into());
    };
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(&item.fields, syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 && matches!(&fields.unnamed[0].ty, syn::Type::Tuple(unit) if unit.elems.is_empty()))
    {
        return Err("Cleanup declaration requires an original nongeneric unit tuple".into());
    }
    let source = environment
        .source(&declaration.path)?
        .range(declaration.span)?;
    if concrete_types::digest(source.as_bytes()) != nominal.declaration_digest {
        return Err("Cleanup declaration source identity disagrees".into());
    }
    Ok(key)
}
