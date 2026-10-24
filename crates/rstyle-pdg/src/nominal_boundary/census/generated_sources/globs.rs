//! Exact crate-rooted original module globs with a macro-free export surface.
use super::*;

pub(super) fn closed(
    import: &syn::ItemUse,
    modules: &BTreeMap<Vec<String>, &Module>,
    configuration: &Configuration<'_>,
) -> Result<bool, String> {
    if import.leading_colon.is_some() {
        return Ok(false);
    }
    let Some(path) = crate_glob(&import.tree) else {
        return Ok(false);
    };
    if !original_path(&path, modules) {
        return Ok(false);
    }

    let module = modules
        .get(&path)
        .ok_or("Missing original glob export module")?;
    let mut surface = macro_scope::Scope::default();
    for item in &module.items {
        if !configuration.active(crate::conditional_source::item_attributes(item))? {
            continue;
        }
        // Reexports may carry macros even when their spelling looks like a type.
        if matches!(item, syn::Item::Use(import) if !matches!(import.vis, syn::Visibility::Inherited))
            || matches!(item, syn::Item::Macro(_))
        {
            return Ok(false);
        }
        if let syn::Item::Mod(module) = item
            && configuration
                .effective_attributes(&module.attrs)?
                .iter()
                .any(|attribute| attribute.path().is_ident("macro_use"))
        {
            return Ok(false);
        }
        surface.observe(item, configuration, None)?;
    }

    Ok(surface
        .lookup("__macro_free_original_export_surface")
        .is_ok())
}

fn crate_glob(tree: &syn::UseTree) -> Option<Vec<String>> {
    let syn::UseTree::Path(root) = tree else {
        return None;
    };
    if root.ident != "crate" {
        return None;
    }

    let mut path = Vec::new();
    let mut tail = root.tree.as_ref();
    while let syn::UseTree::Path(segment) = tail {
        path.push(segment.ident.to_string());
        tail = segment.tree.as_ref();
    }

    (matches!(tail, syn::UseTree::Glob(_)) && !path.is_empty()).then_some(path)
}

fn original_path(path: &[String], modules: &BTreeMap<Vec<String>, &Module>) -> bool {
    for depth in 0..path.len() {
        let Some(parent) = modules.get(&path[..depth]) else {
            return false;
        };
        let name = &path[depth];
        if !parent
            .items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == name.as_str()))
            || parent.items.iter().any(
                |item| matches!(item, syn::Item::Use(import) if imports_name(&import.tree, name)),
            )
        {
            return false;
        }
    }
    modules.contains_key(path)
}

fn imports_name(tree: &syn::UseTree, name: &str) -> bool {
    match tree {
        syn::UseTree::Path(path) => imports_name(&path.tree, name),
        syn::UseTree::Group(group) => group.items.iter().any(|tree| imports_name(tree, name)),
        syn::UseTree::Name(import) => import.ident == name,
        syn::UseTree::Rename(import) => import.rename == name,
        syn::UseTree::Glob(_) => false,
    }
}

/// Normalize only a proven macro-free glob for the existing attribute checker.
pub(super) fn observed(import: &syn::ItemUse) -> syn::Item {
    let mut import = import.clone();
    import.tree = syn::UseTree::Name(syn::UseName {
        ident: syn::Ident::new("__checked_macro_free_glob", proc_macro2::Span::call_site()),
    });
    syn::Item::Use(import)
}
