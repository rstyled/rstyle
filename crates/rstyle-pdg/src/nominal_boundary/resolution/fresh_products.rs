//! Whole source-checked fresh products; this witness grants no Copy or storage.
use super::*;

pub(super) const CAPABILITY: &str = "rust.fresh-reference-product-results";

/// Constructed only from this Resolution's complete selected cleanup evidence.
pub(crate) struct CheckedFreshProduct {
    tree: TypeTree,
}

impl CheckedFreshProduct {
    pub(crate) fn permits(&self, token: &crate::data_types::TypeToken) -> bool {
        &crate::data_types::TypeToken::from_tree(&self.tree) == token
    }
}

impl Resolution {
    pub(crate) fn fresh_reference_product(
        &self,
        tree: &TypeTree,
    ) -> Result<CheckedFreshProduct, String> {
        let (nominal, _) = tree
            .nominal_parts()
            .ok_or("Fresh product requires its complete nominal declaration")?;
        if !eligible(tree) || self.checked_nominal(nominal, &self.no_drop_trees) != Some(tree) {
            return Err("Fresh product lacks its exact selected source/no-drop witness".into());
        }
        Ok(CheckedFreshProduct { tree: tree.clone() })
    }
}

pub(super) fn eligible(tree: &TypeTree) -> bool {
    let Some(fields) = selected::no_drop::reference_products::fields(tree) else {
        return false;
    };
    fields.iter().all(|field| {
        crate::data_types::shared_byte_slice(&field.ty)
            || scalar_field(&field.ty)
            || matches!(&field.ty, TypeTree::Option(payload) if scalar_field(payload))
    })
}

fn scalar_field(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Scalar(token) if concrete_types::scalar(token) && token != "unit")
}

#[cfg(test)]
mod tests;
