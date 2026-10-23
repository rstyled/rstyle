//! Interface authority only. Never calls constructors or the full-body walk.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation};
use crate::nominal_boundary::resolution::CheckedFreshProduct;

pub(crate) struct ProductBoundary {
    pub(crate) result: TypeToken,
    pub(crate) ok: TypeToken,
    pub(crate) error: TypeToken,
    _product: CheckedFreshProduct,
}

pub(crate) fn product_boundary(facts: &dyn EvidenceFacts) -> Result<ProductBoundary, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || !crate::extraction_profile::facts_support(facts, super::super::CAPABILITY)
    {
        return Err("Fresh product Result lacks its registered boundary capabilities".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Err(
            "Fresh product Result requires its complete captured return observation".into(),
        );
    };

    let success = TypeToken::from_tree(ok);
    let error = TypeToken::from_tree(err);

    if !GRAMMAR.permits(&success, &error) {
        return Err(
            "Fresh product Result requires its bounded success and actual error shape".into(),
        );
    }
    copy_errors::checked_error(&error, facts)?;
    constructors::verify_core(facts)?;
    let product = facts.nominal_resolution()?.fresh_reference_product(ok)?;

    if !product.permits(&success) {
        return Err("Fresh success differs from its exact checked product".into());
    }
    let result = TypeToken::Result {
        ok: Box::new(success.clone()),
        err: Box::new(error.clone()),
    };

    if facts.signature.return_ty.normalized.as_deref() != Some(result.text()?.as_str()) {
        return Err("Fresh product Result interface disagrees with its captured type".into());
    }
    Ok(ProductBoundary {
        result,
        ok: success,
        error,
        _product: product,
    })
}
