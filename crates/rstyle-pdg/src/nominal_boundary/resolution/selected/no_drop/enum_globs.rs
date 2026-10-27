//! Source-owned enum wildcards import variants, never macros or trait aliases.
use super::*;
use crate::source_evidence::qualified_name;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Scope {
    pub namespace: Namespace,
    pub unindexed: bool,
    pub closed_imports: bool,
    modules: module_globs::Modules,
    declarations: BTreeSet<Namespace>,
    functions: BTreeSet<Namespace>,
    traits: BTreeSet<Namespace>,
}

impl Scope {
    /// A direct declaration in the current original module, without lexical lookup.
    pub fn direct_module_enum(&self, name: &str) -> bool {
        !self.unindexed && self.direct_module_enum_in(&self.namespace, name)
    }

    /// Query only a direct declaration in a checked original module namespace.
    pub fn direct_module_enum_in(&self, namespace: &[String], name: &str) -> bool {
        self.declarations.contains(&qualified_name(namespace, name))
    }

    /// Original enum wildcards can introduce variants, never a destructor trait.
    pub fn only_original_enum_globs(&self, tree: &syn::UseTree) -> bool {
        self.original_enum_globs(tree, &[])
    }

    fn original_enum_globs(&self, tree: &syn::UseTree, prefix: &[String]) -> bool {
        match tree {
            syn::UseTree::Path(path) => {
                let prefix = prefix
                    .iter()
                    .cloned()
                    .chain([path.ident.to_string()])
                    .collect::<Vec<_>>();
                self.original_enum_globs(&path.tree, &prefix)
            }
            syn::UseTree::Group(group) => group
                .items
                .iter()
                .all(|tree| self.original_enum_globs(tree, prefix)),
            syn::UseTree::Glob(_) => self.contains_local(prefix, &self.declarations),
            _ => true,
        }
    }

    pub fn new(
        modules: &[census::Module],
        cfgs: &[String],
        closed_imports: bool,
    ) -> Result<Self, String> {
        let configuration = crate::conditional_source::Configuration::new(cfgs);
        let mut scope = Self {
            closed_imports,
            ..Self::default()
        };
        for module in modules {
            scope.collect(
                &module.items.iter().collect::<Vec<_>>(),
                &module.namespace,
                &configuration,
            )?;
        }
        Ok(scope)
    }
    fn collect(
        &mut self,
        items: &[&syn::Item],
        namespace: &Namespace,
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        if self.closed_imports {
            self.modules.collect(namespace, items, configuration)?;
        }
        for name in function_imports::names(items, configuration)? {
            self.functions
                .insert(namespace.iter().cloned().chain([name]).collect());
        }
        for item in items {
            if !configuration.active(crate::conditional_source::item_attributes(item))? {
                continue;
            }
            self.collect_item(item, namespace, configuration)?;
        }
        Ok(())
    }
    fn collect_item(
        &mut self,
        item: &syn::Item,
        namespace: &Namespace,
        configuration: &crate::conditional_source::Configuration<'_>,
    ) -> Result<(), String> {
        match item {
            syn::Item::Trait(item) => {
                self.traits.insert(
                    namespace
                        .iter()
                        .cloned()
                        .chain([item.ident.to_string()])
                        .collect(),
                );
            }
            syn::Item::Enum(item) => {
                self.declarations.insert(
                    namespace
                        .iter()
                        .cloned()
                        .chain([item.ident.to_string()])
                        .collect(),
                );
            }
            syn::Item::Mod(item) => {
                if let Some((_, items)) = &item.content {
                    let child = namespace
                        .iter()
                        .cloned()
                        .chain([item.ident.to_string()])
                        .collect();
                    self.collect(&items.iter().collect::<Vec<_>>(), &child, configuration)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn inline(
        items: &[&syn::Item],
        cfgs: &[String],
        closed_imports: bool,
    ) -> Result<Self, String> {
        let mut scope = Self {
            closed_imports,
            ..Self::default()
        };
        scope.collect(
            items,
            &vec![],
            &crate::conditional_source::Configuration::new(cfgs),
        )?;
        Ok(scope)
    }
    fn local_path(&self, prefix: &[String]) -> Option<Namespace> {
        let (root, tail) = prefix.split_first()?;
        if self.unindexed && root != "crate" {
            return None;
        }
        let mut namespace = self.path_root(root)?;

        let mut parents = root == "super";
        for segment in tail {
            if segment == "super" && parents {
                namespace.pop()?;
            } else {
                parents = false;
                if ["self", "super", "crate"].contains(&segment.as_str()) {
                    return None;
                }
                namespace.push(segment.clone());
            }
        }
        Some(namespace)
    }
    fn path_root(&self, root: &str) -> Option<Namespace> {
        match root {
            "self" => Some(self.namespace.clone()),
            "crate" => Some(vec![]),
            "super" => {
                let mut parent = self.namespace.clone();
                parent.pop()?;
                Some(parent)
            }
            _ => None,
        }
    }
    pub fn known_trait(&self, path: &syn::Path) -> bool {
        crate::source_evidence::plain_relative_path(path)
            .is_some_and(|names| self.contains_local(&names, &self.traits))
    }
    pub fn direct_relative_trait(&self, names: &[String]) -> bool {
        !self.unindexed
            && names.len() >= 2
            && self.traits.contains(
                &self
                    .namespace
                    .iter()
                    .chain(names)
                    .cloned()
                    .collect::<Namespace>(),
            )
    }
    fn contains_local(&self, path: &[String], declarations: &BTreeSet<Namespace>) -> bool {
        self.local_path(path)
            .is_some_and(|path| declarations.contains(&path))
    }
    pub fn macro_free_function(&self, prefix: &[String], name: &str) -> bool {
        let path = qualified_name(prefix, name);
        self.contains_local(&path, &self.functions)
    }
    pub fn bindings(
        &self,
        tree: &syn::UseTree,
        prefix: &[String],
        scope: &mut BTreeMap<String, bool>,
    ) {
        match tree {
            syn::UseTree::Path(path) => {
                let child: Vec<_> = prefix
                    .iter()
                    .cloned()
                    .chain([path.ident.to_string()])
                    .collect();
                self.bindings(&path.tree, &child, scope);
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.bindings(item, prefix, scope);
                }
            }
            _ => self.binding_leaf(tree, prefix, scope),
        }
    }
    fn binding_leaf(
        &self,
        tree: &syn::UseTree,
        prefix: &[String],
        scope: &mut BTreeMap<String, bool>,
    ) {
        match tree {
            syn::UseTree::Name(name) => {
                let name = if name.ident == "self" {
                    prefix.last().cloned().unwrap_or_else(|| "self".into())
                } else {
                    name.ident.to_string()
                };
                let source = if name == "self" {
                    prefix.to_vec()
                } else {
                    prefix.iter().cloned().chain([name.clone()]).collect()
                };
                self.named_binding(name, &source, scope);
            }
            syn::UseTree::Rename(rename) => {
                let source = prefix
                    .iter()
                    .cloned()
                    .chain([rename.ident.to_string()])
                    .collect();
                self.named_binding(rename.rename.to_string(), &source, scope);
            }
            syn::UseTree::Glob(_)
                if !self.contains_local(prefix, &self.declarations)
                    && !(self.closed_imports
                        && self
                            .modules
                            .contains(&self.namespace, prefix, self.unindexed)) =>
            {
                scope.insert("*".into(), false);
            }
            _ => {}
        }
    }
    fn named_binding(&self, name: String, source: &Namespace, scope: &mut BTreeMap<String, bool>) {
        scope.insert(
            format!("trait:{name}"),
            self.contains_local(source, &self.traits),
        );
        scope.insert(name, false);
    }
}

#[cfg(test)]
mod tests;
