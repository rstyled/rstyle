//! Exact source-only import receipts, owned by one checked dependency analysis.
use super::*;
use crate::{dependency_namespaces::proof::AnonymousPrelude, unit_store::EvidenceFacts};

#[derive(Default)]
pub(super) struct Namespaces {
    // Unproved imports remain in the original index and retain every barrier.
    proved: Vec<AnonymousPrelude>,
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
