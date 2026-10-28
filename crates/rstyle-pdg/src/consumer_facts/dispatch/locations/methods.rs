//! Same-module nominal identity for original methods, independent of dispatch proofs.
use crate::conditional_source::Configuration;
use std::collections::BTreeMap;

type Declaration = (String, syn::ItemFn, &'static str);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Owner,
    Trait,
}

#[derive(Default)]
struct Names {
    entries: BTreeMap<String, Option<Kind>>,
    glob: bool,
}

impl Names {
    fn insert(&mut self, name: String, kind: Option<Kind>) {
        self.entries
            .entry(name)
            .and_modify(|value| *value = None)
            .or_insert(kind);
    }

    fn contains(&self, name: &str, kind: Kind) -> bool {
        !self.glob && self.entries.get(name) == Some(&Some(kind))
    }

    fn import(&mut self, tree: &syn::UseTree, parent: Option<&syn::Ident>) {
        for binding in import_bindings(tree, parent) {
            match binding {
                Some(name) => self.insert(name.to_string(), None),
                None => self.glob = true,
            }
        }
    }
}

/// Borrow explicit bindings; a wildcard has no finite binding roster.
fn import_bindings<'a>(
    tree: &'a syn::UseTree,
    parent: Option<&'a syn::Ident>,
) -> Vec<Option<&'a syn::Ident>> {
    match tree {
        syn::UseTree::Path(path) => import_bindings(&path.tree, Some(&path.ident)),
        syn::UseTree::Name(name) => vec![Some(if name.ident == "self" {
            parent.unwrap_or(&name.ident)
        } else {
            &name.ident
        })],
        syn::UseTree::Rename(rename) => vec![Some(&rename.rename)],
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|tree| import_bindings(tree, parent))
            .collect(),
        syn::UseTree::Glob(_) => vec![None],
    }
}

pub(super) fn declarations(
    items: Vec<syn::Item>,
    cfgs: &[String],
) -> Result<Vec<Declaration>, String> {
    let names = nominal_names(&items);
    let configuration = Configuration::new(cfgs);
    let mut declarations = Vec::new();
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                declarations.push((function.sig.ident.to_string(), function, "free-function"))
            }
            syn::Item::Impl(item) => {
                declarations.extend(implementation(item, &names, &configuration)?);
            }
            _ => {}
        }
    }

    Ok(declarations)
}

fn nominal_names(items: &[syn::Item]) -> Names {
    let mut names = Names::default();
    for item in items {
        let (name, kind) = match item {
            syn::Item::Struct(item) => (&item.ident, eligible(&item.generics, Kind::Owner)),
            syn::Item::Enum(item) => (&item.ident, eligible(&item.generics, Kind::Owner)),
            syn::Item::Trait(item) => (&item.ident, eligible(&item.generics, Kind::Trait)),
            syn::Item::Type(item) => (&item.ident, None),
            syn::Item::Union(item) => (&item.ident, None),
            syn::Item::Mod(item) => (&item.ident, None),
            syn::Item::ExternCrate(item) => (
                item.rename.as_ref().map_or(&item.ident, |(_, name)| name),
                None,
            ),
            syn::Item::Use(item) => {
                names.import(&item.tree, None);
                continue;
            }
            _ => continue,
        };
        names.insert(name.to_string(), kind);
    }
    names
}

fn eligible(generics: &syn::Generics, kind: Kind) -> Option<Kind> {
    (generics.params.is_empty() && generics.where_clause.is_none()).then_some(kind)
}

fn implementation(
    item: syn::ItemImpl,
    names: &Names,
    configuration: &Configuration<'_>,
) -> Result<Vec<Declaration>, String> {
    let Some((owner, origin)) = owner(&item, names) else {
        return Ok(Vec::new());
    };
    let mut declarations = Vec::new();
    for member in item.items {
        let syn::ImplItem::Fn(function) = member else {
            continue;
        };
        if function.defaultness.is_some() || !configuration.active(&function.attrs)? {
            continue;
        }
        let name = super::super::review::method_name(
            &owner,
            &function.sig.ident.to_string(),
            origin.as_deref(),
        );
        let function = syn::ItemFn {
            attrs: function.attrs,
            vis: function.vis,
            sig: function.sig,
            block: Box::new(function.block),
        };
        declarations.push((name, function, "method"));
    }
    Ok(declarations)
}

fn owner(item: &syn::ItemImpl, names: &Names) -> Option<(String, Option<String>)> {
    if eligible(&item.generics, Kind::Owner).is_none()
        || item.unsafety.is_some()
        || item.defaultness.is_some()
    {
        return None;
    }

    let owner = super::super::shapes::plain_type(&item.self_ty)?;
    if !names.contains(&owner, Kind::Owner) {
        return None;
    }

    let origin = match &item.trait_ {
        Some((None, path, _)) => {
            let origin = super::super::shapes::plain_path(path)?;
            Some(names.contains(&origin, Kind::Trait).then_some(origin)?)
        }
        Some(_) => return None,
        None => None,
    };
    Some((owner, origin))
}

#[cfg(test)]
mod tests;
