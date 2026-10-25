//! Output-only ownership for the exact original nonzero composition.
//! The parser input instance stays in its independently verified callee receipt.
use super::scalar_product_instances as product;
use super::*;
use crate::{nonzero_digit_calls, scalar_product_instances::Observation as Instance};

pub(crate) type CheckedNonzeroProduct = product::CheckedScalarProductInstance;

pub(in crate::nominal_boundary::resolution) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    instance: &Instance,
) -> Result<CheckedNonzeroProduct, String> {
    instance.check_shape()?;
    nonzero_digit_calls::source::shape(facts, instance.argument)?;

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    product::verify_interface(&interface, instance)?;

    if !matches!(interface.parameters.as_slice(), [Observation::Captured { ty }] if concrete_types::byte_views::observed(ty))
    {
        return Err(
            "Nonzero composition source requires one complete shared byte-view formal".into(),
        );
    }

    let TypeTree::NonZero(payload) = &instance.fields[1].ty else {
        return Err("Nonzero product cleanup requires its exact payload".into());
    };
    crate::nonzero_values::verify(payload, facts)?;

    let environment = &resolution.environment;
    environment.check_input_owner(facts)?;
    crate::declaration_origins::verify_in(&interface, origins, environment)?;
    product::verify_declaration(facts, resolution, origins, instance)?;

    let owner = copy_products::Owner::capture(facts, environment, origins)?;
    product::source::unshadowed_byte(facts, environment, owner.namespace())?;
    resolution.verify_nonzero_binding(facts, owner.namespace(), payload.scalar.spelling())?;

    Ok(CheckedNonzeroProduct::output_source(
        instance,
        owner,
        environment,
    ))
}

pub(in crate::nominal_boundary::resolution) fn resolve(
    receipt: &CheckedNonzeroProduct,
    path: &syn::TypePath,
    namespace: &[String],
) -> Result<Option<TypeTree>, String> {
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || path.path.segments[0].ident != "NonZero"
    {
        return Ok(None);
    }

    receipt.verify_namespace(namespace)?;

    let syn::PathArguments::AngleBracketed(arguments) = &path.path.segments[0].arguments else {
        return Err("Nonzero source requires its concrete scalar argument".into());
    };

    let TypeTree::NonZero(payload) = &receipt.instance().fields[1].ty else {
        return Err("Nonzero source lost its checked payload".into());
    };

    if arguments.colon2_token.is_some()
        || arguments.args.len() != 1
        || !matches!(arguments.args.first(), Some(syn::GenericArgument::Type(syn::Type::Path(value))) if value.qself.is_none() && value.path.is_ident(payload.scalar.spelling()))
    {
        return Err("Nonzero source changes its exact primitive substitution".into());
    }
    Ok(Some(receipt.instance().fields[1].ty.clone()))
}
