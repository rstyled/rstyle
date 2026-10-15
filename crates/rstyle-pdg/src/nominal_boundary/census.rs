//! Original unit buffers and independent ordinary-module closure.

use super::UnitContext;
#[cfg(test)]
use crate::concrete_types::digest;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub const KEY: &str = "rstyle.nominal_source_census";
pub const IDENTITY: &str = "rstyle.rust.nominal-source-census/1";
pub const MAX_FILES: usize = 256;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitFile {
    pub path: PathBuf,
    pub bytes: crate::SourceSnapshot,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Census {
    pub identity: String,
    pub context: UnitContext,
    pub files: Vec<UnitFile>,
}

pub struct Module {
    pub path: PathBuf,
    pub namespace: Vec<String>,
    pub items: Vec<syn::Item>,
}

pub fn verify(census: &Census) -> Result<Vec<Module>, String> {
    let files = checked_files(census)?;
    let mut closure = Closure {
        files,
        visited: BTreeSet::new(),
        modules: Vec::new(),
    };
    let root = &census.context.root_source;
    let directory = root.parent().ok_or("Missing root module directory")?;

    closure.file(root, directory, Vec::new())?;
    if closure.visited.len() != census.files.len() {
        return Err("Orphan source file prevents closed selected-unit proof".into());
    }
    Ok(closure.modules)
}

pub fn check_budget(census: &Census) -> Result<(), String> {
    let bytes = census.files.iter().try_fold(0usize, |total, file| {
        total
            .checked_add(file.bytes.len())
            .ok_or("Source census byte overflow")
    })?;
    if census.files.len() > MAX_FILES || bytes > MAX_BYTES {
        return Err("Source census exceeds file/byte budget".into());
    }
    Ok(())
}

fn checked_files(census: &Census) -> Result<BTreeMap<PathBuf, &str>, String> {
    census.context.transport()?;
    check_budget(census)?;
    if census.identity != IDENTITY || census.files.len() > MAX_FILES {
        return Err("Unregistered or over-budget source census".into());
    }
    let mut files = BTreeMap::new();
    let root = census
        .context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing package root")?;

    for file in &census.files {
        if !file.path.is_absolute()
            || !file.path.starts_with(root)
            || file
                .path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || file.digest != file.bytes.identity()
            || files
                .insert(file.path.clone(), file.bytes.as_ref())
                .is_some()
        {
            return Err("Invalid, duplicate, escaping or over-budget census file".into());
        }
    }
    Ok(files)
}

struct Site<'a> {
    path: &'a Path,
    directory: &'a Path,
    namespace: Vec<String>,
}

struct Closure<'a> {
    files: BTreeMap<PathBuf, &'a str>,
    visited: BTreeSet<PathBuf>,
    modules: Vec<Module>,
}

impl Closure<'_> {
    fn file(
        &mut self,
        path: &Path,
        directory: &Path,
        namespace: Vec<String>,
    ) -> Result<(), String> {
        if !self.visited.insert(path.into()) {
            return Err("Source file has ambiguous module ownership".into());
        }
        let source = self
            .files
            .get(path)
            .ok_or("Missing selected module source buffer")?;
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let file = syn::parse_file(source).map_err(|e| e.to_string())?;
        boundary_attributes(&file.attrs)?;

        self.items(
            Site {
                path,
                directory,
                namespace,
            },
            file.items,
        )
    }

    fn items(&mut self, site: Site<'_>, items: Vec<syn::Item>) -> Result<(), String> {
        for item in &items {
            verify_nested_items(item)?;
            if let syn::Item::Mod(module) = item {
                self.child(&site, module)?;
            }
        }
        self.modules.push(Module {
            path: site.path.into(),
            namespace: site.namespace,
            items,
        });
        Ok(())
    }

    fn child(&mut self, site: &Site<'_>, module: &syn::ItemMod) -> Result<(), String> {
        if !module.attrs.is_empty() {
            return Err("Module attributes require resolved presence/path evidence".into());
        }
        let mut child_namespace = site.namespace.clone();
        let name = module.ident.to_string();
        child_namespace.push(name.clone());
        let child_directory = site.directory.join(&name);
        if let Some((_, items)) = &module.content {
            return self.items(
                Site {
                    path: site.path,
                    directory: &child_directory,
                    namespace: child_namespace,
                },
                items.clone(),
            );
        }

        let ordinary = site.directory.join(format!("{name}.rs"));
        let mod_rs = child_directory.join("mod.rs");
        let candidates: Vec<_> = [ordinary, mod_rs]
            .into_iter()
            .filter(|p| self.files.contains_key(p))
            .collect();
        match candidates.as_slice() {
            [source] => self.file(source, &child_directory, child_namespace),
            _ => Err("Missing or ambiguous ordinary external module source".into()),
        }
    }
}

fn verify_nested_items(item: &syn::Item) -> Result<(), String> {
    let mut visitor = Boundary { failure: None };
    syn::visit::Visit::visit_item(&mut visitor, item);
    visitor.failure.map_or(Ok(()), Err)
}

struct Boundary {
    failure: Option<String>,
}
impl<'ast> syn::visit::Visit<'ast> for Boundary {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if self.failure.is_some() {
            return;
        }
        if let Err(error) = boundary_item(item) {
            self.failure = Some(error);
            return;
        }
        syn::visit::visit_item(self, item);
    }
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.failure = Some("Macro invocation could conceal unavailable unit items".into());
    }
}

fn boundary_attributes(attributes: &[syn::Attribute]) -> Result<(), String> {
    if attributes.iter().any(|attr| {
        !attr.path().is_ident("doc")
            && !attr.path().is_ident("allow")
            && !attr.path().is_ident("warn")
            && !attr.path().is_ident("deny")
            && !attr.path().is_ident("forbid")
    }) {
        return Err("Attributes require independent expansion/presence evidence".into());
    }
    Ok(())
}

fn boundary_item(item: &syn::Item) -> Result<(), String> {
    match item {
        syn::Item::Impl(item) if item.trait_.is_none() && item.unsafety.is_none() => {
            boundary_impl(item)
        }
        syn::Item::Fn(item) if item.sig.unsafety.is_none() => boundary_attributes(&item.attrs),
        syn::Item::Struct(item) => boundary_attributes(&item.attrs),
        syn::Item::Mod(item) if item.unsafety.is_none() => Ok(()),
        syn::Item::Use(item) => boundary_attributes(&item.attrs),
        // No item macros/derive, external blocks, trait impls or custom lang items.
        _ => Err("Unit item lies outside independent nominal drop-free boundary".into()),
    }
}

fn boundary_impl(item: &syn::ItemImpl) -> Result<(), String> {
    boundary_attributes(&item.attrs)?;
    crate::concrete_types::lifetimes::parameter_count(&item.generics)?;
    for member in &item.items {
        match member {
            syn::ImplItem::Fn(function) if function.sig.unsafety.is_none() => {
                boundary_attributes(&function.attrs)?
            }
            _ => return Err("Unsupported inherent impl item".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(files: &[(&str, &str)]) -> Census {
        Census {
            identity: IDENTITY.into(),
            context: crate::nominal_boundary::tests::context(),
            files: files
                .iter()
                .map(|(path, bytes)| UnitFile {
                    path: (*path).into(),
                    bytes: (*bytes).into(),
                    digest: digest(bytes.as_bytes()),
                })
                .collect(),
        }
    }

    #[test]
    fn ordinary_inline_and_external_modules_have_exact_unique_ownership() {
        let proof = sample(&[
            (
                "/sample/src/lib.rs",
                "\u{feff}mod data;\r\nmod api { pub fn get() -> i32 { 2 } }\r\n",
            ),
            ("/sample/src/data.rs", "pub struct Data { pub field: i32 }"),
        ]);
        let modules = verify(&proof).unwrap();
        assert_eq!(modules.len(), 3);
        assert!(modules.iter().any(|m| m.namespace == ["data"]));
        assert!(modules.iter().any(|m| m.namespace == ["api"]));
        for path in [
            "/sample/src/orphan.rs",
            "/sample/src/data/mod.rs",
            "/sample/src/lib.rs",
        ] {
            let mut changed = proof.clone();
            changed.files.push(UnitFile {
                path: path.into(),
                bytes: "".into(),
                digest: digest(b""),
            });
            assert!(verify(&changed).is_err());
        }
        let mut changed = proof;
        changed.files.pop();
        assert!(verify(&changed).is_err());
    }

    #[test]
    fn hidden_drop_expansions_presence_and_unsafe_item_forms_fail_closed() {
        for source in [
            "struct Data; impl Drop for Data { fn drop(&mut self) {} }",
            "#[derive(Clone)] struct Data;",
            "macro_rules! create { () => { struct Data; } } create!();",
            "#[cfg(feature = \"hidden\")] mod hidden;",
            "#[path = \"other.rs\"] mod hidden;",
            "unsafe fn hidden() {}",
            "fn hidden() { struct Data; impl Drop for Data { fn drop(&mut self) {} } }",
            "fn hidden() { create!(); }",
            "unsafe extern \"C\" { fn hidden(); }",
            "#[lang = \"drop\"] struct Data;",
        ] {
            assert!(
                verify(&sample(&[("/sample/src/lib.rs", source)])).is_err(),
                "{source}"
            );
        }
        verify(&sample(&[(
            "/sample/src/lib.rs",
            "struct Data { x: i32 } impl Data { fn x(&self) -> i32 { self.x } }",
        )]))
        .unwrap();
    }

    #[test]
    fn source_tampering_navigation_and_resource_failures_are_explicit() {
        let mut proof = sample(&[("/sample/src/lib.rs", "")]);
        proof.files[0].bytes = format!("{}x", proof.files[0].bytes).into();
        assert!(verify(&proof).is_err());
        proof = sample(&[("/sample/src/../outside.rs", "")]);
        assert!(verify(&proof).is_err());
        proof = sample(&[("/sample/src/lib.rs", "")]);
        proof.files[0].bytes = " ".repeat(MAX_BYTES + 1).into();
        proof.files[0].digest = digest(proof.files[0].bytes.as_bytes());
        assert!(verify(&proof).is_err());
        proof.files = (0..=MAX_FILES)
            .map(|index| UnitFile {
                path: format!("/sample/src/{index}.rs").into(),
                bytes: "".into(),
                digest: digest(b""),
            })
            .collect();
        assert!(verify(&proof).is_err());
    }
}
