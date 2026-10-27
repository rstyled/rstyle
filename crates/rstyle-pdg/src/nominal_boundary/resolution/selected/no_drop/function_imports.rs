//! Direct function imports need absence of a paired macro export in that scope.
use std::collections::BTreeSet;

pub(super) fn names(
    items: &[&syn::Item],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<BTreeSet<String>, String> {
    let mut functions = BTreeSet::new();
    let mut counterparts = BTreeSet::new();

    for item in items {
        if !configuration.active(crate::conditional_source::item_attributes(item))? {
            continue;
        }
        match item {
            syn::Item::Fn(item) => {
                functions.insert(item.sig.ident.to_string());
            }
            syn::Item::Use(item) => {
                if !imports(&item.tree, None, &mut counterparts) {
                    return Ok(BTreeSet::new());
                }
            }
            syn::Item::Macro(item) if item.mac.path.is_ident("macro_rules") => {
                let Some(name) = &item.ident else {
                    return Ok(BTreeSet::new());
                };
                counterparts.insert(name.to_string());
            }
            // An invocation can generate a same-name re-export or definition.
            // This bounded route does not infer namespace absence from it.
            syn::Item::Macro(_) | syn::Item::Verbatim(_) => return Ok(BTreeSet::new()),
            syn::Item::Mod(item) if macro_use(&item.attrs, configuration)? => {
                return Ok(BTreeSet::new());
            }
            syn::Item::ExternCrate(item) if macro_use(&item.attrs, configuration)? => {
                return Ok(BTreeSet::new());
            }
            _ => {}
        }
    }
    functions.retain(|name| !counterparts.contains(name));
    Ok(functions)
}

fn macro_use(
    attributes: &[syn::Attribute],
    configuration: &crate::conditional_source::Configuration<'_>,
) -> Result<bool, String> {
    Ok(configuration
        .effective_attributes(attributes)?
        .iter()
        .any(|attribute| attribute.path().is_ident("macro_use")))
}

fn imports(tree: &syn::UseTree, parent: Option<String>, names: &mut BTreeSet<String>) -> bool {
    match tree {
        syn::UseTree::Path(path) => imports(&path.tree, Some(path.ident.to_string()), names),
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .all(|item| imports(item, parent.clone(), names)),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {
            names.insert(imported_name(tree, parent));
            true
        }
        syn::UseTree::Glob(_) => false,
    }
}

fn imported_name(tree: &syn::UseTree, parent: Option<String>) -> String {
    match tree {
        syn::UseTree::Name(name) if name.ident == "self" => parent.unwrap_or_else(|| "self".into()),
        syn::UseTree::Name(name) => name.ident.to_string(),
        syn::UseTree::Rename(rename) => rename.rename.to_string(),
        _ => unreachable!(),
    }
}
