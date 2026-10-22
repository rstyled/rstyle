//! Original unit buffers and independent ordinary-module closure.

use super::UnitContext;
mod cfg_if;
pub mod documentation;
pub mod expansions;
pub mod external_imports;
mod macro_scope;
mod module_proof;
pub(crate) mod policy;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansions: Vec<expansions::ModuleExpansion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documentation: Vec<documentation::Input>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_imports: Vec<external_imports::Import>,
}

pub struct Module {
    pub path: PathBuf,
    pub namespace: Vec<String>,
    pub items: Vec<syn::Item>,
    pub verified_macro_imports: BTreeSet<String>,
    pub verified_external_imports: BTreeSet<String>,
}

pub fn verify(census: &Census) -> Result<Vec<Module>, String> {
    verify_with_constants(census, false)
}

pub(crate) fn verify_with_constants(
    census: &Census,
    constants: bool,
) -> Result<Vec<Module>, String> {
    verify_mode(census, constants, None, InputMode::default())
}

pub(crate) fn verify_selected(census: &Census, cfgs: &[String]) -> Result<Vec<Module>, String> {
    verify_mode(census, true, Some(cfgs), InputMode::default())
}

pub(crate) fn verify_expanded(census: &Census, cfgs: &[String]) -> Result<Vec<Module>, String> {
    verify_mode(
        census,
        true,
        Some(cfgs),
        InputMode {
            expansions: true,
            documentation: false,
            external_imports: false,
        },
    )
}

pub(crate) fn verify_documented(
    census: &Census,
    cfgs: &[String],
    expansions: bool,
) -> Result<Vec<Module>, String> {
    verify_mode(
        census,
        true,
        Some(cfgs),
        InputMode {
            expansions,
            documentation: true,
            external_imports: false,
        },
    )
}

pub(crate) fn verify_imported(
    census: &Census,
    cfgs: &[String],
    expansions: bool,
) -> Result<Vec<Module>, String> {
    verify_mode(
        census,
        true,
        Some(cfgs),
        InputMode {
            expansions,
            documentation: !census.documentation.is_empty(),
            external_imports: true,
        },
    )
}

#[derive(Default)]
struct InputMode {
    expansions: bool,
    documentation: bool,
    external_imports: bool,
}

impl InputMode {
    fn inputs(
        &self,
        census: &Census,
        cfgs: Option<&[String]>,
    ) -> Result<BTreeSet<PathBuf>, String> {
        let inputs = if self.documentation {
            documentation::verify(census, cfgs.ok_or("Missing documentation configuration")?)?
        } else {
            if !census.documentation.is_empty() {
                return Err("Documentation inputs require independent source ownership".into());
            }
            BTreeSet::new()
        };
        if !self.external_imports && !census.external_imports.is_empty() {
            return Err("External macro imports require independent imported closure".into());
        }
        Ok(inputs)
    }
}

fn verify_mode(
    census: &Census,
    constants: bool,
    cfgs: Option<&[String]>,
    mode: InputMode,
) -> Result<Vec<Module>, String> {
    let InputMode {
        expansions,
        external_imports,
        ..
    } = mode;
    check_budget(census)?;
    let inputs = mode.inputs(census, cfgs)?;
    let files = checked_files(census, &inputs)?;
    let mut closure = Closure {
        files,
        visited: inputs,
        modules: Vec::new(),
        constants,
        selected: cfgs.map(crate::conditional_source::Configuration::new),
        expansions: expansions.then_some(census.expansions.as_slice()),
        used_expansions: BTreeSet::new(),
        external_imports: external_imports.then_some(census.external_imports.as_slice()),
        used_external_imports: BTreeSet::new(),
    };
    let root = &census.context.root_source;
    let directory = root.parent().ok_or("Missing root module directory")?;

    closure.file(
        Site {
            path: root,
            directory,
            namespace: Vec::new(),
        },
        macro_scope::Scope::default(),
    )?;
    if closure.visited.len() != census.files.len() {
        return Err("Orphan source file prevents closed selected-unit proof".into());
    }
    if expansions && closure.used_expansions.len() != census.expansions.len() {
        return Err("Module expansion inventory has an unproved or duplicate output".into());
    }
    if external_imports && closure.used_external_imports.len() != census.external_imports.len() {
        return Err("External macro import inventory has unproved or duplicate records".into());
    }
    Ok(closure.modules)
}

pub fn check_budget(census: &Census) -> Result<(), String> {
    let bytes = census.files.iter().try_fold(0usize, |total, file| {
        total
            .checked_add(file.bytes.len())
            .ok_or("Source census byte overflow")
    })?;
    if census.external_imports.len() > MAX_FILES
        || census
            .external_imports
            .iter()
            .any(|import| import.exports.macros.len() > MAX_FILES)
        || census.documentation.len() > MAX_FILES
        || census.expansions.len() > MAX_FILES
        || census
            .expansions
            .iter()
            .any(|record| record.chain.len() > 128)
    {
        return Err("Module expansion inventory exceeds source census budget".into());
    }
    if census.files.len() > MAX_FILES || bytes > MAX_BYTES {
        return Err("Source census exceeds file/byte budget".into());
    }
    Ok(())
}

fn checked_files<'a>(
    census: &'a Census,
    documentation: &BTreeSet<PathBuf>,
) -> Result<BTreeMap<PathBuf, &'a crate::SourceSnapshot>, String> {
    census.context.transport()?;
    check_budget(census)?;
    if census.identity != IDENTITY || census.files.len() > MAX_FILES {
        return Err("Unregistered or over-budget source census".into());
    }
    let mut files = BTreeMap::new();
    let mut locations = BTreeSet::new();
    let root = census
        .context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing package root")?;

    for file in &census.files {
        if !file.path.is_absolute()
            || !file.path.starts_with(root)
            || (!documentation.contains(&file.path)
                && file
                    .path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir)))
            || !locations.insert(documentation::in_package(root, &file.path)?)
            || file.digest != file.bytes.identity()
            || files.insert(file.path.clone(), &file.bytes).is_some()
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
    files: BTreeMap<PathBuf, &'a crate::SourceSnapshot>,
    visited: BTreeSet<PathBuf>,
    modules: Vec<Module>,
    constants: bool,
    selected: Option<crate::conditional_source::Configuration<'a>>,
    expansions: Option<&'a [expansions::ModuleExpansion]>,
    used_expansions: BTreeSet<usize>,
    external_imports: Option<&'a [external_imports::Import]>,
    used_external_imports: BTreeSet<usize>,
}

struct ModuleItems {
    scope: macro_scope::Scope,
    items: Vec<syn::Item>,
    macro_imports: BTreeSet<String>,
    external_imports: BTreeSet<String>,
}

impl ModuleItems {
    fn record(&mut self, item: syn::Item, imported: bool) {
        if imported && let syn::Item::Mod(module) = &item {
            self.macro_imports.insert(module.ident.to_string());
        }
        if imported && let syn::Item::ExternCrate(import) = &item {
            self.external_imports.insert(import.ident.to_string());
        }
        self.items.push(item);
    }
}

impl Closure<'_> {
    fn file(
        &mut self,
        site: Site<'_>,
        scope: macro_scope::Scope,
    ) -> Result<macro_scope::Scope, String> {
        if !self.visited.insert(site.path.into()) {
            return Err("Source file has ambiguous module ownership".into());
        }
        let source = self
            .files
            .get(site.path)
            .ok_or("Missing selected module source buffer")?;
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let file = syn::parse_file(source).map_err(|e| e.to_string())?;
        if let Some(configuration) = &self.selected {
            selected_attributes(configuration, &file.attrs, true)?;
        } else {
            boundary_attributes(&file.attrs)?;
        }

        self.items(site, file.items, scope)
    }

    fn items(
        &mut self,
        site: Site<'_>,
        items: Vec<syn::Item>,
        scope: macro_scope::Scope,
    ) -> Result<macro_scope::Scope, String> {
        let mut state = ModuleItems {
            scope,
            items: Vec::new(),
            macro_imports: BTreeSet::new(),
            external_imports: BTreeSet::new(),
        };

        for item in items {
            if let Some(configuration) = &self.selected
                && !configuration.active(crate::conditional_source::item_attributes(&item))?
            {
                continue;
            }
            let expanded = self.expand(&site, &item, &state.scope)?;
            for item in expanded.unwrap_or_else(|| vec![item]) {
                if let Some(configuration) = &self.selected
                    && !configuration.active(crate::conditional_source::item_attributes(&item))?
                {
                    continue;
                }
                let imported = self.item(&site, &item, &mut state.scope)?;
                state.record(item, imported);
            }
        }
        self.modules.push(Module {
            path: site.path.into(),
            namespace: site.namespace,
            items: state.items,
            verified_macro_imports: state.macro_imports,
            verified_external_imports: state.external_imports,
        });
        Ok(state.scope)
    }

    fn item(
        &mut self,
        site: &Site<'_>,
        item: &syn::Item,
        scope: &mut macro_scope::Scope,
    ) -> Result<bool, String> {
        if self.selected.is_none() {
            verify_nested_items(item, self.constants)?;
        }
        if let Some(names) = self.external_import(site, item)? {
            scope.import_external(names);
            return Ok(true);
        }
        if self.expansions.is_some() {
            self.observe(site, item, scope)?;
        }

        let syn::Item::Mod(module) = item else {
            return Ok(false);
        };
        let child = self.child(site, module, scope.child())?;
        if self.expansions.is_some() && self.macro_use(module)? {
            return scope.import_child(&child);
        }
        Ok(false)
    }

    fn child(
        &mut self,
        site: &Site<'_>,
        module: &syn::ItemMod,
        scope: macro_scope::Scope,
    ) -> Result<macro_scope::Scope, String> {
        if module.unsafety.is_some() {
            return Err("Unsafe module ownership is unsupported".into());
        }
        if let Some(configuration) = &self.selected {
            selected_attributes(configuration, &module.attrs, false)?;
        } else if !module.attrs.is_empty() {
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
                scope,
            );
        }

        let ordinary = site.directory.join(format!("{name}.rs"));
        let mod_rs = child_directory.join("mod.rs");
        let candidates: Vec<_> = [ordinary, mod_rs]
            .into_iter()
            .filter(|p| self.files.contains_key(p))
            .collect();
        match candidates.as_slice() {
            [source] => self.file(
                Site {
                    path: source,
                    directory: &child_directory,
                    namespace: child_namespace,
                },
                scope,
            ),
            _ => Err("Missing or ambiguous ordinary external module source".into()),
        }
    }
}

fn verify_nested_items(item: &syn::Item, constants: bool) -> Result<(), String> {
    let mut visitor = Boundary {
        failure: None,
        constants,
    };
    syn::visit::Visit::visit_item(&mut visitor, item);
    visitor.failure.map_or(Ok(()), Err)
}

struct Boundary {
    failure: Option<String>,
    constants: bool,
}
impl<'ast> syn::visit::Visit<'ast> for Boundary {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if self.failure.is_some() {
            return;
        }
        if let Err(error) = boundary_item(item, self.constants) {
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

fn boundary_item(item: &syn::Item, constants: bool) -> Result<(), String> {
    match item {
        syn::Item::Impl(item) if item.trait_.is_none() && item.unsafety.is_none() => {
            boundary_impl(item, constants)
        }
        syn::Item::Fn(item) if item.sig.unsafety.is_none() => boundary_attributes(&item.attrs),
        syn::Item::Struct(item) => boundary_attributes(&item.attrs),
        syn::Item::Mod(item) if item.unsafety.is_none() => Ok(()),
        syn::Item::Use(item) => boundary_attributes(&item.attrs),
        syn::Item::Const(item) if constants => boundary_attributes(&item.attrs),
        // No item macros/derive, external blocks, trait impls or custom lang items.
        _ => Err("Unit item lies outside independent nominal drop-free boundary".into()),
    }
}

fn boundary_impl(item: &syn::ItemImpl, constants: bool) -> Result<(), String> {
    boundary_attributes(&item.attrs)?;
    crate::concrete_types::lifetimes::parameter_count(&item.generics)?;
    for member in &item.items {
        match member {
            syn::ImplItem::Fn(function) if function.sig.unsafety.is_none() => {
                boundary_attributes(&function.attrs)?
            }
            syn::ImplItem::Const(item) if constants => boundary_attributes(&item.attrs)?,
            _ => return Err("Unsupported inherent impl item".into()),
        }
    }
    Ok(())
}

fn selected_attributes(
    configuration: &crate::conditional_source::Configuration<'_>,
    attributes: &[syn::Attribute],
    root: bool,
) -> Result<(), String> {
    if !configuration.active(attributes)? {
        return Err("Inactive module cannot own selected source".into());
    }
    for attribute in configuration.effective_attributes(attributes)? {
        let path = attribute.path();
        if !["cfg", "doc", "allow", "warn", "deny", "forbid"]
            .iter()
            .any(|name| path.is_ident(name))
            && !(root && path.is_ident("no_std"))
            && !(!root && matches!(&attribute, syn::Meta::Path(path) if path.is_ident("macro_use")))
        {
            return Err("Selected module attribute lacks supported presence/path semantics".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(files: &[(&str, &str)]) -> Census {
        Census {
            expansions: Vec::new(),
            documentation: Vec::new(),
            external_imports: Vec::new(),
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
    fn selected_cfg_closure_owns_exact_active_files_and_keeps_other_items() {
        let proof = sample(&[
            (
                "/sample/src/lib.rs",
                "#![no_std]\n#[cfg(feature = \"api\")] mod api; #[cfg(not(feature = \"api\"))] mod other;",
            ),
            (
                "/sample/src/api.rs",
                "pub struct Data; enum Other { A } impl Drop for Data { fn drop(&mut self) {} }",
            ),
        ]);
        let cfgs = vec!["feature=\"api\"".to_owned()];
        let modules = verify_selected(&proof, &cfgs).unwrap();
        assert_eq!(modules.len(), 2);
        let api = modules
            .iter()
            .find(|module| module.namespace == ["api"])
            .unwrap();
        // Closure is ownership evidence, not a drop/effect certificate.
        assert_eq!(api.items.len(), 3);
        assert!(verify(&proof).is_err());
        assert!(verify_selected(&proof, &[]).is_err());
        let mut orphan = proof;
        orphan.files.push(UnitFile {
            path: "/sample/src/other.rs".into(),
            bytes: "".into(),
            digest: digest(b""),
        });
        assert!(verify_selected(&orphan, &cfgs).is_err());
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
    fn constant_context_preserves_recursive_item_and_expansion_checks() {
        for source in [
            "const LIMIT: i32 = 4; struct Data { x: i32 }",
            "struct Data; impl Data { const LIMIT: i32 = 4; }",
            "const PAIR: (i32, bool) = (4, true); struct Data;",
        ] {
            let proof = sample(&[("/sample/src/lib.rs", source)]);
            assert!(verify(&proof).is_err(), "{source}");
            assert!(verify_with_constants(&proof, true).is_ok(), "{source}");
        }
        for source in [
            "#[cfg(any())] const LIMIT: i32 = 4;",
            "const LIMIT: i32 = make!();",
            "const _: () = { struct Hidden; impl Drop for Hidden { fn drop(&mut self) {} } };",
            "struct Data; impl Data { #[cfg(any())] const LIMIT: i32 = 4; }",
            "struct Data; impl Data { const LIMIT: i32 = make!(); }",
            "const _: () = { unsafe fn hidden() {} };",
        ] {
            assert!(
                verify_with_constants(&sample(&[("/sample/src/lib.rs", source)]), true).is_err(),
                "{source}"
            );
        }
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
