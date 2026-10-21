//! Demand-driven proof of one direct scalar alias in the checked owning module.
use super::*;

pub(super) fn resolve(
    scope: &Scoped<'_>,
    path: &syn::TypePath,
) -> Result<Option<TypeTree>, String> {
    let Some(name) = path.path.get_ident() else {
        return Ok(None);
    };

    let environment = &scope.resolution.environment;
    let key: Vec<_> = scope
        .namespace
        .iter()
        .cloned()
        .chain([name.to_string()])
        .collect();
    if environment.index.declarations.contains_key(&key)
        || environment
            .index
            .imports
            .contains_key(&(scope.namespace.clone(), name.to_string()))
    {
        return Ok(None);
    }

    let mut candidates = environment
        .modules
        .iter()
        .filter(|module| &module.namespace == scope.namespace)
        .flat_map(|module| &module.items)
        .filter_map(|item| match item {
            syn::Item::Type(item) if item.ident == *name => Some(item),
            _ => None,
        });

    let Some(alias) = candidates.next() else {
        return Ok(None);
    };
    if candidates.next().is_some()
        || !alias.generics.params.is_empty()
        || alias.generics.where_clause.is_some()
        || environment
            .index
            .imports
            .contains_key(&(scope.namespace.clone(), name.to_string()))
    {
        return Err("Scalar alias requires one direct nongeneric same-module declaration".into());
    }

    Ok(Some(target(scope, alias)?))
}

fn target(scope: &Scoped<'_>, alias: &syn::ItemType) -> Result<TypeTree, String> {
    let environment = &scope.resolution.environment;
    let syn::Type::Path(target) = alias.ty.as_ref() else {
        return Err("Scalar alias target must be a builtin scalar".into());
    };
    let scalar = target
        .path
        .get_ident()
        .filter(|_| target.qself.is_none())
        .map(ToString::to_string)
        .filter(|name| concrete_types::scalar(name) && name != "unit")
        .ok_or("Scalar alias target must be a bare builtin scalar")?;

    let key: Vec<_> = scope
        .namespace
        .iter()
        .cloned()
        .chain([scalar.clone()])
        .collect();
    if environment.index.declarations.contains_key(&key)
        || environment
            .modules
            .iter()
            .filter(|module| &module.namespace == scope.namespace)
            .flat_map(|module| &module.items)
            .any(|item| match item {
                syn::Item::Union(item) => item.ident == scalar,
                syn::Item::Mod(item) => item.ident == scalar,
                syn::Item::Trait(item) => item.ident == scalar,
                syn::Item::TraitAlias(item) => item.ident == scalar,
                _ => false,
            })
        || environment
            .index
            .imports
            .contains_key(&(scope.namespace.clone(), scalar.clone()))
    {
        return Err("Scalar alias target is shadowed by a declaration or import".into());
    }
    // The checked namespace rejects primitive-shadowing aliases and unresolved
    // macro/import effects. No transitive or compiler-only alias proof is used.
    Ok(TypeTree::Scalar(scalar))
}

impl Resolution {
    pub(crate) fn requires_scalar_aliases(&self, source: &str) -> bool {
        struct Probe<'a> {
            resolution: &'a Resolution,
            found: bool,
        }
        impl<'ast> syn::visit::Visit<'ast> for Probe<'_> {
            fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                self.found |= resolve(
                    &Scoped {
                        resolution: self.resolution,
                        namespace: &self.resolution.namespace,
                    },
                    path,
                )
                .is_ok_and(|ty| ty.is_some());
                syn::visit::visit_type_path(self, path);
            }
        }

        let Ok(function) = syn::parse_str::<syn::ItemFn>(source) else {
            return false;
        };
        let mut probe = Probe {
            resolution: self,
            found: false,
        };

        syn::visit::Visit::visit_item_fn(&mut probe, &function);
        probe.found
    }
}
