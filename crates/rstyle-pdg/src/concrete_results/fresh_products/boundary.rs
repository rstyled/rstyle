//! Interface authority only. Never calls constructors or the full-body walk.
use super::super::interface::{self, Contract, ObservedResult};
use super::*;
use crate::nominal_boundary::resolution::CheckedFreshProduct;

pub(crate) type ProductBoundary = interface::CheckedResult<CheckedFreshProduct>;

pub(crate) fn product_boundary(facts: &dyn EvidenceFacts) -> Result<ProductBoundary, String> {
    interface::verify(
        facts,
        Contract {
            capability: CAPABILITY,
            grammar: &GRAMMAR,
            check_error: copy_errors::checked_error,
            check_success: reference_success,
        },
    )
}

fn reference_success(
    observed: &ObservedResult<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<Option<CheckedFreshProduct>, String> {
    let product = facts
        .nominal_resolution()?
        .fresh_reference_product(observed.success())?;
    Ok(product.permits(&observed.ok).then_some(product))
}
