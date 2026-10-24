use super::*;
use crate::nominal_boundary::resolution::{FunctionSource, Scoped};

pub(super) fn verify(
    facts: &HirFnFacts,
    resolution: &Resolution,
    tree: &TypeTree,
) -> Result<Namespace, String> {
    resolution.environment.check_input_owner(facts)?;
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing constructor source file")?;
    let context = OwnerCheck {
        facts,
        resolution,
        tree,
    };
    let mut owners = Vec::new();

    for module in resolution
        .environment
        .modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        owners.extend(context.module(module)?);
    }

    match owners.as_slice() {
        [owner] => Ok(owner.clone()),
        _ => Err("Copy constructor lacks one exact original inherent owner".into()),
    }
}

struct OwnerCheck<'a> {
    facts: &'a HirFnFacts,
    resolution: &'a Resolution,
    tree: &'a TypeTree,
}

impl OwnerCheck<'_> {
    fn module(
        &self,
        module: &crate::nominal_boundary::census::Module,
    ) -> Result<Vec<Namespace>, String> {
        let bytes = self.resolution.environment.source(&module.path)?.bytes();
        let mut owners = Vec::new();

        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            if !item.generics.params.is_empty()
                || item.generics.where_clause.is_some()
                || item.trait_.is_some()
                || item.unsafety.is_some()
            {
                continue;
            }
            for member in &item.items {
                let syn::ImplItem::Fn(method) = member else {
                    continue;
                };
                if !(FunctionSource {
                    vis: &method.vis,
                    sig: &method.sig,
                    body: &method.block,
                })
                .exact(bytes, self.facts)
                {
                    continue;
                }
                self.implementation(item, method, &module.namespace)?;
                owners.push(module.namespace.clone());
            }
        }

        Ok(owners)
    }

    fn implementation(
        &self,
        item: &syn::ItemImpl,
        method: &syn::ImplItemFn,
        namespace: &Namespace,
    ) -> Result<(), String> {
        super::super::super::function_attributes(self.facts, &item.attrs)?;
        super::super::super::function_attributes(self.facts, &method.attrs)?;
        let syn::Type::Path(path) = item.self_ty.as_ref() else {
            return Err("Copy constructor requires a direct original impl owner".into());
        };
        let owner = Scoped {
            resolution: self.resolution,
            namespace,
        }
        .name(path)?;

        if self.resolution.trees.get(&owner) != Some(self.tree) {
            return Err("Copy constructor impl owner differs from checked result".into());
        }

        ancestors(self.facts, self.resolution, namespace)
    }
}

fn ancestors(
    facts: &HirFnFacts,
    resolution: &Resolution,
    namespace: &[String],
) -> Result<(), String> {
    for module in &resolution.environment.modules {
        if !namespace.starts_with(&module.namespace) {
            continue;
        }
        let Some(next) = namespace.get(module.namespace.len()) else {
            continue;
        };
        for item in &module.items {
            if let syn::Item::Mod(item) = item
                && item.ident == next
            {
                super::super::super::function_attributes(facts, &item.attrs)?;
            }
        }
    }
    Ok(())
}
