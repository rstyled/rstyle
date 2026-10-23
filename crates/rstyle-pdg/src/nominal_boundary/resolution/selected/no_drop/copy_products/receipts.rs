//! A receipt exists only after its own declaration, fields, Copy and source proof.
use super::*;

pub(in crate::nominal_boundary::resolution::selected::no_drop) struct Receipt {
    tree: TypeTree,
    certificate: CheckedNoDrop,
}

impl Receipt {
    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn checked(
        tree: &TypeTree,
    ) -> Result<Self, String> {
        let (nominal, _) = tree
            .nominal_parts()
            .ok_or("Missing checked Copy declaration nominal")?;
        Ok(Self {
            tree: tree.clone(),
            certificate: CheckedNoDrop {
                nominal: nominal.clone(),
            },
        })
    }

    pub(super) fn tree(&self) -> &TypeTree {
        &self.tree
    }
    pub(super) fn certificate(&self) -> &CheckedNoDrop {
        &self.certificate
    }
}
