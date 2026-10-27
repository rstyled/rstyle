//! Exact nominal and core/primitive names in their owning checked lexical scopes.
use super::*;

pub(super) fn verify(
    authority: &SourceAuthority<'_>,
    record: &ProductConstant,
    source: &SourceSelection<'_>,
) -> Result<(), String> {
    let namespace = selection::namespace(record)?;
    let (nominal, _) = record
        .ty
        .nominal_parts()
        .ok_or("Missing nominal constant owner")?;
    for unit in [&source.constant_unit, &source.constructor_unit] {
        let syn::Type::Path(path) = unit.implementation.self_ty.as_ref() else {
            return Err("Product constant impl lacks its original lexical nominal owner".into());
        };
        let actual = Scoped {
            resolution: authority.resolution,
            namespace: &namespace,
        }
        .resolve(path)?;
        if actual != record.ty {
            return Err("Product constant original impl selects a different nominal owner".into());
        }
    }
    if record.definition.path != format!("{}::{}", nominal.definition_path, source.constant.ident)
        || record.constructor.definition.path
            != format!(
                "{}::{}",
                nominal.definition_path, source.constructor.sig.ident
            )
    {
        return Err(
            "Product constant selected members differ from original nominal source paths".into(),
        );
    }

    selected::check_function_namespace(authority.facts, authority.resolution, &namespace)?;
    crate::data_library::verify_sysroot_roots(
        authority.facts.facts(),
        &["core"],
        "Product constant initializer",
    )?;
    roots(authority, &namespace)
}

fn roots(authority: &SourceAuthority<'_>, namespace: &Namespace) -> Result<(), String> {
    let environment = &authority.resolution.environment;
    let configuration = crate::conditional_source::Configuration::new(&authority.facts.build.cfgs);
    module_roots(environment, namespace, &configuration)?;
    imported_roots(environment, namespace, &configuration)
}

fn module_roots(
    environment: &Environment,
    namespace: &Namespace,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    for module in environment
        .modules
        .iter()
        .filter(|module| namespace.starts_with(&module.namespace))
    {
        for item in &module.items {
            if !configuration.active(crate::conditional_source::item_attributes(item))? {
                continue;
            }
            for name in ["core", "usize", "u32"] {
                if builtin_options::type_name(item).as_deref() == Some(name)
                    && !core_extern(item, name)
                {
                    return Err(
                        "Product constant initializer has a shadowed core or primitive root".into(),
                    );
                }
            }
        }
    }

    Ok(())
}

fn imported_roots(
    environment: &Environment,
    namespace: &Namespace,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    for ((scope, name), target) in &environment.index.imports {
        if !namespace.starts_with(scope) {
            continue;
        }
        if ["core", "usize", "u32"].contains(&name.as_str()) {
            return Err(
                "Product constant initializer has an imported core or primitive root".into(),
            );
        }
        if name.starts_with("*::") {
            for root in ["core", "usize", "u32"] {
                selected::glob_excludes(environment, target, root, configuration)?;
            }
        }
    }

    Ok(())
}

fn core_extern(item: &syn::Item, name: &str) -> bool {
    matches!(item, syn::Item::ExternCrate(item) if name == "core" && item.ident == "core" && item.rename.is_none())
}

pub(super) fn shadows(item: &syn::Item, name: &str) -> bool {
    if builtin_options::type_name(item).as_deref() == Some(name) {
        return true;
    }
    match item {
        syn::Item::Use(item) => imported(&item.tree, name, None),
        syn::Item::Fn(item) => item.sig.ident == name,
        syn::Item::Const(item) => item.ident == name,
        syn::Item::Static(item) => item.ident == name,
        syn::Item::Macro(item) => item.ident.is_none(),
        _ => false,
    }
}

fn imported(tree: &syn::UseTree, expected: &str, parent: Option<&syn::Ident>) -> bool {
    match tree {
        syn::UseTree::Name(item) => {
            let name = if item.ident == "self" {
                parent.unwrap_or(&item.ident)
            } else {
                &item.ident
            };
            name == expected
        }
        syn::UseTree::Rename(item) => item.rename == expected,
        syn::UseTree::Path(item) => imported(&item.tree, expected, Some(&item.ident)),
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| imported(item, expected, parent)),
        syn::UseTree::Glob(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_roots_and_generated_local_bindings_are_never_assumed_harmless() {
        for source in [
            "type SmallIndex = Other;",
            "use other::SmallIndex;",
            "use other::{self as SmallIndex};",
            "use other::*;",
            "unknown!();",
            "fn SmallIndex() {}",
        ] {
            let item: syn::Item = syn::parse_str(source).unwrap();
            assert!(shadows(&item, "SmallIndex"), "{source}");
        }
        let harmless: syn::Item = syn::parse_str("use other::Different;").unwrap();
        assert!(!shadows(&harmless, "SmallIndex"));
    }
}
