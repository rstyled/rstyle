//! Bind the actual original inherent owner before using its Copy source closure.
use super::*;
use crate::nominal_boundary::resolution::{self, PathResolution, Scoped};

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn needed(
    facts: &HirFnFacts,
    resolution: &Resolution,
) -> Result<bool, String> {
    let environment = &resolution.environment;
    if environment.index.unresolved_expansions.is_empty()
        || !facts
            .build
            .frontend_options
            .contains_key(crate::inherent_calls::KEY)
    {
        return Ok(false);
    }
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing Copy owner source file")?;
    Ok(environment.modules.iter().any(|module| {
        module.path == std::path::Path::new(file)
            && environment
                .index
                .unresolved_expansions
                .iter()
                .any(|candidate| module.namespace.starts_with(candidate))
    }))
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn needed_namespace(
    resolution: &Resolution,
    namespace: &[String],
) -> bool {
    resolution
        .environment
        .index
        .unresolved_expansions
        .iter()
        .any(|candidate| namespace.starts_with(candidate))
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn observation(
    facts: &HirFnFacts,
) -> Result<Option<crate::inherent_calls::Definition>, String> {
    let Some(value) = facts.build.frontend_options.get(crate::inherent_calls::KEY) else {
        return Ok(None);
    };
    let observed: crate::inherent_calls::Definition =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if observed.identity != crate::inherent_calls::IDENTITY
        || observed.definition_identity.is_empty()
        || observed.implementation_identity.is_empty()
        || facts
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            != Some(observed.definition_identity.as_str())
        || observed.function_span != facts.fn_span
        || facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            != Some(observed.source_file.as_str())
    {
        return Err("Copy source owner lacks its exact original inherent observation".into());
    }
    Ok(Some(observed))
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) struct Resolved {
    pub root: TypeTree,
    pub namespace: Namespace,
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn resolve(
    facts: &HirFnFacts,
    resolution: &Resolution,
    observed: &crate::inherent_calls::Definition,
) -> Result<Option<Resolved>, String> {
    let environment = &resolution.environment;
    let mut candidates = Vec::new();

    for (index, module) in environment
        .modules
        .iter()
        .enumerate()
        .filter(|(_, module)| module.path == std::path::Path::new(&observed.source_file))
    {
        let bytes = environment.source(&module.path)?.bytes();
        for (namespace, receiver) in resolution::module_functions(index, bytes, facts, environment)?
        {
            if let Some(receiver) = receiver {
                let syn::Type::Path(path) = receiver.as_ref() else {
                    return Err("Copy source owner requires an original nominal type path".into());
                };
                let scoped = Scoped {
                    resolution,
                    namespace: &namespace,
                };
                let key = scoped.name(path)?;
                if key.join("::") != observed.owner_nominal.definition_path {
                    return Err("Copy source owner path contradicts its captured nominal".into());
                }
                if !resolution.trees.contains_key(&key) {
                    return Ok(None);
                }
                let root = scoped.resolve(path)?;
                if !exact_impl(facts, module, bytes, observed)? {
                    return Ok(None);
                }
                if root.nominal_parts().map(|(nominal, _)| nominal) != Some(&observed.owner_nominal)
                {
                    return Err(
                        "Copy source owner differs from its independently resolved nominal".into(),
                    );
                }
                candidates.push(Resolved { root, namespace });
            }
        }
    }
    unique(candidates)
}

fn unique(candidates: Vec<Resolved>) -> Result<Option<Resolved>, String> {
    let [root] = candidates.as_slice() else {
        return Err("Copy source requires one exact ordinary inherent function owner".into());
    };
    Ok(Some(Resolved {
        root: root.root.clone(),
        namespace: root.namespace.clone(),
    }))
}

fn exact_impl(
    facts: &HirFnFacts,
    module: &census::Module,
    bytes: &str,
    observed: &crate::inherent_calls::Definition,
) -> Result<bool, String> {
    let implementations: Vec<_> = module
        .items
        .iter()
        .filter_map(|item| {
            let syn::Item::Impl(item) = item else {
                return None;
            };
            (item.trait_.is_none()
                && item.unsafety.is_none()
                && item.defaultness.is_none()
                && resolution::implementation_source_span(bytes, item)
                    == observed.implementation_span
                && resolution::supported_inherent_member(item, bytes, facts))
            .then_some(item)
        })
        .collect();
    let [implementation] = implementations.as_slice() else {
        return Err("Copy source receipt lacks its exact original inherent impl".into());
    };
    Ok(implementation.generics.params.is_empty() && implementation.generics.where_clause.is_none())
}

pub(in crate::nominal_boundary::resolution::selected::no_drop) fn interface(
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
    root: &TypeTree,
) -> Result<bool, String> {
    interface.validate()?;
    if interface.parameters.len() != facts.signature.params.len()
        || facts.signature.has_self && interface.parameters.is_empty()
    {
        return Err("Copy owner interface parameter inventory disagrees".into());
    }
    for (index, observation) in interface.parameters.iter().enumerate() {
        let Observation::Captured { ty } = observation else {
            return Err("Copy owner source requires every complete parameter".into());
        };
        if index == 0 && facts.signature.has_self {
            if !matches!(ty, TypeTree::Shared(owner) if owner.as_ref() == root)
                || !shared_receiver(facts)?
            {
                return Ok(false);
            }
        } else if !scalar(ty) {
            return Ok(false);
        }
    }
    Ok(matches!(&interface.result, Observation::Captured { ty } if scalar(ty) || ty == root))
}

fn shared_receiver(facts: &HirFnFacts) -> Result<bool, String> {
    let method: syn::ImplItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    Ok(
        matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(receiver))
        if receiver.reference.is_some() && receiver.mutability.is_none() && receiver.colon_token.is_none()),
    )
}

fn scalar(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Scalar(token) if concrete_types::scalar(token) && token != "unit")
}
