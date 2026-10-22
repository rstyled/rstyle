//! Closed original module exports, without importing unresolved reexports.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Modules {
    closed: BTreeMap<Namespace, bool>,
    aliases: BTreeMap<(Namespace, String), Namespace>,
    imported: BTreeSet<(Namespace, String)>,
}

impl Modules {
    pub fn collect(
        &mut self,
        namespace: &Namespace,
        items: &[&syn::Item],
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        let closed =
            super::super::imports::glob_items_policy(items.iter().copied(), configuration, true)
                .is_ok()
                && items.iter().all(|item| !protected_export(item));
        self.closed
            .entry(namespace.clone())
            .and_modify(|prior| *prior &= closed)
            .or_insert(closed);
        for item in items {
            if !configuration.active(crate::conditional_source::item_attributes(item))? {
                continue;
            }
            if let syn::Item::Use(item) = item {
                for (name, source) in source_imports::bindings(&item.tree) {
                    self.imported.insert((namespace.clone(), name.clone()));
                    if item.leading_colon.is_none()
                        && let Some(target) = local(namespace, &source)
                    {
                        self.aliases.insert((namespace.clone(), name), target);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn contains(&self, namespace: &Namespace, prefix: &[String], unindexed: bool) -> bool {
        !unindexed && self.target(namespace, prefix).is_some()
    }
    fn target(&self, namespace: &Namespace, prefix: &[String]) -> Option<Namespace> {
        let target = local(namespace, prefix)?;
        if self.closed.get(&target) == Some(&true) {
            let (name, owner) = target.split_last()?;
            return (!self.imported.contains(&(owner.to_vec(), name.clone()))).then_some(target);
        }
        for depth in 0..target.len() {
            let owner = target[..depth].to_vec();
            if let Some(alias) = self.aliases.get(&(owner, target[depth].clone())) {
                let resolved: Namespace =
                    alias.iter().chain(&target[depth + 1..]).cloned().collect();
                return (self.closed.get(&resolved) == Some(&true)).then_some(resolved);
            }
        }
        None
    }
}

pub(in crate::nominal_boundary::resolution::selected) fn lexical_target(
    environment: &Environment,
    namespace: &Namespace,
    prefix: &[String],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<Namespace, String> {
    original_glob(environment, namespace, prefix, configuration)?;
    let mut modules = Modules::default();
    for module in &environment.modules {
        modules.collect(
            &module.namespace,
            &module.items.iter().collect::<Vec<_>>(),
            configuration,
        )?;
    }
    modules.target(namespace, prefix).ok_or_else(|| {
        format!(
            "Original module glob lacks a closed lexical target: {} in {}",
            prefix.join("::"),
            namespace.join("::")
        )
    })
}

fn original_glob(
    environment: &Environment,
    namespace: &Namespace,
    prefix: &[String],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<(), String> {
    let mut matching = 0;
    for module in environment
        .modules
        .iter()
        .filter(|module| &module.namespace == namespace)
    {
        for item in &module.items {
            let syn::Item::Use(item) = item else {
                continue;
            };
            if item.leading_colon.is_some() || !configuration.active(&item.attrs)? {
                continue;
            }
            matching += glob_paths(&item.tree, Vec::new())
                .iter()
                .filter(|path| path.as_slice() == prefix)
                .count();
        }
    }
    if matching != 1 {
        return Err("Lexical glob proof lacks its exact unique active original import".into());
    }
    Ok(())
}

fn glob_paths(tree: &syn::UseTree, mut prefix: Namespace) -> Vec<Namespace> {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            glob_paths(&path.tree, prefix)
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|item| glob_paths(item, prefix.clone()))
            .collect(),
        syn::UseTree::Glob(_) => vec![prefix],
        _ => Vec::new(),
    }
}

fn local(namespace: &Namespace, path: &[String]) -> Option<Namespace> {
    let first = path.first()?;
    if ["core", "std", "alloc", "<external>"].contains(&first.as_str()) {
        return None;
    }
    if ["crate", "self", "super"].contains(&first.as_str()) {
        anchored(path.to_vec(), namespace, false).ok()
    } else {
        Some(namespace.iter().chain(path).cloned().collect())
    }
}

fn protected_export(item: &syn::Item) -> bool {
    let (name, visible) = match item {
        syn::Item::Macro(item) => (item.ident.as_ref().map(ToString::to_string), true),
        syn::Item::Fn(item) => (
            Some(item.sig.ident.to_string()),
            !matches!(item.vis, syn::Visibility::Inherited),
        ),
        syn::Item::Const(item) => (
            Some(item.ident.to_string()),
            !matches!(item.vis, syn::Visibility::Inherited),
        ),
        syn::Item::Static(item) => (
            Some(item.ident.to_string()),
            !matches!(item.vis, syn::Visibility::Inherited),
        ),
        _ => (super::super::builtin_options::type_name(item), true),
    };
    visible && name.is_some_and(|name| closure::protected(&name) || name == "Drop")
}
