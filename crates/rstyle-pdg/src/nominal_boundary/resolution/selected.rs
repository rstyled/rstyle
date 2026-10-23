//! Demand-specific nominal proof in a complete, cfg-selected mixed-content unit.
use super::*;
use crate::declaration_origins::Origins;
mod borrowed;
mod core_imports;
mod enum_globs;
mod imports;
pub(super) use imports::glob_excludes;
#[cfg(test)]
mod explicit_u8_attacks;
#[cfg(test)]
mod function_cleanup_tests;
pub(super) mod local_traits;
mod metadata;
pub(super) mod no_drop;
pub(super) mod passive;
mod scalar_leaf;

#[derive(Default)]
pub(super) struct Selection {
    pub copies: std::collections::BTreeSet<Namespace>,
    pub no_drop: std::collections::BTreeSet<Namespace>,
    pub passive_dependencies: bool,
}

pub(super) fn verify(
    facts: &HirFnFacts,
    origins: &Origins,
    resolution: &Resolution,
    owner: Option<&crate::product_instances::CheckedProjection>,
) -> Result<Selection, String> {
    let trees = &resolution.trees;
    let environment = &resolution.environment;
    if trees.is_empty() && !supports(facts, "rust.accelerator-packing")? {
        return Err("Mixed-unit boundary requires selected Copy declarations".into());
    }
    if let Some(owner) = owner {
        if owner.facts() != facts
            || !supports(facts, crate::product_instances::activation::CAPABILITY)?
        {
            return Err("Product source receipt requires its exact owning active instance".into());
        }
        owner.copy_source().verify_owner(facts, environment)?;
    }
    let checker = DeclarationChecker {
        owner,
        facts,
        origins,
        environment,
        resolution,
    };
    let mut selection = Selection::default();
    for (key, tree) in trees {
        selection.record(key, checker.verify(key, tree)?);
    }

    for (key, tree) in trees {
        if !selection.copies.contains(key) && !selection.no_drop.contains(key) {
            borrowed::verify(facts, tree, &selection.copies)?;
        }
    }
    Ok(selection)
}

struct CheckedDeclaration {
    copy: bool,
    no_drop: bool,
    passive_dependencies: bool,
}

impl Selection {
    fn record(&mut self, key: &Namespace, checked: CheckedDeclaration) {
        if checked.copy {
            self.copies.insert(key.clone());
        }
        if checked.no_drop {
            self.no_drop.insert(key.clone());
        }
        self.passive_dependencies |= checked.passive_dependencies;
    }
}

struct DeclarationChecker<'a> {
    facts: &'a HirFnFacts,
    origins: &'a Origins,
    environment: &'a Environment,
    resolution: &'a Resolution,
    owner: Option<&'a crate::product_instances::CheckedProjection>,
}

impl DeclarationChecker<'_> {
    fn verify(&self, key: &Namespace, tree: &TypeTree) -> Result<CheckedDeclaration, String> {
        verify_type(self.facts, tree)?;
        let declaration = self
            .environment
            .index
            .declarations
            .get(key)
            .ok_or("Missing selected struct declaration")?;
        let item =
            &self.environment.modules[declaration.module_index].items[declaration.item_index];
        let attributes = declaration_attributes(item, tree)?;
        let copy = checked_copy(self.facts, self.origins, tree, attributes)?;
        let source_closure = self.cleanup_certificate(tree, copy)?;
        let passive_dependencies = match check_namespace_with_closure(
            self.facts,
            self.environment,
            &declaration.namespace,
            source_closure.as_ref(),
        ) {
            Ok(()) => false,
            Err(error) => {
                if !copy || !passive::verify(self.facts, tree)? {
                    return Err(error);
                }
                true
            }
        };

        match item {
            syn::Item::Struct(item) => structure(self.facts, item, copy)?,
            syn::Item::Enum(item) => {
                explicit_u8_owner(self.facts, item, tree)?;
                scalar_enum(self.facts, item)?;
            }
            _ => unreachable!(),
        }
        Ok(CheckedDeclaration {
            copy,
            no_drop: source_closure.is_some(),
            passive_dependencies,
        })
    }
}

impl DeclarationChecker<'_> {
    fn cleanup_certificate(
        &self,
        tree: &TypeTree,
        copy: bool,
    ) -> Result<Option<no_drop::CheckedNoDrop>, String> {
        let retained = self
            .owner
            .and_then(|owner| owner.copy_source().declaration(tree));
        let checked = if let Some(checked) = retained {
            checked.clone()
        } else {
            if copy || !requested_no_drop(self.facts, tree)? {
                return Ok(None);
            }
            if super::fresh_products::eligible(tree) {
                no_drop::reference_products::verify(
                    self.facts,
                    tree,
                    self.resolution,
                    self.origins,
                )?
            } else {
                no_drop::verify(self.facts, tree, self.environment)?
            }
        };
        if tree.nominal_parts().map(|(nominal, _)| nominal) != Some(checked.nominal()) {
            return Err("Cleanup certificate identifies another declaration".into());
        }
        Ok(Some(checked))
    }
}

fn requested_no_drop(facts: &HirFnFacts, tree: &TypeTree) -> Result<bool, String> {
    if !supports(facts, "rust.fresh-concrete-results")? {
        return Ok(false);
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured { ty } = &interface.result else {
        return Ok(false);
    };
    if let TypeTree::Result { ok, .. } = ty
        && supports(facts, super::fresh_products::CAPABILITY)?
        && ok.as_ref() == tree
        && super::fresh_products::eligible(tree)
    {
        return Ok(true);
    }
    let error = match ty {
        TypeTree::Result { err, .. } => err.as_ref(),
        TypeTree::Struct { .. } => ty,
        _ => return Ok(false),
    };
    if error == tree
        && supports(facts, no_drop::enum_errors::CAPABILITY)?
        && no_drop::enum_errors::supported(tree)
    {
        return Ok(true);
    }
    Ok(error == tree
        && matches!(tree, TypeTree::Struct { fields, .. }
        if fields.len() == 1 && fields[0].name == "0" && fields[0].ty == TypeTree::Scalar("unit".into())))
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
    } else if matches!(tree, TypeTree::Struct { .. }) || requested_no_drop(facts, tree)? {
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
        TypeTree::Option(payload) => {
            supports(facts, super::fresh_products::CAPABILITY).unwrap_or(false)
                && matches!(payload.as_ref(), TypeTree::Scalar(token)
                    if concrete_types::scalar(token) && token != "unit")
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
    nominal_attributes(
        facts,
        &item.generics,
        &item.attrs,
        AttributeOwner::Struct { transparent },
    )?;
    for field in &item.fields {
        function_attributes(facts, &field.attrs)?;
    }
    Ok(())
}

fn scalar_enum(facts: &HirFnFacts, item: &syn::ItemEnum) -> Result<(), String> {
    let explicit = supports(facts, concrete_types::EXPLICIT_U8_CAPABILITY)?
        && item
            .attrs
            .iter()
            .any(|attr| concrete_types::explicit_u8_repr(&attr.meta));
    if explicit {
        concrete_types::explicit_u8_fieldless_declaration(item)?;
    }

    nominal_attributes(
        facts,
        &item.generics,
        &item.attrs,
        AttributeOwner::Enum {
            explicit_u8: explicit,
        },
    )?;
    for variant in &item.variants {
        for attribute in &variant.attrs {
            if !(explicit && concrete_types::explicit_u8_variant_expect(&attribute.meta)) {
                function_attributes(facts, std::slice::from_ref(attribute))?;
            }
        }
        for field in &variant.fields {
            function_attributes(facts, &field.attrs)?;
        }
    }
    Ok(())
}

fn explicit_u8_owner(
    facts: &HirFnFacts,
    item: &syn::ItemEnum,
    tree: &TypeTree,
) -> Result<(), String> {
    let TypeTree::FieldlessEnum {
        source, variants, ..
    } = tree
    else {
        return Ok(());
    };
    if concrete_types::fieldless_variants(source).is_ok()
        && !item.attrs.iter().any(|attr| attr.path().is_ident("repr"))
    {
        return Ok(());
    }
    if !supports(facts, concrete_types::EXPLICIT_U8_CAPABILITY)?
        || concrete_types::explicit_u8_fieldless_declaration(item)? != *variants
    {
        return Err("Fieldless enum lacks exact complete explicit u8 source proof".into());
    }
    Ok(())
}

enum AttributeOwner {
    Struct { transparent: bool },
    Enum { explicit_u8: bool },
}

fn nominal_attributes(
    facts: &HirFnFacts,
    generics: &syn::Generics,
    attributes: &[syn::Attribute],
    owner: AttributeOwner,
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
        if matches!(owner, AttributeOwner::Struct { transparent: true })
            && attribute.path().is_ident("repr")
            && matches!(&attribute, syn::Meta::List(list)
                if list.parse_args::<syn::Ident>().is_ok_and(|ident| ident == "transparent"))
        {
            continue;
        }
        if matches!(owner, AttributeOwner::Enum { explicit_u8: true })
            && concrete_types::explicit_u8_repr(&attribute)
        {
            continue;
        }
        if matches!(owner, AttributeOwner::Enum { .. })
            && supports(facts, crate::borrowed_result_types::CAPABILITY)?
            && non_exhaustive_attribute(&attribute)
        {
            continue;
        }
        ordinary(&attribute)?;
    }
    Ok(())
}

fn non_exhaustive_attribute(attribute: &syn::Meta) -> bool {
    attribute.path().is_ident("non_exhaustive") && matches!(attribute, syn::Meta::Path(_))
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
        if supports(facts, concrete_types::EXPLICIT_U8_CAPABILITY)?
            && metadata::deprecated(&attribute)
        {
            continue;
        }
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

/// Reuse only the exact external declaration already proved by census closure.
pub(super) fn module_item_with_imports(
    item: &syn::Item,
    owner: &census::Module,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    if let syn::Item::ExternCrate(import) = item
        && owner
            .verified_external_imports
            .contains(&import.ident.to_string())
    {
        return Ok(());
    }
    module_item_in(item, configuration, true)
}

pub(super) fn module_item_for(
    facts: &HirFnFacts,
    item: &syn::Item,
    owner: &census::Module,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    if !supports(facts, concrete_types::EXPLICIT_U8_CAPABILITY)?
        || matches!(item, syn::Item::ExternCrate(_))
    {
        return module_item_with_imports(item, owner, configuration);
    }
    module_item_policy_for(
        item,
        configuration,
        ItemPolicy {
            macro_imports: true,
            test_marker: false,
            deprecated_expect: true,
        },
    )
}

pub(super) fn module_item_in(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
    macro_imports: bool,
) -> Result<(), String> {
    module_item_policy(item, configuration, macro_imports, false)
}

fn module_item_policy(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
    macro_imports: bool,
    test_marker: bool,
) -> Result<(), String> {
    module_item_policy_for(
        item,
        configuration,
        ItemPolicy {
            macro_imports,
            test_marker,
            deprecated_expect: false,
        },
    )
}

struct ItemPolicy {
    macro_imports: bool,
    test_marker: bool,
    deprecated_expect: bool,
}

fn module_item_policy_for(
    item: &syn::Item,
    configuration: &crate::conditional_source::Configuration<'_>,
    policy: ItemPolicy,
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
        module_attribute(item, &attribute, &policy)?;
    }
    Ok(())
}

fn module_attribute(
    item: &syn::Item,
    attribute: &syn::Meta,
    policy: &ItemPolicy,
) -> Result<(), String> {
    if attribute.path().is_ident("derive") {
        derive(attribute)?;
    } else if policy.deprecated_expect && census::module_lints::deprecated_expect(attribute) {
        // Exact lint-only metadata changes no source, export or effects.
    } else if policy.macro_imports
        && matches!(item, syn::Item::Mod(_))
        && matches!(attribute, syn::Meta::Path(path) if path.is_ident("macro_use"))
    {
        // Imports macro names from an independently closed source module.
        // Actual invocations and shadowing definitions remain checked.
    } else if policy.test_marker
        && matches!(item, syn::Item::Fn(_))
        && matches!(attribute, syn::Meta::Path(path) if path.is_ident("test"))
    {
        // Export-name proof only; the newcap cleanup visitor independently
        // verifies builtin authority and visits the entire original body.
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
        ordinary(attribute)?;
    }
    Ok(())
}

pub(super) fn check_namespace(
    input: &dyn crate::unit_store::EvidenceFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    let source_closure = function_cleanup_certificate(input, environment, namespace)?;
    check_namespace_with_closure(
        input.facts(),
        environment,
        namespace,
        source_closure.as_ref(),
    )
}

fn function_cleanup_certificate(
    input: &dyn crate::unit_store::EvidenceFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<Option<no_drop::CheckedNoDrop>, String> {
    let facts = input.facts();
    if crate::product_instances::activation::verify(input)? {
        let owner = input
            .product_instance()
            .ok_or("Missing checked product owner")?;
        owner.copy_source().verify_owner(facts, environment)?;
        return owner
            .copy_source()
            .function_namespace(facts, environment, namespace)
            .map(|certificate| certificate.cloned());
    }
    if crate::extraction_profile::facts_support(input, crate::range_values::CONSUMING_CAPABILITY)
        && crate::range_values::consuming_input(input)
    {
        crate::range_values::consuming::verify_fields(input)?;
        let source = super::checked_copy_product_source(facts)?;
        return source
            .function_namespace(facts, environment, namespace)
            .map(|certificate| certificate.cloned());
    }
    if !supports(facts, crate::borrowed_result_types::CAPABILITY)? {
        return Ok(None);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Ok(None);
    };

    if !crate::borrowed_result_types::tree_result(ok, err) || !requested_no_drop(facts, err)? {
        return Ok(None);
    }
    // Reuse the same exact declaration/whole-unit certificate selected above.
    // This proves no function effects; only the coarse expansion flag changes.
    no_drop::verify(facts, err, environment).map(Some)
}

pub(super) struct LexicalGlob<'a> {
    pub namespace: &'a Namespace,
    pub path: &'a [String],
}

pub(super) fn lexical_glob_excludes(
    facts: &HirFnFacts,
    environment: &Environment,
    original: LexicalGlob<'_>,
    name: &str,
) -> Result<bool, String> {
    if function_cleanup_certificate(facts, environment, original.namespace)?.is_none() {
        return Ok(false);
    }
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    let target = no_drop::module_globs::lexical_target(
        environment,
        original.namespace,
        original.path,
        &configuration,
    )?;

    imports::checked_glob_excludes(environment, &target, name)?;
    Ok(true)
}

fn check_namespace_with_closure(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
    source_closure: Option<&no_drop::CheckedNoDrop>,
) -> Result<(), String> {
    if supports(facts, "rust.closed-scalar-module-dependencies")?
        && scalar_leaf::verify(facts, environment, namespace)?
    {
        return imports::verify_leaf(facts, environment, namespace);
    }
    // A private exact-owner certificate replaces only this coarse expansion
    // flag. Scalar-leaf imports above and all other ownership checks remain.
    if source_closure.is_none()
        && environment
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
    fn non_exhaustive_error_enum_marker_requires_exact_builtin_grammar() {
        for (tokens, expected) in [
            ("#[non_exhaustive]", true),
            ("#[non_exhaustive()]", false),
            ("#[non_exhaustive = true]", false),
            ("#[custom::non_exhaustive]", false),
            ("#[custom]", false),
        ] {
            let item: syn::ItemEnum =
                syn::parse_str(&format!("{tokens} enum Error {{ Missing }}")).unwrap();
            assert_eq!(
                super::non_exhaustive_attribute(&item.attrs[0].meta),
                expected
            );
        }
    }

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
