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

    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn tree(&self) -> &TypeTree {
        &self.tree
    }
    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn certificate(
        &self,
    ) -> &CheckedNoDrop {
        &self.certificate
    }
}

/// Shared lookup and namespace binding; each receipt independently proves its owner.
pub(in crate::nominal_boundary::resolution::selected::no_drop) trait SourceReceipt {
    fn verify_owner(&self, facts: &HirFnFacts, environment: &Environment) -> Result<(), String>;
    fn owner(&self) -> &super::Owner;
    fn root_tree(&self) -> &TypeTree;
    fn receipts(&self) -> &[Receipt];

    fn declaration(&self, tree: &TypeTree) -> Option<&CheckedNoDrop> {
        self.receipts()
            .iter()
            .find(|receipt| receipt.tree() == tree)
            .map(Receipt::certificate)
    }

    fn root(&self) -> &CheckedNoDrop {
        self.declaration(self.root_tree())
            .expect("Complete recursive source proof retains its root receipt")
    }

    fn function_namespace<'a>(
        &'a self,
        facts: &HirFnFacts,
        environment: &Environment,
        namespace: &[String],
    ) -> Result<Option<&'a CheckedNoDrop>, String> {
        self.verify_owner(facts, environment)?;
        Ok((self.owner().namespace() == namespace).then(|| self.root()))
    }
}
