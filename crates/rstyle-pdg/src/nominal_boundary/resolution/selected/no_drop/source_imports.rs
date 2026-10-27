//! Direct original import paths; aliases never supply authority by spelling.
use super::*;
use std::collections::BTreeMap;

pub(super) fn has_glob(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Path(path) => has_glob(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(has_glob),
        _ => false,
    }
}

pub(super) fn bindings(tree: &syn::UseTree) -> Vec<(String, Namespace)> {
    let mut result = Vec::new();
    collect(tree, &[], &mut result);
    result
}

fn collect(tree: &syn::UseTree, prefix: &[String], result: &mut Vec<(String, Namespace)>) {
    match tree {
        syn::UseTree::Path(path) => {
            let prefix: Namespace = prefix
                .iter()
                .cloned()
                .chain([path.ident.to_string()])
                .collect();
            collect(&path.tree, &prefix, result);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect(item, prefix, result);
            }
        }
        leaf => result.extend(imported_binding(leaf, prefix)),
    }
}

fn imported_binding(tree: &syn::UseTree, prefix: &[String]) -> Option<(String, Namespace)> {
    let (original, alias) = match tree {
        syn::UseTree::Name(name) => (&name.ident, &name.ident),
        syn::UseTree::Rename(rename) => (&rename.ident, &rename.rename),
        _ => return None,
    };
    let source: Namespace = if original == "self" {
        prefix.to_vec()
    } else {
        prefix
            .iter()
            .cloned()
            .chain([original.to_string()])
            .collect()
    };
    let name = if alias == "self" {
        prefix.last()?.clone()
    } else {
        alias.to_string()
    };

    Some((name, source))
}

pub(super) fn record(tree: &syn::UseTree, scope: &mut BTreeMap<String, bool>) {
    for (name, source) in bindings(tree) {
        clear(scope, &name);
        if source
            .first()
            .is_some_and(|root| root == "core" || root == "std")
        {
            scope.insert(format!("standard:{name}:{}", source.join("::")), true);
        }
    }
}

/// A direct declaration both shadows any canonical import and records its role.
pub(super) fn bind(scope: &mut BTreeMap<String, bool>, name: String, nominal: bool) {
    clear(scope, &name);
    scope.insert(name, nominal);
}

pub(super) fn clear(scope: &mut BTreeMap<String, bool>, name: &str) {
    let prefix = format!("standard:{name}:");
    scope.retain(|key, _| !key.starts_with(&prefix));
}

pub(super) fn resolve(path: &syn::Path, scopes: &[BTreeMap<String, bool>]) -> Option<Namespace> {
    if path.leading_colon.is_some()
        || path
            .segments
            .iter()
            .any(|part| !matches!(part.arguments, syn::PathArguments::None))
    {
        return None;
    }

    let names: Namespace = path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    let first = names.first()?;
    if first == "core" || first == "std" {
        return Some(names);
    }

    let scope = scopes
        .iter()
        .rev()
        .find(|scope| scope.contains_key(first))?;
    let prefix = format!("standard:{first}:");
    let matches: Vec<_> = scope
        .keys()
        .filter_map(|key| key.strip_prefix(&prefix))
        .collect();
    let [canonical] = matches.as_slice() else {
        return None;
    };

    Some(
        canonical
            .split("::")
            .map(str::to_owned)
            .chain(names[1..].iter().cloned())
            .collect(),
    )
}
