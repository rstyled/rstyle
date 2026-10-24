//! Discharge only original block imports of exact same-module enum variants.
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
            && self.enums.direct_module_enum(&name)
            && !scope.contains_key(&name)
            && !self.names.aliases.contains(&name)
            && !self
                .scopes
                .iter()
                .skip(1)
                .any(|scope| scope.contains_key(&name) || scope.contains_key("*"))
    }
}

#[cfg(test)]
mod tests;
