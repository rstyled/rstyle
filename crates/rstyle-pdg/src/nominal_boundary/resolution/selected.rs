//! Demand-specific nominal proof in a complete, cfg-selected mixed-content unit.
use super::*;
use crate::declaration_origins::Origins;
mod borrowed;
mod imports;
pub(super) mod passive;
mod scalar_leaf;

pub(super) struct Selection {
    pub copies: std::collections::BTreeSet<Namespace>,
    pub passive_dependencies: bool,
}

pub(super) fn verify(
    facts: &HirFnFacts,
    origins: &Origins,
    trees: &BTreeMap<Namespace, TypeTree>,
    environment: &Environment,
) -> Result<Selection, String> {
    if trees.is_empty() && !supports(facts, "rust.accelerator-packing")? {
        return Err("Mixed-unit boundary requires selected Copy declarations".into());
    }
    let mut copies = std::collections::BTreeSet::new();
    let mut passive_dependencies = false;
    for (key, tree) in trees {
        verify_type(facts, tree)?;
        let declaration = environment
            .index
            .declarations
            .get(key)
            .ok_or("Missing selected struct declaration")?;
        let item = &environment.modules[declaration.module_index].items[declaration.item_index];
        let attributes = declaration_attributes(item, tree)?;
        if checked_copy(facts, origins, tree, attributes)? {
            copies.insert(key.clone());
        }
        if let Err(error) = check_namespace(facts, environment, &declaration.namespace) {
            if !copies.contains(key) || !passive::verify(facts, tree)? {
                return Err(error);
            }
            passive_dependencies = true;
        }
        match item {
            syn::Item::Struct(item) => structure(facts, item, copies.contains(key))?,
            syn::Item::Enum(item) => scalar_enum(facts, item)?,
            _ => unreachable!(),
        }
    }

    for (key, tree) in trees {
        if !copies.contains(key) {
            borrowed::verify(facts, tree, &copies)?;
        }
    }
    Ok(Selection {
        copies,
        passive_dependencies,
    })
}

fn declaration_attributes<'a>(
    item: &'a syn::Item,
    tree: &TypeTree,
) -> Result<&'a [syn::Attribute], String> {
    match item {
        syn::Item::Struct(item) => Ok(&item.attrs),
        syn::Item::Enum(item)
            if matches!(
                tree,
                TypeTree::Enum { .. } | TypeTree::FieldlessEnum { .. } | TypeTree::MixedEnum { .. }
            ) =>
        {
            Ok(&item.attrs)
        }
        _ => Err("Selected declaration handle has the wrong nominal kind".into()),
    }
}

fn verify_type(facts: &HirFnFacts, tree: &TypeTree) -> Result<(), String> {
    if matches!(tree, TypeTree::MixedEnum { .. }) && !supports(facts, "rust.borrowed-mixed-owners")?
    {
        return Err("Mixed enum child requires its registered borrowed-owner capability".into());
    }
    if matches!(tree, TypeTree::Enum { .. }) && !supports(facts, "rust.scalar-enum-values")? {
        return Err("Scalar enum ownership requires its registered capability".into());
    }
    if matches!(tree, TypeTree::FieldlessEnum { .. })
        && (!supports(facts, "rust.fieldless-enum-values")?
            || facts.build.pointer_width.value != Some(64))
    {
        return Err("Fieldless enum requires its registered ownership capability".into());
    }

    if tree
        .nominal_fields()
        .any(|field| !supported_field(facts, &field.ty))
    {
        return Err("Mixed-unit Copy fields require supported complete values".into());
    }
    Ok(())
}

fn checked_copy(
    facts: &HirFnFacts,
    origins: &Origins,
    tree: &TypeTree,
    attributes: &[syn::Attribute],
) -> Result<bool, String> {
    let (nominal, _) = tree.nominal_parts().ok_or("Expected nominal declaration")?;
    let origin = origins
        .declarations
        .iter()
        .find(|origin| &origin.nominal == nominal)
        .ok_or("Missing selected declaration origin")?;
    if let Some(copy) = &origin.builtin_copy {
        crate::declaration_origins::copy::verify(facts, origin, copy, attributes)?;
        Ok(true)
    } else if matches!(tree, TypeTree::Struct { .. }) {
        Ok(false)
    } else {
        Err("Scalar enum requires positive builtin Copy evidence".into())
    }
}

fn supported_field(facts: &HirFnFacts, tree: &TypeTree) -> bool {
    match tree {
        TypeTree::Scalar(_) | TypeTree::Struct { .. } => true,
        TypeTree::FieldlessEnum { .. } => {
            supports(facts, "rust.fieldless-enum-values").unwrap_or(false)
        }
        TypeTree::Enum { .. } => supports(facts, "rust.scalar-enum-values").unwrap_or(false),
        TypeTree::MixedEnum { .. } => {
            supports(facts, "rust.borrowed-mixed-owners").unwrap_or(false)
        }
        TypeTree::Shared(referent)
            if supports(facts, "rust.borrowed-mixed-owners").unwrap_or(false) =>
        {
            matches!(referent.as_ref(), TypeTree::Slice(element) if matches!(element.as_ref(), TypeTree::Scalar(value) if concrete_types::scalar(value) && value != "unit"))
        }
        TypeTree::Array { element, .. } => {
            supports(facts, "rust.copy-array-fields").unwrap_or(false)
                && matches!(element.as_ref(), TypeTree::Scalar(_))
        }
        _ => false,
    }
}

fn structure(facts: &HirFnFacts, item: &syn::ItemStruct, copy: bool) -> Result<(), String> {
    let transparent = copy
        && item.fields.len() == 1
        && item.generics.params.is_empty()
        && supports(facts, "rust.borrowed-mixed-owners")?;
    nominal_attributes(facts, &item.generics, &item.attrs, transparent)?;
    for field in &item.fields {
        function_attributes(facts, &field.attrs)?;
    }
    Ok(())
}

fn scalar_enum(facts: &HirFnFacts, item: &syn::ItemEnum) -> Result<(), String> {
    nominal_attributes(facts, &item.generics, &item.attrs, false)?;
    for variant in &item.variants {
        function_attributes(facts, &variant.attrs)?;
        for field in &variant.fields {
            function_attributes(facts, &field.attrs)?;
        }
    }
    Ok(())
}

fn nominal_attributes(
    facts: &HirFnFacts,
    generics: &syn::Generics,
    attributes: &[syn::Attribute],
    transparent: bool,
) -> Result<(), String> {
    if !generics.params.is_empty() && !supports(facts, "rust.borrowed-mixed-owners")?
        || crate::concrete_types::lifetimes::parameter_count(generics).is_err()
    {
        return Err("Selected declaration requires its bounded nominal lifetime contract".into());
    }
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);

    for attribute in configuration.effective_attributes(attributes)? {
        if attribute.path().is_ident("derive") {
            derive(&attribute)?;
            continue;
        }
        if transparent
            && attribute.path().is_ident("repr")
            && matches!(&attribute, syn::Meta::List(list)
                if list.parse_args::<syn::Ident>().is_ok_and(|ident| ident == "transparent"))
        {
            continue;
        }
        ordinary(&attribute)?;
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
        if crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.scalar-from-conversions")
            && inline_attribute(&attribute)
        {
            continue;
        }
        ordinary(&attribute)?;
    }
    Ok(())
}

fn inline_attribute(attribute: &syn::Meta) -> bool {
    if !attribute.path().is_ident("inline") {
        return false;
    }
    match attribute {
        syn::Meta::Path(_) => true,
        syn::Meta::List(list) => syn::parse2::<syn::Ident>(list.tokens.clone())
            .ok()
            .is_some_and(|name| name == "always" || name == "never"),
        _ => false,
    }
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
    module_item_in(item, configuration, false)
}

pub(super) fn module_item_in(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
    macro_imports: bool,
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
        } else if macro_imports
            && matches!(item, syn::Item::Mod(_))
            && matches!(&attribute, syn::Meta::Path(path) if path.is_ident("macro_use"))
        {
            // Imports macro names from an independently closed source module.
            // Actual invocations and shadowing definitions remain checked.
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
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    if supports(facts, "rust.closed-scalar-module-dependencies")?
        && scalar_leaf::verify(facts, environment, namespace)?
    {
        return imports::verify_leaf(facts, environment, namespace);
    }
    if environment
        .index
        .unresolved_expansions
        .iter()
        .any(|candidate| namespace.starts_with(candidate))
    {
        if supports(facts, "rust.scoped-nominal-imports")? {
            return imports::verify(facts, environment, namespace).map_err(|error| {
                format!("Selected source has unresolved lexical expansion/import effects: {error}")
            });
        }
        return Err("Selected source has unresolved lexical expansion/import effects".into());
    }
    Ok(())
}

pub(super) fn implementation(facts: &HirFnFacts, item: &syn::ItemImpl) -> bool {
    item.trait_.is_none()
        && item.unsafety.is_none()
        && (if supports(facts, "rust.borrowed-mixed-owners").unwrap_or(false) {
            crate::inherent_calls::lifetimes::implementation(item)
        } else {
            item.generics.params.is_empty() && item.generics.where_clause.is_none()
        })
        && function_attributes(facts, &item.attrs).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn inline_owner_attributes_require_the_exact_builtin_grammar() {
        for tokens in ["#[inline]", "#[inline(always)]", "#[inline(never)]"] {
            let item: syn::ItemFn = syn::parse_str(&format!("{tokens} fn f() {{}} ")).unwrap();
            assert!(super::inline_attribute(&item.attrs[0].meta), "{tokens}");
        }
        for tokens in [
            "#[inline()]",
            "#[inline(sometimes)]",
            "#[inline(always, never)]",
            "#[inline = \"always\"]",
            "#[custom::inline]",
            "#[cold]",
        ] {
            let item: syn::ItemFn = syn::parse_str(&format!("{tokens} fn f() {{}} ")).unwrap();
            assert!(!super::inline_attribute(&item.attrs[0].meta), "{tokens}");
        }
    }
}
