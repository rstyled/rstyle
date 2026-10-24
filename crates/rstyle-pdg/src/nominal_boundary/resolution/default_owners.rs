//! Exact original static Default owners; ordinary Copy and body checks still apply.
use super::*;
use crate::source_evidence::trait_members::{self, AttributePolicy};

const SOURCE_ATTRIBUTES: AttributePolicy = AttributePolicy {
    inactive: "Default source owner is inactive",
    member: |member| match member {
        syn::ImplItem::Fn(method) => method.attrs.as_slice(),
        _ => &[],
    },
};

pub(super) fn call_owner(
    resolution: &Resolution,
    facts: &HirFnFacts,
    path: &syn::TypePath,
    nominal: &concrete_types::Nominal,
) -> Result<(), String> {
    binding(
        facts,
        &resolution.environment,
        &resolution.namespace,
        &syn::parse_quote!(Default),
    )?;
    resolution.matches_nominal(path, nominal)?;

    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    for module in &resolution.environment.modules {
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            if item.trait_.is_some() || !configuration.active(&item.attrs)? {
                continue;
            }
            if item.items.iter().any(|member| {
                matches!(member, syn::ImplItem::Fn(method)
                if method.sig.ident == "default")
            }) {
                inherent_conflict(resolution, module, item, nominal)?;
            }
        }
    }
    Ok(())
}

fn inherent_conflict(
    resolution: &Resolution,
    module: &census::Module,
    item: &syn::ItemImpl,
    nominal: &concrete_types::Nominal,
) -> Result<(), String> {
    let syn::Type::Path(path) = item.self_ty.as_ref() else {
        return Err("Default invocation has an unresolved competing inherent owner".into());
    };
    let owner = Scoped {
        resolution,
        namespace: &module.namespace,
    }
    .resolve(path)?;
    if owner
        .nominal_parts()
        .is_some_and(|(owner, _)| owner == nominal)
    {
        return Err("Default invocation is shadowed by an original inherent member".into());
    }
    Ok(())
}

pub(super) fn bind(
    input: &dyn crate::unit_store::EvidenceFacts,
    resolution: &mut Resolution,
) -> Result<bool, String> {
    let facts = input.facts();
    if !supports(facts, crate::default_calls::CAPABILITY)?
        || !facts
            .build
            .frontend_options
            .contains_key(crate::default_calls::KEY)
    {
        return Ok(false);
    }

    let (namespace, member) = find(facts, &resolution.environment)?;
    binding(
        facts,
        &resolution.environment,
        &namespace,
        &member.trait_path,
    )?;
    resolution.namespace = namespace;
    resolve_receiver(resolution, &member.self_type, facts)?;
    let syn::Type::Path(path) = &member.self_type else {
        return Err("Default owner lacks a concrete original nominal path".into());
    };
    resolution.matches_nominal(path, &member.nominal)?;

    let owner = resolution.resolve(path)?;
    if !resolution.checked_owned_copy(&owner) {
        return Err("Default owner lacks complete positive Copy source proof".into());
    }
    selected::check_namespace(input, &resolution.environment, &resolution.namespace)?;
    Ok(true)
}

fn find(
    facts: &HirFnFacts,
    environment: &Environment,
) -> Result<(Namespace, crate::default_calls::SourceMember), String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing Default source file")?;
    let mut found = None;
    for module in environment
        .modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        let bytes = environment.source(&module.path)?.bytes();
        for item in &module.items {
            if let syn::Item::Impl(item) = item
                && let Some(member) = crate::default_calls::source_member(facts, item, bytes)?
            {
                trait_members::attributes(
                    facts,
                    item,
                    SOURCE_ATTRIBUTES,
                    selected::function_attributes,
                )?;
                if found.replace((module.namespace.clone(), member)).is_some() {
                    return Err("Ambiguous original Default owner".into());
                }
            }
        }
    }
    found.ok_or_else(|| "Missing exact original Default owner".into())
}

fn binding(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &Namespace,
    path: &syn::Path,
) -> Result<(), String> {
    if !path.is_ident("Default") || !selected::no_drop::default_prelude(facts) {
        return Err("Default owner requires the reviewed direct prelude binding".into());
    }
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    for length in 0..=namespace.len() {
        let scope = &namespace[..length];
        selected::unshadowed_type_declarations(environment, scope, "Default")?;
        for ((owner, name), target) in &environment.index.imports {
            if owner != scope {
                continue;
            }
            if name == "Default" {
                return Err("Default prelude trait has an explicit import binding".into());
            }
            if name.starts_with("*::") {
                selected::glob_excludes(environment, target, "Default", &configuration)?;
            }
        }
    }
    selected::library_root(facts, environment, namespace, "core")
}
