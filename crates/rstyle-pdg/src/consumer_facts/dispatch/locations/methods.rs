//! Same-module nominal identity for original methods, independent of dispatch proofs.
use crate::conditional_source::Configuration;
use std::collections::BTreeMap;

mod lifetimes;

type Declaration = (String, syn::ItemFn, &'static str);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Owner,
    Trait,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Nominal {
    kind: Kind,
    arity: usize,
    generic: bool,
}

#[derive(Default)]
struct Names {
    entries: BTreeMap<String, Option<Nominal>>,
    glob: bool,
}

impl Names {
    fn insert(&mut self, name: String, kind: Option<Nominal>) {
        self.entries
            .entry(name)
            .and_modify(|value| *value = None)
            .or_insert(kind);
    }

    fn nominal(&self, name: &str, kind: Kind) -> Option<Nominal> {
        if self.glob {
            return None;
        }
        self.entries
            .get(name)
            .copied()
            .flatten()
            .filter(|item| item.kind == kind)
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
    source: &str,
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
                declarations.extend(implementation(item, &names, &configuration, source)?);
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

fn eligible(generics: &syn::Generics, kind: Kind) -> Option<Nominal> {
    let generic = generics
        .params
        .iter()
        .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)));
    if generic {
        return generic_parameters(generics).then_some(Nominal {
            kind,
            arity: generics.params.len(),
            generic,
        });
    }

    let parameters = lifetimes::parameters(generics)?;
    if kind == Kind::Trait && !parameters.is_empty() {
        return None;
    }
    Some(Nominal {
        kind,
        arity: parameters.len(),
        generic,
    })
}

fn implementation(
    item: syn::ItemImpl,
    names: &Names,
    configuration: &Configuration<'_>,
    source: &str,
) -> Result<Vec<Declaration>, String> {
    let Some((owner, origin)) = owner(&item, names, source) else {
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

fn owner(item: &syn::ItemImpl, names: &Names, source: &str) -> Option<(String, Option<String>)> {
    if item.unsafety.is_some() || item.defaultness.is_some() {
        return None;
    }
    let path = lifetimes::owner_path(&item.self_ty)?;
    let nominal = names.nominal(&path.ident.to_string(), Kind::Owner)?;
    let owner = nominal_name(path, nominal, &item.generics, source)?;

    let origin = match &item.trait_ {
        Some((None, path, _)) if path.leading_colon.is_none() && path.segments.len() == 1 => {
            let path = path.segments.first()?;
            let nominal = names.nominal(&path.ident.to_string(), Kind::Trait)?;
            Some(if nominal.generic {
                nominal_name(path, nominal, &item.generics, source)?
            } else if matches!(path.arguments, syn::PathArguments::None) {
                path.ident.to_string()
            } else {
                return None;
            })
        }
        Some(_) => return None,
        None => None,
    };
    Some((owner, origin))
}

fn nominal_name(
    path: &syn::PathSegment,
    nominal: Nominal,
    generics: &syn::Generics,
    source: &str,
) -> Option<String> {
    if !nominal.generic
        && nominal.arity == 0
        && matches!(path.arguments, syn::PathArguments::None)
        && generic_parameters(generics)
    {
        return Some(path.ident.to_string());
    }
    if !nominal.generic {
        let parameters = lifetimes::parameters(generics)?;
        lifetimes::arguments(&path.arguments, nominal.arity, &parameters)?;
        return Some(path.ident.to_string());
    }

    if !generic_parameters(generics) {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &path.arguments else {
        return None;
    };
    if arguments.colon2_token.is_some() || arguments.args.len() != nominal.arity {
        return None;
    }
    use syn::spanned::Spanned;
    let range = super::shapes::span(source, path.span());
    let spelling = crate::source_evidence::byte_range(source, range).ok()?;
    super::super::review::canonical(spelling).ok()
}

fn generic_parameters(generics: &syn::Generics) -> bool {
    let mut names = std::collections::BTreeSet::new();
    generics.params.iter().all(|parameter| {
        let (name, attributes) = match parameter {
            syn::GenericParam::Lifetime(parameter) => {
                (parameter.lifetime.ident.to_string(), &parameter.attrs)
            }
            syn::GenericParam::Type(parameter) => (parameter.ident.to_string(), &parameter.attrs),
            syn::GenericParam::Const(parameter) => (parameter.ident.to_string(), &parameter.attrs),
        };
        attributes.is_empty() && names.insert(name)
    })
}

#[cfg(test)]
mod tests;
