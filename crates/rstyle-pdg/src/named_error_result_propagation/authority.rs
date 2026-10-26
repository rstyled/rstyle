//! Selected core implementations, exact substitutions, and complete residual type.
use super::capture::{Shape, Witness};
use crate::result_propagation::authority::{Protocol, SOURCE_DIGESTS};
use crate::{HirNode, unit_store::EvidenceFacts};

pub(super) fn verify(
    witness: &Witness,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned named error reader Result propagation compiler/target")?;

    crate::data_library::verify_sysroot_roots(
        facts,
        &["core", "std"],
        "Named error reader Result propagation",
    )?;

    let protocol = Protocol {
        registry,
        prefix: "rstyle.result-propagation",
    };

    protocol.sources(SOURCE_DIGESTS)?;

    if operand.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(witness.shape.input.as_str())
        || facts.signature.return_ty.observed != witness.shape.output
    {
        return Err(
            "Named error reader propagation substitutions disagree with concrete observations"
                .into(),
        );
    }

    protocol.selection(&witness.branch, "try-branch", "::branch")?;

    protocol.selection(&witness.residual, "from-residual", "::from_residual")?;

    protocol.selection(&witness.conversion, "identity-from", "::from")?;

    protocol.variant(&witness.break_arm.variant, "Break")?;

    protocol.variant(&witness.continue_arm.variant, "Continue")?;

    concrete_shape(&witness.shape)?;

    signatures(witness)
}

pub(super) fn signatures(witness: &Witness) -> Result<(), String> {
    let shape = &witness.shape;

    let branch = &witness.branch;

    let residual = &witness.residual;

    let conversion = &witness.conversion;

    concrete_shape(shape)?;
    let output = &shape.output;

    if branch.arguments != [shape.input.as_str()]
        || branch.implementation_arguments != [shape.payload.as_str(), shape.error.as_str()]
        || branch.input != shape.input
        || branch.output != shape.control_flow
        || residual.arguments != [output.as_str(), shape.residual.as_str()]
        || residual.implementation_arguments
            != ["(u32, usize)", shape.error.as_str(), shape.error.as_str()]
        || residual.input != shape.residual
        || &residual.output != output
        || conversion.arguments != [shape.error.as_str(), shape.error.as_str()]
        || conversion.implementation_arguments != [shape.error.as_str()]
        || conversion.input != shape.error
        || conversion.output != shape.error
    {
        return Err(
            "Propagation substitutions/signatures do not prove exact same-error identity".into(),
        );
    }
    Ok(())
}

pub(super) fn concrete_shape(shape: &Shape) -> Result<(), String> {
    let result = ["core::result::Result", "std::result::Result"];

    let control = ["core::ops::ControlFlow", "std::ops::ControlFlow"];

    if shape.payload != "()"
        || shape.output_payload != "(u32, usize)"
        || !result
            .iter()
            .any(|path| shape.input == format!("{path}<{}, {}>", shape.payload, shape.error))
        || !result.iter().any(|path| {
            shape.output == format!("{path}<{}, {}>", shape.output_payload, shape.error)
        })
        || !result
            .iter()
            .any(|path| shape.residual == format!("{path}<!, {}>", shape.error))
        || !control.iter().any(|path| {
            shape.control_flow == format!("{path}<{}, ()>", shape.residual)
                || shape.control_flow == format!("{path}<{}>", shape.residual)
        })
    {
        return Err(
            "Named error reader propagation requires exact unit/tuple Result and residual ControlFlow"
                .into(),
        );
    }
    Ok(())
}
