//! Exact original static TryFrom owner; source binding precedes permission selection.
use super::*;
use crate::source_evidence::trait_members::{self, AttributePolicy};

const SOURCE_ATTRIBUTES: AttributePolicy = AttributePolicy {
    inactive: "TryFrom original owner has an inactive member",
    member: |member| match member {
        syn::ImplItem::Fn(item) => item.attrs.as_slice(),
        syn::ImplItem::Type(item) => item.attrs.as_slice(),
        _ => &[],
    },
};

pub(super) fn find(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<(Namespace, crate::tryfrom_calls::SourceMember), String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing TryFrom source file")?;
    let mut found = None;
    for (module_index, module) in resolution
        .environment
        .modules
        .iter()
        .enumerate()
        .filter(|(_, module)| module.path == std::path::Path::new(file))
    {
        let bytes = resolution.environment.source(&module.path)?.bytes();
        for (item, _) in resolution
            .environment
            .owner_candidates(module_index, facts.fn_span)
        {
            if let syn::Item::Impl(item) = item
                && let Some(member) = crate::tryfrom_calls::source_member(
                    facts,
                    item,
                    bytes,
                    &crate::tryfrom_calls::RegistryAuthority,
                )?
            {
                trait_members::attributes(
                    facts,
                    item,
                    SOURCE_ATTRIBUTES,
                    selected::function_attributes,
                )?;
                if found.replace((module.namespace.clone(), member)).is_some() {
                    return Err("Ambiguous original TryFrom owner".into());
                }
            }
        }
    }
    found.ok_or_else(|| "Missing exact original TryFrom owner".into())
}

pub(super) fn lexical(
    facts: &HirFnFacts,
    scope: &Scoped<'_>,
    member: &crate::tryfrom_calls::SourceMember,
    closure: &selected::no_drop::CheckedNoDrop,
) -> Result<(), String> {
    trait_binding(facts, scope, member, closure)?;
    if scope.resolve(&member.self_type)? != member.success
        || scope.resolve(&member.error_type)? != member.error
    {
        return Err("TryFrom source owner or associated Error resolves to another type".into());
    }
    result_owner(facts, scope, &member.result_type, closure)?;
    result_payloads(scope, member)
}

fn result_owner(
    facts: &HirFnFacts,
    scope: &Scoped<'_>,
    path: &syn::TypePath,
    closure: &selected::no_drop::CheckedNoDrop,
) -> Result<(), String> {
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    let root = match names.as_slice() {
        [name] if name == "Result" => None,
        [root, module, name]
            if ["core", "std"].contains(&root.as_str())
                && module == "result"
                && name == "Result" =>
        {
            Some(root)
        }
        _ => return Err("TryFrom result requires direct core/prelude Result source".into()),
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() {
        return Err("TryFrom result has a qualified or absolute source path".into());
    }
    if let Some(root) = root {
        selected::source_library_root(facts, scope, root, closure)?;
    } else {
        unshadowed(
            facts,
            &scope.resolution.environment,
            scope.namespace,
            "Result",
        )?;
    }

    Ok(())
}

fn result_payloads(
    scope: &Scoped<'_>,
    member: &crate::tryfrom_calls::SourceMember,
) -> Result<(), String> {
    let syn::PathArguments::AngleBracketed(arguments) = &member
        .result_type
        .path
        .segments
        .last()
        .ok_or("Missing TryFrom Result source path")?
        .arguments
    else {
        return Err("TryFrom Result lacks original payload arguments".into());
    };
    let arguments: Vec<_> = arguments.args.iter().collect();
    let [
        syn::GenericArgument::Type(ok),
        syn::GenericArgument::Type(error),
    ] = arguments.as_slice()
    else {
        return Err("TryFrom Result requires exactly two original payload types".into());
    };
    for (source, expected) in [(ok, &member.success), (error, &member.error)] {
        if &result_payload(source, member, scope)? != expected {
            return Err(
                "TryFrom Result source payload disagrees with selected concrete interface".into(),
            );
        }
    }

    Ok(())
}

fn result_payload(
    source: &syn::Type,
    member: &crate::tryfrom_calls::SourceMember,
    scope: &Scoped<'_>,
) -> Result<TypeTree, String> {
    let syn::Type::Path(path) = source else {
        return Err("TryFrom Result payload is not an original nominal path".into());
    };
    if path.qself.is_none() && path.path.is_ident("Self") {
        return Ok(member.success.clone());
    }
    if path.qself.is_none()
        && path.path.segments.len() == 2
        && path.path.segments[0].ident == "Self"
        && path.path.segments[1].ident == "Error"
        && path
            .path
            .segments
            .iter()
            .all(|part| part.arguments.is_empty())
    {
        return Ok(member.error.clone());
    }
    scope.resolve(path)
}

fn unshadowed(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &Namespace,
    name: &str,
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    for length in 0..=namespace.len() {
        let scope = &namespace[..length];
        selected::unshadowed_type_declarations(environment, scope, name)?;
        for ((owner, imported), target) in &environment.index.imports {
            if owner != scope {
                continue;
            }
            if imported == name {
                return Err("TryFrom source binding has an explicit shadowing import".into());
            }
            if imported.starts_with("*::") {
                selected::glob_excludes(environment, target, name, &configuration)?;
            }
        }
    }

    Ok(())
}

fn trait_binding(
    facts: &HirFnFacts,
    scope: &Scoped<'_>,
    member: &crate::tryfrom_calls::SourceMember,
    closure: &selected::no_drop::CheckedNoDrop,
) -> Result<(), String> {
    let path = &member.trait_path;
    let names: Vec<_> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let prelude = names == ["TryFrom"];
    let rooted = names == ["core", "convert", "TryFrom"];
    if !(prelude || rooted)
        || prelude
            && (path.leading_colon.is_some()
                || !matches!(facts.build.edition.known_value(), Some("2021" | "2024"))
                || !selected::no_drop::default_prelude(facts))
    {
        return Err("TryFrom requires its reviewed direct core/prelude binding".into());
    }
    for name in [member.input.as_str()]
        .into_iter()
        .chain(prelude.then_some("TryFrom"))
    {
        unshadowed(facts, &scope.resolution.environment, scope.namespace, name)?;
    }

    selected::source_library_root(facts, scope, "core", closure)
}

pub(super) fn bind(
    input: &dyn crate::unit_store::EvidenceFacts,
    resolution: &mut Resolution,
) -> Result<bool, String> {
    let facts = input.facts();
    if !supports(facts, crate::tryfrom_calls::CAPABILITY)?
        || !facts
            .build
            .frontend_options
            .contains_key(crate::tryfrom_calls::KEY)
    {
        return Ok(false);
    }
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    if !matches!(&interface.result,
        Observation::Captured { ty: TypeTree::Result { ok, err } }
            if crate::concrete_results::scalar_errors::GRAMMAR.permits(
                &crate::data_types::TypeToken::from_tree(ok),
                &crate::data_types::TypeToken::from_tree(err)))
    {
        return Ok(false);
    }
    let receipt = resolution
        .scalar_result_source
        .as_ref()
        .ok_or("TryFrom requires its independently checked original source receipt")?;
    let namespace = receipt
        .checked_namespace(facts, &resolution.environment)?
        .to_vec();
    let member = receipt
        .trait_member()
        .ok_or("TryFrom requires its exact checked trait source owner")?;
    if !resolution.checked_owned_copy(&member.success)
        || !member
            .error
            .nominal_parts()
            .is_some_and(|(nominal, _)| resolution.checked_no_drop(nominal))
    {
        return Err("TryFrom owner requires independent Copy-success/no-drop-error proof".into());
    }

    let self_type = member.self_type.clone();
    resolution.namespace = namespace;
    resolve_receiver(resolution, &syn::Type::Path(self_type), facts)?;
    selected::check_function_namespace(input, resolution, &resolution.namespace)?;

    Ok(true)
}

/// Link candidates carry no lexical, cleanup or callable permission. Those checks
/// remain mandatory when a call uses the selected helper.
pub(crate) fn call_link(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Option<serde_json::Value> {
    if let Some(link) = crate::result_maps::trait_leaf::link(node, facts) {
        return Some(link);
    }
    let association = call_association(node, facts)?;

    Some(association_resolution(&association))
}

#[derive(serde::Serialize)]
struct CallResolution<'a> {
    local: bool,
    path: &'a str,
    definition_identity: &'a str,
    #[serde(rename = "crate")]
    crate_name: &'a str,
    tryfrom_requested: &'a str,
}

fn association_resolution(association: &crate::tryfrom_calls::Association) -> serde_json::Value {
    serde_json::to_value(CallResolution {
        local: true,
        path: &association.selected_path,
        definition_identity: &association.selected_identity,
        crate_name: &association.crate_name,
        tryfrom_requested: &association.requested_path,
    })
    .expect("TryFrom resolution contains only primitive fields")
}

fn call_association(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Option<crate::tryfrom_calls::Association> {
    let dispatch = node.dispatch.as_ref()?;
    if node.kind != "expr_call"
        || dispatch.kind != "resolved_direct"
        || ![
            "core::convert::TryFrom::try_from",
            "std::convert::TryFrom::try_from",
        ]
        .contains(&dispatch.op.as_str())
        || dispatch.operand_tys.as_slice()
            != [crate::tryfrom_calls::input::captured(facts.facts()).ok()?]
        || !crate::extraction_profile::facts_support(facts, crate::tryfrom_calls::CAPABILITY)
    {
        return None;
    }
    let association =
        crate::tryfrom_calls::association(node, facts, &crate::tryfrom_calls::RegistryAuthority)
            .ok()?;
    Some(association)
}

pub(crate) fn call_resolution(
    node: &crate::HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Option<serde_json::Value> {
    if let Some(link) = crate::result_maps::trait_leaf::resolution(node, facts) {
        return Some(link);
    }
    let association = call_association(node, facts)?;
    let resolution = facts.nominal_resolution().ok()?;
    let owner = syn::TypePath {
        qself: None,
        path: association.owner_path.clone(),
    };
    if resolution.resolve(&owner).ok()? != association.success
        || !resolution.checked_owned_copy(&association.success)
        || !association
            .error
            .nominal_parts()
            .is_some_and(|(nominal, _)| resolution.checked_no_drop(nominal))
        || call_owner(&resolution, &association.success, facts).is_err()
    {
        return None;
    }

    Some(association_resolution(&association))
}

fn call_owner(
    resolution: &Resolution,
    success: &TypeTree,
    facts: &HirFnFacts,
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    for module in &resolution.environment.modules {
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            if item.trait_.is_some()
                || !configuration.active(&item.attrs)?
                || !item.items.iter().any(|item| {
                    matches!(item, syn::ImplItem::Fn(method)
                    if method.sig.ident == "try_from")
                })
            {
                continue;
            }
            let syn::Type::Path(path) = item.self_ty.as_ref() else {
                return Err("TryFrom call has an unresolved competing inherent owner".into());
            };
            let scoped = Scoped {
                resolution,
                namespace: &module.namespace,
            };
            let key = scoped.name(path)?;
            if resolution.environment.index.declarations.contains_key(&key)
                && success
                    .nominal_parts()
                    .is_some_and(|(nominal, _)| key.join("::") != nominal.definition_path)
            {
                continue;
            }
            if scoped.resolve(path)? == *success {
                return Err("TryFrom call is shadowed by an original inherent method".into());
            }
        }
    }

    Ok(())
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        if crate::tryfrom_calls::input::captured(facts.facts()).is_ok_and(|input| input != "usize")
        {
            crate::tryfrom_calls::input::CAPABILITY
        } else {
            crate::tryfrom_calls::CAPABILITY
        },
        || {
            facts.build.frontend_options.contains_key(crate::tryfrom_calls::KEY)
            && facts.build.frontend_options.data::<InterfaceObservations>().is_ok_and(|interface| {
                matches!(&interface.result, Observation::Captured { ty: TypeTree::Result { ok, err } }
                    if crate::concrete_results::scalar_errors::GRAMMAR.permits(
                        &crate::data_types::TypeToken::from_tree(ok), &crate::data_types::TypeToken::from_tree(err)))
            })
            || crate::helper_calls::call_nodes(facts).iter().any(|node| call_resolution(node, facts).is_some())
        },
        "Concrete TryFrom requires its registered document policy",
    )
}
