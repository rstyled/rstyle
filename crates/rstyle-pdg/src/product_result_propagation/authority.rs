//! Exact selected core implementations and same-error identity conversion.
use super::*;
use crate::result_propagation::authority::{Protocol, SOURCE_DIGESTS};

pub(super) fn verify(
    witness: &Witness,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned product Result propagation compiler/target")?;

    crate::data_library::verify_sysroot_roots(
        facts,
        &["core", "std"],
        "Product Result propagation",
    )?;

    let protocol = Protocol {
        registry,
        prefix: "rstyle.result-propagation",
    };
    protocol.sources(SOURCE_DIGESTS)?;

    if operand.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(witness.shape.input.as_str())
        || facts.signature.return_ty.observed != witness.shape.output
    {
        return Err("Product propagation substitutions disagree with concrete observations".into());
    }

    protocol.selection(&witness.branch, "try-branch", "::branch")?;
    protocol.selection(&witness.residual, "from-residual", "::from_residual")?;
    protocol.selection(&witness.conversion, "identity-from", "::from")?;

    protocol.variant(&witness.break_arm.variant, "Break")?;
    protocol.variant(&witness.continue_arm.variant, "Continue")?;

    signatures(witness)
}

pub(super) fn signatures(witness: &Witness) -> Result<(), String> {
    let shape = &witness.shape;
    let branch = &witness.branch;
    let residual = &witness.residual;
    let conversion = &witness.conversion;

    let output = shape::generated_output(shape)?;

    if branch.arguments != [shape.input.as_str()]
        || branch.implementation_arguments != [shape.payload.as_str(), shape.error.as_str()]
        || branch.input != shape.input
        || branch.output != shape.control_flow
        || residual.arguments != [output.as_str(), shape.residual.as_str()]
        || residual.implementation_arguments
            != [
                shape.payload.as_str(),
                shape.error.as_str(),
                shape.error.as_str(),
            ]
        || residual.input != shape.residual
        || residual.output != output
        || conversion.arguments != [shape.error.as_str(), shape.error.as_str()]
        || conversion.implementation_arguments != [shape.error.as_str()]
        || conversion.input != shape.error
        || conversion.output != shape.error
    {
        return Err(
            "Product propagation substitutions/signatures do not prove exact same-error identity"
                .into(),
        );
    }

    Ok(())
}
