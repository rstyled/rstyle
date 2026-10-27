//! Original textual macro scope; unknown expansion never proves an empty scope.
use super::expansions::FileSpan;
use crate::conditional_source::Configuration;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DefinitionBinding {
    /// Exact original-source/namespace identity, not an invented rustc DefId.
    pub identity: String,
    pub site: FileSpan,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Scope {
    bindings: BTreeMap<String, DefinitionBinding>,
    introduced: BTreeMap<String, DefinitionBinding>,
    poisoned: bool,
    external: BTreeSet<String>,
}

impl Scope {
    /// Snapshot at the child's declaration. Later parent definitions cannot
    /// retroactively modify inherited bindings; only new child bindings export.
    pub fn child(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            introduced: BTreeMap::new(),
            poisoned: self.poisoned,
            external: self.external.clone(),
        }
    }

    pub fn lookup(&self, name: &str) -> Result<Option<&DefinitionBinding>, String> {
        if self.poisoned {
            return Err("Unknown expansion prevents exact textual macro lookup".into());
        }
        if !self.bindings.contains_key(name) && self.external.contains(name) {
            return Err("External macro has no local expansion ownership permission".into());
        }
        Ok(self.bindings.get(name))
    }

    /// Retain every proven imported name; imports supply no local macro body.
    pub fn import_external(&mut self, names: Vec<String>) {
        self.external.extend(names);
    }

    /// The caller supplies an original, checked full-item binding only for a
    /// macro_rules definition. Known cfg-if invocations must be expanded first.
    pub fn observe(
        &mut self,
        item: &syn::Item,
        configuration: &Configuration<'_>,
        binding: Option<DefinitionBinding>,
    ) -> Result<(), String> {
        let attributes = crate::conditional_source::item_attributes(item);
        if !configuration.active(attributes)? {
            return Ok(());
        }

        let effective = configuration.effective_attributes(attributes)?;
        let macro_use = effective
            .iter()
            .any(|attribute| attribute.path().is_ident("macro_use"));
        self.poisoned |= effective
            .iter()
            .any(|attribute| !known_attribute(attribute))
            || uncertain_item(item, macro_use);

        match item {
            syn::Item::Macro(item)
                if item.mac.path.is_ident("macro_rules") && item.ident.is_some() =>
            {
                self.define(
                    item,
                    binding.ok_or("Missing exact original macro definition binding")?,
                )?;
            }
            syn::Item::Macro(_) | syn::Item::Verbatim(_) | syn::Item::Use(_) => {}
            syn::Item::ExternCrate(_) if macro_use => {}
            _ if binding.is_some() => {
                return Err(
                    "Definition binding does not belong to an original macro_rules item".into(),
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply only to a checked #[macro_use] module, never an ordinary child.
    pub fn import_child(&mut self, child: &Scope) -> Result<bool, String> {
        if child.poisoned {
            self.poisoned = true;
            return Ok(false);
        }
        for (name, binding) in &child.introduced {
            self.bindings.insert(name.clone(), binding.clone());
            self.introduced.insert(name.clone(), binding.clone());
        }
        Ok(true)
    }

    fn define(&mut self, item: &syn::ItemMacro, binding: DefinitionBinding) -> Result<(), String> {
        let name = item
            .ident
            .as_ref()
            .ok_or("Missing macro_rules name")?
            .to_string();

        if protected(&name) {
            self.poisoned = true;
            return Ok(());
        }
        if binding.identity.is_empty()
            || binding.site.digest.is_empty()
            || !binding.site.path.is_absolute()
            || binding.site.span.start >= binding.site.span.end
        {
            return Err(
                "Macro definition shadows protected names or lacks exact original identity".into(),
            );
        }

        self.bindings.insert(name.clone(), binding.clone());
        self.introduced.insert(name, binding);
        Ok(())
    }
}

fn uncertain_item(item: &syn::Item, macro_use: bool) -> bool {
    match item {
        syn::Item::Macro(item) => !item.mac.path.is_ident("macro_rules") || item.ident.is_none(),
        syn::Item::Verbatim(_) => true,
        syn::Item::Use(item) => uncertain_import(&item.tree),
        syn::Item::ExternCrate(_) => macro_use,
        _ => false,
    }
}

fn uncertain_import(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => uncertain_import(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(uncertain_import),
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Name(name) => protected(&name.ident.to_string()),
        syn::UseTree::Rename(name) => protected(&name.rename.to_string()),
    }
}

fn protected(name: &str) -> bool {
    crate::concrete_types::scalar(name)
        || builtin_derive(name)
        || [
            "char",
            "f32",
            "f64",
            "str",
            "core",
            "std",
            "assert",
            "assert_eq",
            "assert_ne",
            "debug_assert",
            "debug_assert_eq",
            "debug_assert_ne",
            "cfg",
            "include",
            "include_str",
            "include_bytes",
            "panic",
            "unreachable",
            "unimplemented",
            "todo",
            "matches",
            "format_args",
            "format",
            "vec",
            "write",
            "writeln",
            "print",
            "println",
            "eprint",
            "eprintln",
            "concat",
            "stringify",
            "env",
            "option_env",
            "line",
            "file",
            "column",
            "module_path",
        ]
        .contains(&name)
}

fn builtin_derive(name: &str) -> bool {
    [
        "Clone",
        "Copy",
        "Debug",
        "Eq",
        "PartialEq",
        "Hash",
        "Ord",
        "PartialOrd",
        "Default",
    ]
    .contains(&name)
}

fn known_attribute(attribute: &syn::Meta) -> bool {
    if [
        "cfg",
        "doc",
        "allow",
        "warn",
        "deny",
        "forbid",
        "inline",
        "cold",
        "must_use",
        "repr",
        "non_exhaustive",
    ]
    .iter()
    .any(|name| attribute.path().is_ident(name))
    {
        return true;
    }
    if matches!(attribute, syn::Meta::Path(path) if path.is_ident("macro_use")) {
        return true;
    }
    if let syn::Meta::List(list) = attribute
        && list.path.is_ident("derive")
    {
        return list
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|paths| {
                paths.iter().all(|path| {
                    path.get_ident()
                        .is_some_and(|name| builtin_derive(&name.to_string()))
                })
            });
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_names_remain_external_and_do_not_erase_unrelated_local_bindings() {
        let mut scope = Scope::default();
        scope.import_external(vec!["format".into(), "vec".into()]);
        assert!(scope.lookup("format").is_err());
        assert!(scope.lookup("vec").is_err());
        assert!(scope.lookup("cfg_if").unwrap().is_none());
        let item: syn::Item = syn::parse_str("macro_rules! cfg_if { () => {} }").unwrap();
        scope
            .observe(
                &item,
                &Configuration::new(&[]),
                Some(binding("local:cfg_if")),
            )
            .unwrap();
        assert_eq!(
            scope.lookup("cfg_if").unwrap().unwrap().identity,
            "local:cfg_if"
        );
        assert!(scope.child().lookup("format").is_err());
    }

    fn binding(identity: &str) -> DefinitionBinding {
        DefinitionBinding {
            identity: identity.into(),
            site: FileSpan {
                path: "/sample/src/macros.rs".into(),
                digest: "sha256:source-checked-by-caller".into(),
                span: crate::ByteSpan { start: 0, end: 40 },
            },
        }
    }

    fn define(scope: &mut Scope, identity: &str) {
        let item = syn::parse_str("macro_rules! cfg_if { () => {}; }").unwrap();
        scope
            .observe(&item, &Configuration::new(&[]), Some(binding(identity)))
            .unwrap();
    }

    #[test]
    fn child_scope_is_a_temporal_snapshot_and_only_introduced_definitions_export() {
        let mut parent = Scope::default();
        define(&mut parent, "first");
        let mut child = parent.child();
        define(&mut parent, "later-parent");
        assert_eq!(child.lookup("cfg_if").unwrap().unwrap().identity, "first");
        let empty = parent.child();
        parent.import_child(&empty).unwrap();
        assert_eq!(
            parent.lookup("cfg_if").unwrap().unwrap().identity,
            "later-parent"
        );
        define(&mut child, "child-rebinding");
        parent.import_child(&child).unwrap();
        assert_eq!(
            parent.lookup("cfg_if").unwrap().unwrap().identity,
            "child-rebinding"
        );
    }

    #[test]
    fn nested_macro_use_exports_compose_through_mixed_ordinary_items() {
        let mut parent = Scope::default();
        let mut support = parent.child();
        let mut macros = support.child();
        define(&mut macros, "original-support-macros");
        support.import_child(&macros).unwrap();
        for source in [
            "#[derive(Clone, Copy)] struct Value(u8);",
            "pub use other::{Value, State};",
            "fn helper() {}",
            "const VALUE: u8 = 0;",
        ] {
            let item = syn::parse_str(source).unwrap();
            support
                .observe(&item, &Configuration::new(&[]), None)
                .unwrap();
        }
        parent.import_child(&support).unwrap();
        assert_eq!(
            parent.lookup("cfg_if").unwrap().unwrap().identity,
            "original-support-macros"
        );
        let mut ordinary = parent.child();
        define(&mut ordinary, "unexported-ordinary-child");
        assert_eq!(
            parent.lookup("cfg_if").unwrap().unwrap().identity,
            "original-support-macros"
        );
    }

    #[test]
    fn active_unknown_expansion_or_attribute_never_proves_an_empty_export() {
        for source in [
            "unknown!();",
            "#[unknown] struct Value;",
            "#[derive(custom::Clone)] struct Value;",
            "use other::*;",
            "#[macro_use] extern crate other;",
        ] {
            let mut scope = Scope::default();
            define(&mut scope, "known-before-unknown");
            scope
                .observe(
                    &syn::parse_str(source).unwrap(),
                    &Configuration::new(&[]),
                    None,
                )
                .unwrap();
            assert!(scope.lookup("cfg_if").is_err(), "{source}");
            assert!(!Scope::default().import_child(&scope).unwrap(), "{source}");
        }
        let mut scope = Scope::default();
        scope
            .observe(
                &syn::parse_str("#[cfg(missing)] unknown!();").unwrap(),
                &Configuration::new(&[]),
                None,
            )
            .unwrap();
        assert!(scope.lookup("cfg_if").unwrap().is_none());
    }

    #[test]
    fn protected_names_and_missing_original_definition_evidence_reject() {
        for source in [
            "macro_rules! Copy { () => {}; }",
            "macro_rules! panic { () => {}; }",
            "use other::Clone;",
            "use other::Name as f64;",
        ] {
            let item: syn::Item = syn::parse_str(source).unwrap();
            let record = matches!(&item, syn::Item::Macro(_)).then(|| binding("forged"));
            let mut scope = Scope::default();
            scope
                .observe(&item, &Configuration::new(&[]), record)
                .unwrap();
            assert!(scope.lookup("cfg_if").is_err(), "{source}");
        }
        let item = syn::parse_str("macro_rules! cfg_if { () => {}; }").unwrap();
        assert!(
            Scope::default()
                .observe(&item, &Configuration::new(&[]), None)
                .is_err()
        );
    }
}
