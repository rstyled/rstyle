//! Demand-specific nominal proof in a complete, cfg-selected mixed-content unit.
use super::*;
use crate::declaration_origins::Origins;

pub(super) fn verify(
    facts: &HirFnFacts,
    origins: &Origins,
    trees: &BTreeMap<Namespace, TypeTree>,
    environment: &Environment,
) -> Result<(), String> {
    if trees.is_empty() {
        return Err("Mixed-unit boundary requires selected Copy declarations".into());
    }
    for (key, tree) in trees {
        let TypeTree::Struct {
            nominal, fields, ..
        } = tree
        else {
            return Err("Mixed-unit proof requires concrete structs".into());
        };
        if fields
            .iter()
            .any(|field| !matches!(field.ty, TypeTree::Scalar(_) | TypeTree::Struct { .. }))
        {
            return Err("Mixed-unit Copy fields require scalars or checked nested structs".into());
        }
        let declaration = environment
            .index
            .declarations
            .get(key)
            .ok_or("Missing selected struct declaration")?;
        let syn::Item::Struct(item) =
            &environment.modules[declaration.module_index].items[declaration.item_index]
        else {
            return Err("Selected declaration handle is not a struct".into());
        };
        let origin = origins
            .declarations
            .iter()
            .find(|origin| &origin.nominal == nominal)
            .ok_or("Missing selected declaration origin")?;
        let copy = origin
            .builtin_copy
            .as_ref()
            .ok_or("Mixed-unit declaration lacks builtin Copy evidence")?;
        crate::declaration_origins::copy::verify(facts, origin, copy, &item.attrs)?;
        check_namespace(environment, &declaration.namespace)?;
        structure(facts, item)?;
    }
    Ok(())
}

fn structure(facts: &HirFnFacts, item: &syn::ItemStruct) -> Result<(), String> {
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err("Selected Copy struct must be nongeneric".into());
    }
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);

    for attribute in configuration.effective_attributes(&item.attrs)? {
        if attribute.path().is_ident("derive") {
            derive(&attribute)?;
            continue;
        }
        ordinary(&attribute)?;
    }
    for field in &item.fields {
        function_attributes(facts, &field.attrs)?;
    }
    Ok(())
}

pub(super) fn function_attributes(
    facts: &HirFnFacts,
    attributes: &[syn::Attribute],
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    if !configuration.active(attributes)? {
        return Err("Selected source item is inactive".into());
    }
    for attribute in configuration.effective_attributes(attributes)? {
        ordinary(&attribute)?;
    }
    Ok(())
}

fn ordinary(attribute: &syn::Meta) -> Result<(), String> {
    if ["cfg", "doc", "allow", "warn", "deny", "forbid"]
        .iter()
        .any(|name| attribute.path().is_ident(name))
    {
        Ok(())
    } else {
        Err("Selected source item has unsupported attributes".into())
    }
}

pub(super) fn builtin_derive(name: &str) -> bool {
    matches!(
        name,
        "Clone" | "Copy" | "Debug" | "Eq" | "PartialEq" | "Hash" | "Ord" | "PartialOrd" | "Default"
    )
}

fn derive(attribute: &syn::Meta) -> Result<(), String> {
    let syn::Meta::List(list) = attribute else {
        return Err("Malformed derive".into());
    };
    let paths = list
        .parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
        .map_err(|error| error.to_string())?;
    if paths.iter().any(|path| {
        path.get_ident()
            .is_none_or(|name| !builtin_derive(&name.to_string()))
    }) {
        return Err("Unknown derive can introduce lexical bindings".into());
    }
    Ok(())
}

pub(super) fn module_item(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    if let syn::Item::Type(item) = item
        && concrete_types::scalar(&item.ident.to_string())
    {
        return Err("Primitive spelling is shadowed by an unresolved alias".into());
    }
    if let syn::Item::Macro(item) = item
        && (item.ident.is_none()
            || item
                .ident
                .as_ref()
                .is_some_and(|name| builtin_derive(&name.to_string())))
    {
        return Err("Unresolved lexical macro expansion".into());
    }
    for attribute in
        configuration.effective_attributes(crate::conditional_source::item_attributes(item))?
    {
        if attribute.path().is_ident("derive") {
            derive(&attribute)?;
        } else if ![
            "non_exhaustive",
            "repr",
            "inline",
            "cold",
            "must_use",
            "deprecated",
            "automatically_derived",
        ]
        .iter()
        .any(|name| attribute.path().is_ident(name))
        {
            ordinary(&attribute)?;
        }
    }
    Ok(())
}

pub(super) fn check_namespace(
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    if environment
        .index
        .unresolved_expansions
        .iter()
        .any(|candidate| namespace.starts_with(candidate))
    {
        return Err("Selected source has unresolved lexical expansion/import effects".into());
    }
    Ok(())
}

pub(super) fn implementation(facts: &HirFnFacts, item: &syn::ItemImpl) -> bool {
    item.trait_.is_none()
        && item.unsafety.is_none()
        && item.generics.params.is_empty()
        && item.generics.where_clause.is_none()
        && function_attributes(facts, &item.attrs).is_ok()
}
