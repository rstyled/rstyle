//! Bounded local name imports, using the existing closed selected module census.
use super::*;

pub(super) fn verify(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    verify_in(facts, environment, namespace, true)
}

pub(super) fn verify_leaf(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<(), String> {
    verify_in(facts, environment, namespace, false)
}

fn verify_in(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
    ancestors: bool,
) -> Result<(), String> {
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
            module_item_with_imports(item, module, &configuration)?;
            if ancestors || !verified_macro_import(item, module) {
                macro_import(item, module, environment, &configuration)?;
            }
        }
    }
    for ((scope, name), target) in &environment.index.imports {
        if !relevant(namespace, scope, ancestors) {
            continue;
        }
        if name.starts_with("*::") {
            if !ancestors {
                return Err("Scalar leaf has an unresolved local glob import".into());
            }
            glob(environment, target, &configuration)?;
        } else if protected(name) || !ancestors && matches!(name.as_str(), "f32" | "f64") {
            return Err(
                "Selected import shadows a primitive, library root or builtin derive".into(),
            );
        }
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

fn protected(name: &str) -> bool {
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
    let module = modules
        .next()
        .ok_or("Import target lacks a closed local source module")?;
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
    for item in &local_module(environment, target)?.items {
        module_item(item, configuration)?;
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
