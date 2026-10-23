//! Bounded local name imports, using the existing closed selected module census.
use super::*;

pub(super) fn verify(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    verify_in(
        environment,
        namespace,
        Scope {
            facts,
            ancestors: true,
            closure: None,
        },
    )
}

pub(super) fn verify_leaf(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    verify_in(
        environment,
        namespace,
        Scope {
            facts,
            ancestors: false,
            closure: None,
        },
    )
}

pub(super) fn verify_source(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
    closure: &no_drop::CheckedNoDrop,
) -> Result<(), String> {
    verify_in(
        environment,
        namespace,
        Scope {
            facts,
            ancestors: true,
            closure: Some(closure),
        },
    )
}

struct Scope<'a> {
    facts: &'a HirFnFacts,
    ancestors: bool,
    closure: Option<&'a no_drop::CheckedNoDrop>,
}

impl Scope<'_> {
    fn item(
        &self,
        item: &syn::Item,
        owner: &census::Module,
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        let facts = self.facts;
        if self.closure.is_none() || matches!(item, syn::Item::ExternCrate(_)) {
            return module_item_for(facts, item, owner, configuration);
        }
        module_item_policy_for(
            item,
            configuration,
            ItemPolicy {
                macro_imports: true,
                test_marker: false,
                deprecated_expect: true,
                source_closure: self.closure,
            },
        )
    }
}

fn verify_in(
    environment: &Environment,
    namespace: &[String],
    scope: Scope<'_>,
) -> Result<(), String> {
    let facts = scope.facts;
    let ancestors = scope.ancestors;
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    let control = facts
        .signature
        .params
        .iter()
        .any(|parameter| parameter.ty.normalized.as_deref() == Some("i32"));

    for module in environment
        .modules
        .iter()
        .filter(|m| relevant(namespace, &m.namespace, ancestors))
    {
        for item in &module.items {
            if !ancestors && shadows_float(item, control) {
                return Err("Scalar leaf primitive name has a local declaration".into());
            }
            scope.item(item, module, &configuration)?;
            if ancestors || !verified_macro_import(item, module) {
                macro_import(item, module, environment, &configuration)?;
            }
        }
    }
    for ((scope, name), target) in &environment.index.imports {
        if !relevant(namespace, scope, ancestors) {
            continue;
        }
        verify_binding(
            facts,
            environment,
            enum_globs::Original {
                namespace: scope,
                key: name,
                target,
            },
            ancestors,
        )?;
    }
    Ok(())
}

fn verify_binding(
    facts: &HirFnFacts,
    environment: &Environment,
    original: enum_globs::Original<'_>,
    ancestors: bool,
) -> Result<(), String> {
    let enum_globs::Original {
        namespace: scope,
        key: name,
        target,
    } = original;
    if name.starts_with("*::") {
        if !ancestors {
            return Err("Scalar leaf has an unresolved local glob import".into());
        }
        if !enum_globs::verify(facts, environment, original)? {
            let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
            glob(environment, target, &configuration)?;
        }
    } else if (protected(name) || !ancestors && matches!(name, "f32" | "f64"))
        && !core_imports::debug(facts, environment, (scope, name, target))?
    {
        return Err("Selected import shadows a primitive, library root or builtin derive".into());
    }
    Ok(())
}

fn relevant(namespace: &[String], scope: &[String], ancestors: bool) -> bool {
    namespace == scope || ancestors && namespace.starts_with(scope)
}

fn shadows_float(item: &syn::Item, control: bool) -> bool {
    let name = match item {
        syn::Item::Type(item) => &item.ident,
        syn::Item::Struct(item) => &item.ident,
        syn::Item::Enum(item) => &item.ident,
        syn::Item::Trait(item) => &item.ident,
        syn::Item::Union(item) => &item.ident,
        syn::Item::Mod(item) => &item.ident,
        syn::Item::ExternCrate(item) => item.rename.as_ref().map_or(&item.ident, |(_, name)| name),
        _ => return false,
    };
    name == "f32" || name == "f64" || control && name == "i32"
}

fn verified_macro_import(item: &syn::Item, owner: &census::Module) -> bool {
    matches!(item, syn::Item::Mod(module) if owner.verified_macro_imports.contains(&module.ident.to_string()))
}

pub(super) fn protected(name: &str) -> bool {
    builtin_derive(name) || concrete_types::scalar(name) || matches!(name, "str" | "core" | "std")
}

fn local_module<'a>(
    environment: &'a Environment,
    target: &[String],
) -> Result<&'a census::Module, String> {
    let mut modules = environment
        .modules
        .iter()
        .filter(|module| module.namespace == target);
    let module = modules.next().ok_or_else(|| {
        format!(
            "Import target lacks a closed local source module: {}",
            target.join("::")
        )
    })?;
    if modules.next().is_some() {
        return Err("Import target has ambiguous local source modules".into());
    }
    Ok(module)
}

fn macro_import(
    item: &syn::Item,
    owner: &census::Module,
    environment: &Environment,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    let syn::Item::Mod(item) = item else {
        return Ok(());
    };
    if !configuration
        .effective_attributes(&item.attrs)?
        .iter()
        .any(|a| a.path().is_ident("macro_use"))
    {
        return Ok(());
    }

    let target: Vec<_> = owner
        .namespace
        .iter()
        .cloned()
        .chain([item.ident.to_string()])
        .collect();
    for item in &local_module(environment, &target)?.items {
        module_item(item, configuration)?;
        let syn::Item::Macro(item) = item else {
            return Err("Macro import requires a closed module of local macro definitions".into());
        };
        if !item.mac.path.is_ident("macro_rules")
            || item
                .ident
                .as_ref()
                .is_none_or(|name| protected(&name.to_string()))
        {
            return Err("Imported macro can shadow relevant source names".into());
        }
    }
    Ok(())
}

fn glob(
    environment: &Environment,
    target: &[String],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    glob_items(
        local_module(environment, target)?.items.iter(),
        configuration,
    )
}

pub(super) fn glob_items<'a>(
    items: impl IntoIterator<Item = &'a syn::Item>,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    glob_items_policy(items, configuration, false)
}

pub(super) fn glob_items_policy<'a>(
    items: impl IntoIterator<Item = &'a syn::Item>,
    configuration: &crate::conditional_source::Configuration<'_>,
    test_marker: bool,
) -> Result<(), String> {
    for item in items {
        module_item_policy(item, configuration, false, test_marker)?;
        let (name, visibility) = match item {
            syn::Item::Use(item) if !matches!(item.vis, syn::Visibility::Inherited) => {
                return Err("Glob target contains an unresolved public reexport".into());
            }
            syn::Item::Struct(item) => (&item.ident, &item.vis),
            syn::Item::Enum(item) => (&item.ident, &item.vis),
            syn::Item::Type(item) => (&item.ident, &item.vis),
            syn::Item::Trait(item) => (&item.ident, &item.vis),
            syn::Item::Union(item) => (&item.ident, &item.vis),
            syn::Item::Mod(item) => (&item.ident, &item.vis),
            syn::Item::Fn(item) => (&item.sig.ident, &item.vis),
            syn::Item::Const(item) => (&item.ident, &item.vis),
            syn::Item::Static(item) => (&item.ident, &item.vis),
            syn::Item::ExternCrate(item) => (
                item.rename.as_ref().map_or(&item.ident, |(_, name)| name),
                &item.vis,
            ),
            syn::Item::Impl(_) | syn::Item::Use(_) | syn::Item::Macro(_) => continue,
            _ => return Err("Glob target has unsupported export syntax".into()),
        };
        if !matches!(visibility, syn::Visibility::Inherited) && protected(&name.to_string()) {
            return Err("Glob export shadows a primitive, library root or builtin derive".into());
        }
    }
    Ok(())
}

/// Retain the closed glob proof before excluding one additional type binding.
/// Both source namespaces and private/inactive declarations remain blockers.
pub(in crate::nominal_boundary::resolution) fn glob_excludes(
    environment: &Environment,
    target: &[String],
    name: &str,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    glob(environment, target, configuration)?;
    excludes_binding(&local_module(environment, target)?.items, name)
}

/// Caller must supply a separately closed exact original module target.
pub(super) fn checked_glob_excludes(
    environment: &Environment,
    target: &[String],
    name: &str,
) -> Result<(), String> {
    excludes_binding(&local_module(environment, target)?.items, name)
}

fn excludes_binding(items: &[syn::Item], name: &str) -> Result<(), String> {
    source_bindings(items)
        .find(|binding| binding == name)
        .map_or(Ok(()), |_| {
            Err("Selected glob target can shadow the requested source binding".into())
        })
}

/// Both value and type namespaces matter: builtin source lookup also asks about
/// Some/None/Ok/Err. Conservatively retain private and inactive declarations.
fn source_bindings(items: &[syn::Item]) -> impl Iterator<Item = String> {
    items.iter().filter_map(|item| {
        super::super::builtin_options::type_name(item).or_else(|| match item {
            syn::Item::Fn(item) => Some(item.sig.ident.to_string()),
            syn::Item::Const(item) => Some(item.ident.to_string()),
            syn::Item::Static(item) => Some(item.ident.to_string()),
            syn::Item::Macro(item) => item.ident.as_ref().map(ToString::to_string),
            _ => None,
        })
    })
}

/// Source-root binding proof, separate from compiler --extern override checks.
pub(in crate::nominal_boundary::resolution) fn library_root(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
    root: &str,
) -> Result<(), String> {
    crate::data_library::verify_sysroot_roots(facts, &[root], "Selected core import")?;
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);

    for module in environment
        .modules
        .iter()
        .filter(|module| relevant(namespace, &module.namespace, true))
    {
        for item in &module.items {
            if !configuration.active(crate::conditional_source::item_attributes(item))? {
                continue;
            }
            module_item_for(facts, item, module, &configuration)?;
            macro_import(item, module, environment, &configuration)?;
            if super::super::builtin_options::type_name(item).as_deref() == Some(root)
                && !matches!(item, syn::Item::ExternCrate(item) if item.ident == root && item.rename.is_none())
            {
                return Err("Selected library root has a local source binding".into());
            }
        }
    }
    library_root_imports(environment, namespace, root, &configuration)
}

fn library_root_imports(
    environment: &Environment,
    namespace: &[String],
    root: &str,
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    for ((scope, name), target) in &environment.index.imports {
        if !relevant(namespace, scope, true) {
            continue;
        }
        if name == root {
            return Err("Selected library root has an imported source binding".into());
        }
        if name.starts_with("*::") {
            glob(environment, target, configuration)?;
        }
    }
    Ok(())
}

/// Check declarations in the exact lexical scope before using a prelude type.
pub(in crate::nominal_boundary::resolution) fn unshadowed_type_declarations(
    environment: &Environment,
    scope: &[String],
    name: &str,
) -> Result<(), String> {
    for module in environment
        .modules
        .iter()
        .filter(|module| module.namespace == scope)
    {
        for item in &module.items {
            if builtin_options::type_name(item).as_deref() == Some(name) {
                return Err("Prelude type binding is shadowed by an original declaration".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod marker_tests {
    use super::*;

    #[test]
    fn unrelated_glob_declarations_do_not_supply_marker_bindings() {
        let items = syn::parse_file("pub struct Span { pub start: usize, pub end: usize }")
            .unwrap()
            .items;
        assert!(excludes_binding(&items, "Send").is_ok());
        assert!(excludes_binding(&items, "Sync").is_ok());
        for source in [
            "pub trait Send {}",
            "pub type Send = usize;",
            "pub mod Send {}",
            "pub fn Send() {}",
            "pub const Send: usize = 0;",
        ] {
            let items = syn::parse_file(source).unwrap().items;
            assert!(excludes_binding(&items, "Send").is_err());
        }
    }
    #[test]
    fn constructor_names_require_absence_in_the_value_namespace() {
        for (source, name) in [
            ("pub fn Some() {}", "Some"),
            ("pub const None: usize = 0;", "None"),
            ("pub static Ok: usize = 0;", "Ok"),
            ("pub struct Err;", "Err"),
        ] {
            let items = syn::parse_file(source).unwrap().items;
            assert!(excludes_binding(&items, name).is_err());
        }
    }
}
