//! Exact complete constructor products, authorized only by a private checked owner.
mod shape;
#[cfg(test)]
mod tests;

use super::CheckedProjection;
use crate::{HirFnFacts, HirNode, data_types::TypeToken, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.checked-reference-constructor-instances";
pub(crate) const UNIT: &str = "data.reference-instance-unit";
type Result<T> = std::result::Result<T, String>;

pub(crate) struct Products {
    result: TypeToken,
}

impl Products {
    /// Called only after the complete original body and no-drop closure proof.
    pub(super) fn checked(facts: &HirFnFacts) -> Result<Self> {
        let result = crate::data_types::parse(
            facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .ok_or("Checked constructor lacks its complete result token")?,
        )?;
        shape::verify(&result)?;

        let products = Self { result };
        products.sites(facts)?;
        Ok(products)
    }

    fn sites(&self, facts: &HirFnFacts) -> Result<()> {
        let input = facts
            .body
            .children
            .get(1)
            .ok_or("Checked constructor omits its result site")?;
        if input.children.len() != 4 || input.children[1].children.len() != 2 {
            return Err("Checked constructor loses its complete Input/Span operand roster".into());
        }
        for (node, ty) in [
            (input, self.input()),
            (&input.children[1], self.span()),
            (&input.children[2], self.anchored()),
        ] {
            if crate::data_types::parse(crate::data_aggregates::token(node)?)? != *ty {
                return Err(
                    "Checked product site differs from its complete result component".into(),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn permits(&self, ty: &TypeToken) -> bool {
        shape::contains(&self.result, ty)
    }

    fn input(&self) -> &TypeToken {
        &self.result
    }

    fn span(&self) -> &TypeToken {
        let TypeToken::Struct { fields, .. } = &self.result else {
            unreachable!("checked Input product")
        };
        &fields[1].1
    }

    fn anchored(&self) -> &TypeToken {
        let TypeToken::Struct { fields, .. } = &self.result else {
            unreachable!("checked Input product")
        };
        &fields[2].1
    }
}

fn checked(facts: &dyn EvidenceFacts) -> Result<&CheckedProjection> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Reference constructor products require their registered capability".into());
    }
    facts
        .reference_instance()
        .ok_or("Reference product requires its private checked instance owner".into())
}

fn input(instance: &CheckedProjection) -> &HirNode {
    &instance.facts().body.children[1]
}

pub(crate) fn struct_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    matches!(
        site(node, facts),
        Some(ProductSite::Input | ProductSite::Span)
    )
}

pub(crate) fn slots(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Vec<Option<usize>>> {
    let instance = checked(facts)?;
    let input = input(instance);
    let original = if node.hir == input.hir {
        input
    } else if node.hir == input.children[1].hir {
        &input.children[1]
    } else {
        return Err("Reference struct is outside its exact proved constructor sites".into());
    };

    exact(node, original)?;
    Ok((0..original.children.len()).map(Some).collect())
}

pub(crate) struct UnitVariant<'a> {
    pub name: &'static str,
    pub ty: &'a TypeToken,
}

pub(crate) fn unit_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    site(node, facts) == Some(ProductSite::Unit)
}

pub(crate) fn unit<'a>(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Result<UnitVariant<'a>> {
    let instance = checked(facts)?;
    let original = &input(instance).children[2];
    let ty = instance.products().anchored();

    exact(node, original)?;

    Ok(UnitVariant { name: "No", ty })
}

fn exact(node: &HirNode, original: &HirNode) -> Result<()> {
    if node != original {
        return Err(
            "Reference product site changed its complete checked source/HIR/type evidence".into(),
        );
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
enum ProductSite {
    Input,
    Span,
    Unit,
}

fn site(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<ProductSite> {
    let instance = checked(facts).ok()?;
    let input = input(instance);
    if node.hir == input.hir {
        Some(ProductSite::Input)
    } else if node.hir == input.children[1].hir {
        Some(ProductSite::Span)
    } else if node.hir == input.children[2].hir {
        Some(ProductSite::Unit)
    } else {
        None
    }
}
