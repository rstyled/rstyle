//! Exact source-only import receipts, owned by one checked dependency analysis.
use super::*;
use crate::{
    dependency_namespaces::proof::{AnonymousPrelude, GeneratedExports},
    unit_store::EvidenceFacts,
};

#[derive(Default)]
pub(super) struct Namespaces {
    // Unproved imports remain in the original index and retain every barrier.
    proved: Vec<AnonymousPrelude>,
    generated: Vec<GeneratedExports>,
}

impl Environment {
    pub(crate) fn with_dependency_namespaces(
        mut self,
        input: &dyn EvidenceFacts,
    ) -> Result<Self, String> {
        if !crate::extraction_profile::facts_support(
            input.facts(),
            crate::dependency_namespaces::CAPABILITY,
        ) {
            return Ok(self);
        }
        let Some(capture) = input.dependency_namespace_capture()? else {
            return Ok(self);
        };
        for import in &capture.imports {
            let Some(owner) = input.dependency_namespace(import)? else {
                continue;
            };
            if crate::extraction_profile::facts_support(
                input.facts(),
                crate::dependency_namespaces::GENERATED_CAPABILITY,
            ) && let Ok(receipt) = owner.generated_exports()
                && crate::dependency_namespaces::proof::verify_site(
                    &self.census,
                    &self.modules,
                    receipt.import(),
                )
                .is_ok()
                && self.generated_target(&receipt)
            {
                self.dependency_namespaces.generated.push(receipt);
            }
            // A failed proof grants nothing; the unchanged original import is
            // still checked through the ordinary unresolved-import paths.
            let Ok(receipt) = owner.anonymous_prelude() else {
                continue;
            };
            if crate::dependency_namespaces::proof::verify_consumer(
                &self.census,
                &self.modules,
                &receipt,
            )
            .is_ok()
            {
                self.dependency_namespaces.proved.push(receipt);
            }
        }
        Ok(self)
    }

    fn generated_target(&self, receipt: &GeneratedExports) -> bool {
        let site = receipt.import();
        let expected = site
            .target
            .path
            .split("::")
            .map(str::to_owned)
            .collect::<Vec<_>>();
        self.foreign_namespace(&site.scope, &site.path, &site.target.crate_name)
            .is_ok_and(|path| path == expected)
    }

    fn foreign_namespace(
        &self,
        scope: &[String],
        path: &[String],
        external: &str,
    ) -> Result<Vec<String>, String> {
        let mut names = if path.first().is_some_and(|first| first == external) {
            path.to_vec()
        } else {
            anchored(path.to_vec(), &scope.to_vec(), false)?
        };
        for _ in 0..concrete_types::MAX_DEPTH {
            if names.first().is_some_and(|first| first == external) {
                return if self.dependency_root_shadowed(scope, external) {
                    Err("Dependency namespace root is shadowed".into())
                } else {
                    Ok(names)
                };
            }
            let (name, namespace) = names
                .split_last()
                .ok_or("Empty dependency namespace path")?;
            names = self
                .index
                .imports
                .get(&(namespace.to_vec(), name.clone()))
                .cloned()
                .ok_or("Dependency namespace lacks its original lexical reexport")?;
        }
        Err("Dependency namespace reexport cycle or depth exceeds source budget".into())
    }

    fn dependency_root_shadowed(&self, scope: &[String], external: &str) -> bool {
        scope.iter().enumerate().any(|(depth, _)| {
            let mut candidate = scope[..=depth].to_vec();
            candidate.push(external.into());
            self.index.declarations.contains_key(&candidate)
                || self
                    .modules
                    .iter()
                    .any(|module| module.namespace == candidate)
                || self
                    .index
                    .imports
                    .contains_key(&(scope[..=depth].to_vec(), external.into()))
        }) || self.index.declarations.contains_key(&vec![external.into()])
            || self
                .modules
                .iter()
                .any(|module| module.namespace == [external])
            || self
                .index
                .imports
                .contains_key(&(Vec::new(), external.into()))
    }

    pub(in crate::nominal_boundary::resolution) fn generated_use(
        &self,
        namespace: &[String],
        item: &syn::ItemUse,
    ) -> Option<&GeneratedExports> {
        if !item.attrs.is_empty() || item.leading_colon.is_some() || !direct_glob(&item.tree) {
            return None;
        }
        self.dependency_namespaces.generated.iter().find(|receipt| {
            let site = receipt.import();
            site.scope == namespace
                && self.census.files.iter().any(|file| {
                    file.path == site.declaration.path
                        && crate::source_evidence::original_span(&file.bytes, item.span())
                            == site.declaration.span
                })
        })
    }

    pub(in crate::nominal_boundary::resolution) fn anonymous_glob(
        &self,
        namespace: &[String],
        path: &[String],
    ) -> bool {
        if self.dependency_namespaces.proved.is_empty() {
            return false;
        }
        let mut found = false;
        for module in self
            .modules
            .iter()
            .filter(|module| module.namespace == namespace)
        {
            for item in &module.items {
                if let syn::Item::Use(item) = item
                    && contains_glob(&item.tree, &[], path)
                {
                    if !self.anonymous_use(namespace, item) {
                        return false;
                    }
                    found = true;
                }
            }
        }
        found
    }

    pub(in crate::nominal_boundary::resolution) fn anonymous_use(
        &self,
        namespace: &[String],
        item: &syn::ItemUse,
    ) -> bool {
        if self.dependency_namespaces.proved.is_empty()
            || !item.attrs.is_empty()
            || item.leading_colon.is_some()
            || !direct_glob(&item.tree)
        {
            return false;
        }
        self.dependency_namespaces.proved.iter().any(|receipt| {
            let site = receipt.import();
            site.scope == namespace
                && self.census.files.iter().any(|file| {
                    file.path == site.declaration.path
                        && crate::source_evidence::original_span(&file.bytes, item.span())
                            == site.declaration.span
                })
        })
    }
}

fn direct_glob(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => direct_glob(&path.tree),
        syn::UseTree::Glob(_) => true,
        _ => false,
    }
}

fn contains_glob(tree: &syn::UseTree, prefix: &[String], expected: &[String]) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            let child: Vec<_> = prefix
                .iter()
                .cloned()
                .chain([path.ident.to_string()])
                .collect();
            contains_glob(&path.tree, &child, expected)
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|tree| contains_glob(tree, prefix, expected)),
        syn::UseTree::Glob(_) => prefix == expected,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(source: &str) -> Environment {
        let mut census = (*super::super::tests::census()).clone();
        census.files[0].bytes = source.into();
        census.files[0].digest = census.files[0].bytes.checksum().into();
        Environment::with_options(Arc::new(census), false, Some(&[]), false).unwrap()
    }

    #[test]
    fn foreign_module_routes_require_the_original_unshadowed_reexport() {
        let scope = vec!["date".into()];
        let path = vec!["crate".into(), "convert".into()];
        let good = environment("pub use time_core::convert; mod date {}");
        assert_eq!(
            good.foreign_namespace(&scope, &path, "time_core").unwrap(),
            ["time_core", "convert"]
        );
        for source in [
            "pub use wrong::convert; mod date {}",
            "mod convert {} mod date {}",
            "mod time_core {} pub use time_core::convert; mod date {}",
            "pub use time_core::convert; mod date { mod time_core {} }",
            "use crate::B as convert; use crate::convert as B; mod date {}",
        ] {
            assert!(
                environment(source)
                    .foreign_namespace(&scope, &path, "time_core")
                    .is_err(),
                "{source}"
            );
        }
    }
}
