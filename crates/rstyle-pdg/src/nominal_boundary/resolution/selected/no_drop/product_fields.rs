//! Shared exact declaration, positive Copy and exhaustive product-source checks.
use super::*;
use crate::nominal_boundary::resolution::Resolution;

pub(super) struct Proof<'a> {
    facts: &'a HirFnFacts,
    resolution: &'a Resolution,
    origins: &'a Origins,
    source: SourceClosure,
}

enum SourceClosure {
    ReferenceProduct,
    CopyProduct,
}

impl<'a> Proof<'a> {
    pub(super) fn reference(
        facts: &'a HirFnFacts,
        resolution: &'a Resolution,
        origins: &'a Origins,
    ) -> Self {
        Self {
            facts,
            resolution,
            origins,
            source: SourceClosure::ReferenceProduct,
        }
    }

    pub(super) fn copy_product(
        facts: &'a HirFnFacts,
        resolution: &'a Resolution,
        origins: &'a Origins,
    ) -> Self {
        Self {
            facts,
            resolution,
            origins,
            source: SourceClosure::CopyProduct,
        }
    }

    pub(super) fn declaration(&self, tree: &TypeTree) -> Result<Namespace, String> {
        let (nominal, _) = tree
            .nominal_parts()
            .ok_or("Missing product dependency nominal")?;
        let key: Namespace = nominal
            .definition_path
            .split("::")
            .map(str::to_owned)
            .collect();
        if self.resolution.trees.get(&key) != Some(tree) {
            return Err(
                "Cleanup product differs from the independently resolved complete tree".into(),
            );
        }

        let environment = &self.resolution.environment;
        let declaration = environment
            .index
            .declarations
            .get(&key)
            .ok_or("Missing cleanup product declaration")?;
        if !environment.declaration_matches(declaration, &nominal.declaration_digest)? {
            return Err("Cleanup product original declaration identity disagrees".into());
        }
        Ok(key)
    }

    pub(super) fn item(&self, key: &Namespace) -> Result<&syn::Item, String> {
        let environment = &self.resolution.environment;
        let declaration = environment
            .index
            .declarations
            .get(key)
            .ok_or("Missing cleanup declaration")?;
        Ok(&environment.modules[declaration.module_index].items[declaration.item_index])
    }

    pub(super) fn copy(&self, tree: &TypeTree) -> Result<(), String> {
        self.copy_with(tree, &mut None)
    }

    pub(super) fn copy_receipts(
        &self,
        tree: &TypeTree,
    ) -> Result<Vec<copy_products::Receipt>, String> {
        let mut receipts = Some(Vec::new());

        self.copy_with(tree, &mut receipts)?;
        receipts.ok_or_else(|| "Copy product receipt sink disappeared".into())
    }

    fn copy_with(
        &self,
        tree: &TypeTree,
        receipts: &mut Option<Vec<copy_products::Receipt>>,
    ) -> Result<(), String> {
        if matches!(tree, TypeTree::Scalar(_)) {
            return Ok(());
        }
        if let TypeTree::Option(payload) = tree {
            return super::super::super::builtin_options::scalar_copy(self.facts, payload);
        }
        let key = self.declaration(tree)?;
        let item = self.item(&key)?;
        let attributes = super::super::declaration_attributes(item, tree)?;
        if !super::super::checked_copy(self.facts, self.origins, tree, attributes)? {
            return Err(
                "Cleanup product dependency lacks exact positive builtin Copy proof".into(),
            );
        }

        match item {
            syn::Item::Struct(item) => super::super::structure(self.facts, item, true)?,
            syn::Item::Enum(item) => super::super::scalar_enum(self.facts, item)?,
            _ => return Err("Cleanup dependency is not a complete struct or enum".into()),
        }
        for field in tree.nominal_fields() {
            self.copy_with(&field.ty, receipts)?;
        }
        self.closure(&key)?;
        if let Some(receipts) = receipts {
            receipts.push(copy_products::Receipt::checked(tree)?);
        }
        Ok(())
    }

    pub(super) fn closure(&self, key: &Namespace) -> Result<(), String> {
        crate::data_library::verify_sysroot_roots(
            self.facts,
            &["alloc", "core", "std"],
            "Product cleanup source",
        )?;
        let registry = crate::data_library::registry_for(self.facts)
            .ok_or("Product cleanup requires pinned compiler/target")?;
        if !library_macros::pinned_native(registry) {
            return Err("Product cleanup macro source pins are incomplete".into());
        }

        let environment = &self.resolution.environment;
        // This evidence-only mode must not share ordinary admission caches.
        match self.source {
            SourceClosure::ReferenceProduct => {
                closure::verify_reference_product(environment, self.facts, key)
            }
            SourceClosure::CopyProduct => {
                closure::verify_copy_product(environment, self.facts, key)
            }
        }
    }
}

#[cfg(test)]
pub(super) mod test_support;
