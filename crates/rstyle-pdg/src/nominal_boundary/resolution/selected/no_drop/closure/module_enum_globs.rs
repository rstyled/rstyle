//! Discharge original block enum globs with exact declaration ownership.
use super::*;

impl Closure<'_> {
    pub(super) fn module_enum_globs(&self, block: &syn::Block, scope: &mut BTreeMap<String, bool>) {
        if !self.external_macros
            || self.generated_context != 0
            || self.enums.unindexed
            || !scope.contains_key("*")
        {
            return;
        }
        let mut wildcard = false;
        for statement in &block.stmts {
            let syn::Stmt::Item(syn::Item::Use(item)) = statement else {
                continue;
            };
            if self.configuration.active(&item.attrs) != Ok(true) {
                continue;
            }
            if !source_imports::has_glob(&item.tree) {
                continue;
            }
            wildcard = true;
            if item.leading_colon.is_some() || !self.module_enum_import(&item.tree, scope) {
                return;
            }
        }
        if wildcard {
            scope.remove("*");
        }
    }

    fn module_enum_import(&self, tree: &syn::UseTree, scope: &BTreeMap<String, bool>) -> bool {
        let syn::UseTree::Path(path) = tree else {
            return false;
        };
        let name = path.ident.to_string();
        matches!(path.tree.as_ref(), syn::UseTree::Glob(_))
            && (self.enums.direct_module_enum(&name)
                || self.digit_products && self.imported_module_enum(&name))
            && !scope.contains_key(&name)
            && !self.names.aliases.contains(&name)
            && !self
                .scopes
                .iter()
                .skip(1)
                .any(|scope| scope.contains_key(&name) || scope.contains_key("*"))
    }

    fn imported_module_enum(&self, name: &str) -> bool {
        let Some(environment) = self.dependency_sources else {
            return false;
        };
        if !environment
            .index
            .imports
            .contains_key(&(self.enums.namespace.clone(), name.into()))
        {
            return false;
        }
        let path = crate::source_evidence::qualified_name(&self.enums.namespace, name);
        let Ok(target) =
            crate::nominal_boundary::resolution::import_chains::resolve(path, &environment.index)
        else {
            return false;
        };
        let Some(declaration) = environment.index.declarations.get(&target) else {
            return false;
        };
        let Some(module) = environment.modules.get(declaration.module_index) else {
            return false;
        };

        let Some((name, namespace)) = target.split_last() else {
            return false;
        };
        module.namespace == namespace
            && self.enums.direct_module_enum_in(namespace, name)
            && matches!(module.items.get(declaration.item_index), Some(syn::Item::Enum(item)) if item.ident == name)
    }
}

#[cfg(test)]
mod imported_tests;
#[cfg(test)]
mod tests;
