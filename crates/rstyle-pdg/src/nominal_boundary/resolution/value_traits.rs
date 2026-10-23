//! Original primitive by-value trait ownership; no callable effects authority.
use super::*;
use crate::source_evidence;

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &HirFnFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        crate::local_value_trait_calls::CAPABILITY,
        || candidate(facts),
        "Primitive trait receivers require their registered document policy",
    )
}

pub(crate) fn receiver(facts: &HirFnFacts, observation: &Observation) -> Result<bool, String> {
    if !candidate(facts) {
        return Ok(false);
    }
    if !supports(facts, crate::local_value_trait_calls::CAPABILITY)? {
        return Err("Primitive trait receiver requires its registered source policy".into());
    }
    let environment = standalone_environment(facts)?;
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing primitive trait source file")?;
    let mut matched = 0;

    for module in environment
        .modules
        .iter()
        .filter(|module| module.path == std::path::Path::new(file))
    {
        let bytes = environment.source(&module.path)?.bytes();
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            let Some(member) = crate::local_value_trait_calls::source_member(facts, item, bytes)?
            else {
                continue;
            };
            verify_member(facts, &environment, module, &member)?;
            matched += 1;
        }
    }
    if matched != 1 {
        return Err("Primitive receiver lacks one exact original trait implementation".into());
    }
    if !matches!(observation, Observation::Captured { ty: TypeTree::Scalar(name) } if name == "usize")
    {
        return Err("Primitive trait receiver contradicts captured interface".into());
    }
    Ok(true)
}

fn verify_member(
    facts: &HirFnFacts,
    environment: &Environment,
    module: &census::Module,
    member: &crate::local_value_trait_calls::SourceMember,
) -> Result<(), String> {
    selected::primitive_trait_namespace(facts, environment, &module.namespace)?;
    let names = source_evidence::plain_relative_path(&member.trait_path)
        .ok_or("Primitive trait requires an original lexical path")?;

    let key = lexical(environment, &module.namespace, names)?;
    let expected = source_evidence::plain_relative_path(&member.trait_identity)
        .ok_or("Primitive selected trait identity is not an original path")?;
    if key != expected || !member.self_type.path.is_ident("usize") {
        return Err("Primitive trait source binding differs from selected implementation".into());
    }

    let (name, namespace) = key.split_last().ok_or("Empty primitive trait identity")?;
    let declarations: Vec<_> = environment
        .modules
        .iter()
        .filter(|candidate| candidate.namespace == namespace)
        .flat_map(|candidate| &candidate.items)
        .filter_map(|item| match item {
            syn::Item::Trait(item) if item.ident == name => Some(item),
            _ => None,
        })
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err("Primitive trait lacks one original declaration".into());
    };
    crate::local_value_trait_calls::verify_trait(member, declaration)
}

pub(crate) fn candidate(facts: &HirFnFacts) -> bool {
    facts.signature.has_self
        && facts
            .signature
            .params
            .first()
            .is_some_and(|parameter| parameter.ty.normalized.as_deref() == Some("usize"))
}

fn lexical(
    environment: &Environment,
    namespace: &Namespace,
    names: Namespace,
) -> Result<Namespace, String> {
    let first = names.first().ok_or("Empty primitive trait path")?;
    if let Some(target) = environment
        .index
        .imports
        .get(&(namespace.clone(), first.clone()))
    {
        return Ok(target.iter().chain(names[1..].iter()).cloned().collect());
    }
    anchored(names, namespace, false)
}
