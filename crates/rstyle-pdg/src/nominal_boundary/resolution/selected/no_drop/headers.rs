//! Alias closure is conservative: ambiguous spellings enlarge rejection sets.
use super::*;
use std::collections::BTreeSet;

pub(super) struct Resolution {
    pub local_nominal: bool,
    pub standard_roots: bool,
    pub local_trait: bool,
    pub resolved_trait: Option<Namespace>,
}

#[derive(Default)]
pub(super) struct Names {
    pub drops: BTreeSet<String>,
    pub targets: BTreeSet<String>,
    pub aliases: BTreeSet<String>,
    imports: Vec<(String, String)>,
    pub from_shadowed: bool,
}

impl Names {
    pub fn new(target: &Namespace) -> Self {
        Self {
            drops: BTreeSet::from(["Drop".into()]),
            targets: target.last().cloned().into_iter().collect(),
            ..Self::default()
        }
    }
    pub fn close(&mut self) {
        loop {
            let mut changed = false;
            for (source, alias) in &self.imports {
                if self.drops.contains(source) {
                    changed |= self.drops.insert(alias.clone());
                }
                if self.targets.contains(source) {
                    changed |= self.targets.insert(alias.clone());
                }
            }
            if !changed {
                break;
            }
        }
    }
    pub fn import(&mut self, tree: &syn::UseTree) -> Result<(), String> {
        match tree {
            syn::UseTree::Path(path) => self.import(&path.tree),
            syn::UseTree::Group(group) => {
                for child in &group.items {
                    self.import(child)?;
                }
                Ok(())
            }
            syn::UseTree::Rename(rename) => {
                self.imports
                    .push((rename.ident.to_string(), rename.rename.to_string()));
                Ok(())
            }
            syn::UseTree::Name(_) => Ok(()),
            syn::UseTree::Glob(_) => Ok(()),
        }
    }
    pub fn verify(&self, item: &syn::ItemImpl, resolution: Resolution) -> Result<(), String> {
        let Some((_, path, _)) = &item.trait_ else {
            return Ok(());
        };
        if resolution.local_trait {
            return Ok(());
        }

        let self_ty = transparent(item.self_ty.as_ref());
        let syn::Type::Path(ty) = self_ty else {
            return match self_ty {
                syn::Type::Array(_)
                | syn::Type::Slice(_)
                | syn::Type::Tuple(_)
                | syn::Type::Reference(_)
                | syn::Type::Ptr(_)
                | syn::Type::BareFn(_)
                | syn::Type::Never(_) => Ok(()),
                _ => Err("Cleanup self type is not independently resolved".into()),
            };
        };
        if ty.qself.is_some() {
            return Err("Associated cleanup self type is unsupported".into());
        }
        let name = ty
            .path
            .segments
            .last()
            .ok_or("Missing cleanup self nominal")?
            .ident
            .to_string();
        if item
            .generics
            .type_params()
            .any(|parameter| parameter.ident == name)
        {
            return Err("Generic cleanup self type is unsupported".into());
        }
        let possible_target = self.targets.contains(&name) || self.aliases.contains(&name);
        if !possible_target && ty.path.segments.len() == 1 && resolution.local_nominal {
            return Ok(());
        }
        // Unknown trait names can be re-exported Drop aliases. Only independently
        // rooted standard traits are sufficient to exclude that possibility here.
        if resolution.standard_roots
            && (known_trait(path) || resolution.resolved_trait.as_deref().is_some_and(known_path))
        {
            return Ok(());
        }
        Err(possible_destructor(path, &name))
    }
}

fn transparent(mut ty: &syn::Type) -> &syn::Type {
    loop {
        ty = match ty {
            syn::Type::Paren(inner) => &inner.elem,
            syn::Type::Group(inner) => &inner.elem,
            _ => return ty,
        };
    }
}

fn known_trait(path: &syn::Path) -> bool {
    let segments: Vec<_> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    known_path(&segments)
        && segments != ["core", "error", "Error"]
        && !matches!(
            segments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            ["core" | "std", "convert", "From"]
        )
}

fn known_path(segments: &[String]) -> bool {
    matches!(
        segments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice(),
        ["core" | "std", "fmt", "Display"]
            | ["core" | "std", "ops", "Index" | "IndexMut"]
            | ["core" | "std", "error", "Error"]
            | ["core" | "std", "convert", "From"]
    )
}

fn possible_destructor(path: &syn::Path, name: &str) -> String {
    format!(
        "Selected or unresolved owner has a possible destructor trait implementation: {} for {name}",
        path.segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    )
}
